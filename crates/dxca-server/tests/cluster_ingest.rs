//! M3 end-to-end + the milestone's exit criterion: node-status behaviour
//! against real (fake) cluster nodes, including a deliberately flaky one
//! (docs/PLAN.md §10 M3).
//!
//! - Happy path: login prompt → call → welcome → spot line. The node goes
//!   proven-Live, the spot lands in the ring as a synthetic decode, and
//!   the telnet feed carries a `DX de <node>:` line. A spot whose call
//!   cell is not a call (`EU`) is kept in the ring and relayed nowhere.
//! - Flaky node: accepts TCP, streams banner noise, never acks. The
//!   session must stay UNPROVEN (yellow, 1.x honest-status), the auth
//!   watchdog must recycle it, and the backoff attempt count must
//!   escalate instead of resetting on bare TCP.

use dxca_connect::dxcluster::ClientConfig;
use dxca_server::config::Config;
use dxca_server::nodes::NodeManager;
use dxca_server::pipeline;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn wait_until<F: Fn() -> bool>(what: &str, deadline_s: u64, f: F) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(deadline_s);
    while !f() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn test_client_cfg(port: u16) -> ClientConfig {
    let mut cfg = ClientConfig::new("127.0.0.1", port, "VU2CPL");
    cfg.reconnect_schedule_s = vec![1, 1];
    cfg.auth_timeout_s = 1;
    cfg.silence_timeout_s = 600;
    cfg.init_commands = Vec::new(); // keep the fake node's read side simple
    cfg
}

#[tokio::test]
async fn cluster_spot_flows_to_ring_and_telnet() {
    // Fake DXSpider node.
    let node = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let node_port = node.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut s, _) = node.accept().await.unwrap();
        s.write_all(b"login: ").await.unwrap();
        let mut buf = [0u8; 256];
        let n = s.read(&mut buf).await.unwrap(); // the callsign line
        assert!(String::from_utf8_lossy(&buf[..n]).contains("VU2CPL"));
        s.write_all(b"Hello VU2CPL, welcome to the test node\r\n")
            .await
            .unwrap();
        // VU2OY's RBN Aggregator, 2026-10-28: `EU` in the call cell. It has
        // no callsign, so it must not be relayed as `UNKNOWN`.
        s.write_all(b"DX de VU2OY-#:   18100.0  EU   -12 dB   6 FT8  1185  0633Z\r\n")
            .await
            .unwrap();
        s.write_all(
            b"DX de W3LPL:     14074.0  K1JT           FT8 -10 dB                  1428Z\r\n",
        )
        .await
        .unwrap();
        // Hold the connection open.
        let _ = s.read(&mut buf).await;
    });

    let cfg = Config {
        telnet_port: 0,
        udp_sources: Vec::new(),
        ..Config::default()
    };
    let (state, input_tx) = pipeline::start(&cfg, None).await.expect("pipeline");
    let mut telnet = TcpStream::connect(("127.0.0.1", state.telnet.local_port()))
        .await
        .unwrap();

    let manager = NodeManager::new();
    manager.start_node("VE7CC".into(), test_client_cfg(node_port), input_tx);

    // Status: proven Live with both spots counted.
    let mgr = &manager;
    wait_until("node proven live with its spots", 10, || {
        mgr.statuses()
            .get("VE7CC")
            .is_some_and(|s| s.proven && s.spot_count == 2)
    })
    .await;
    let st = manager.statuses()["VE7CC"].clone();
    assert_eq!(st.state, "Live");
    assert!(st.connected);
    assert_eq!(st.attempt, 0);

    // The synthetic decodes are in the ring, 1.x handleClusterSpot-style.
    // The `EU` one is kept for the Spots table, naming nobody.
    wait_until("spots in ring", 5, || state.recent_spots(2).len() == 2).await;
    let eu = &state.recent_spots(2)[1];
    assert_eq!(eu.message, "CQ EU");
    assert_eq!(eu.dx_callsign(), None);
    let spot = &state.recent_spots(1)[0];
    assert_eq!(spot.message, "CQ K1JT");
    assert_eq!(spot.source_name, "VE7CC");
    assert_eq!(spot.mode, "FT8");
    assert_eq!(spot.dial_frequency_hz, 14_074_000);

    // And the telnet feed carries the re-formatted line, node as spotter,
    // with nothing ahead of it for the `EU` spot.
    let mut got = String::new();
    let mut buf = [0u8; 1024];
    while !got.contains("K1JT") {
        let n = tokio::time::timeout(std::time::Duration::from_secs(5), telnet.read(&mut buf))
            .await
            .expect("telnet line timed out")
            .unwrap();
        assert!(n > 0);
        got.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    assert!(got.contains("DX de VE7CC:"), "got: {got:?}");
    assert!(!got.contains("UNKNOWN"), "got: {got:?}");
}

#[tokio::test]
async fn flaky_node_stays_unproven_and_recycles_with_escalating_backoff() {
    // A node that accepts and talks, but never acks a login and never
    // sends data — the VE7CC-2026-08-24 failure mode from the 1.x notes.
    let node = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let node_port = node.local_addr().unwrap().port();
    let accepts = Arc::new(AtomicUsize::new(0));
    let accepts_srv = accepts.clone();
    tokio::spawn(async move {
        loop {
            let (mut s, _) = node.accept().await.unwrap();
            accepts_srv.fetch_add(1, Ordering::SeqCst);
            // Banner noise only — mentions no ack words, shows no prompt.
            let _ = s.write_all(b"AR-Test node v0.0\r\n").await;
            let mut buf = [0u8; 256];
            // Swallow whatever the client sends; never reply again.
            while matches!(s.read(&mut buf).await, Ok(n) if n > 0) {}
        }
    });

    let cfg = Config {
        telnet_port: 0,
        udp_sources: Vec::new(),
        ..Config::default()
    };
    let (_state, input_tx) = pipeline::start(&cfg, None).await.expect("pipeline");
    let manager = NodeManager::new();
    manager.start_node("FLAKY".into(), test_client_cfg(node_port), input_tx);

    // The watchdog (auth_timeout 1 s) must recycle the unproven session and
    // the supervisor must redial: at least two accepts.
    let a = accepts.clone();
    wait_until("second connect after watchdog recycle", 15, || {
        a.load(Ordering::SeqCst) >= 2
    })
    .await;

    let st = manager.statuses()["FLAKY"].clone();
    // Honest status: never proven, never "Live"; attempts escalated.
    assert!(!st.proven, "flaky node must never show proven");
    assert_ne!(st.state, "Live");
    assert!(
        st.attempt >= 1,
        "backoff attempt must escalate, got {}",
        st.attempt
    );
    assert_eq!(st.spot_count, 0);

    // And it keeps cycling (attempt keeps climbing, still unproven).
    let a = accepts.clone();
    wait_until("third connect", 15, || a.load(Ordering::SeqCst) >= 3).await;
    assert!(!manager.statuses()["FLAKY"].proven);
}

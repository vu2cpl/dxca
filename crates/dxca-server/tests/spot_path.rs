//! M2 end-to-end: feed real captured decoder datagrams (dxca-core's test
//! vectors) into a running pipeline over actual UDP sockets, then assert
//! the two output paths:
//!  - the passthrough destination receives every datagram byte-identical
//!    (the 1094/1094 invariant from the live capture, in miniature);
//!  - a telnet client gets the banner and a DX-Spider line carrying the
//!    callsign extracted from the decode.

use dxca_server::config::{BroadcastDestination, Config, UdpSource};
use dxca_server::pipeline;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};

fn vector(decoder: &str, name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dxca-core/tests/vectors")
        .join(decoder)
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("vector {}: {e}", p.display()))
}

#[tokio::test]
async fn vectors_flow_to_passthrough_and_telnet() {
    // The passthrough "logger" (stand-in RUMlog) binds first so we know
    // its port for the destination config.
    let logger = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    let logger_port = logger.local_addr().unwrap().port();

    let source_port = 48_334; // test-only fixed port for the JTDX source
    let cfg = Config {
        telnet_port: 0, // ephemeral — read back from the state
        udp_sources: vec![UdpSource {
            name: "JTDX".into(),
            port: source_port,
            enabled: true,
        }],
        broadcast_destinations: vec![BroadcastDestination {
            name: "logger".into(),
            ip: Ipv4Addr::LOCALHOST,
            port: logger_port,
            format: "passthrough".into(),
            sources: Vec::new(),
            unfiltered: false,
            enabled: true,
        }],
        ..Config::default()
    };

    let (state, _input_tx) = pipeline::start(&cfg, None).await.expect("pipeline start");

    // Telnet client connects and sees the banner.
    let mut telnet = TcpStream::connect(("127.0.0.1", state.telnet.local_port()))
        .await
        .unwrap();
    let banner = read_some(&mut telnet, "DXCA").await;
    assert!(banner.contains("DX Cluster Server"));

    // Feed a real Status (sets the dial) then a real Decode.
    let status = vector("jtdx", "type01-1.bin");
    let decode = vector("jtdx", "type02-1.bin");
    let sender = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    sender
        .send_to(&status, ("127.0.0.1", source_port))
        .await
        .unwrap();
    sender
        .send_to(&decode, ("127.0.0.1", source_port))
        .await
        .unwrap();

    // Passthrough must deliver both datagrams byte-identical, in order.
    for expected in [&status, &decode] {
        let mut buf = vec![0u8; 65_536];
        let (n, _) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            logger.recv_from(&mut buf),
        )
        .await
        .expect("passthrough datagram timed out")
        .unwrap();
        assert_eq!(
            &buf[..n],
            expected.as_slice(),
            "passthrough not byte-identical"
        );
    }

    // The telnet client gets a DX line for the decode, with the callsign
    // the extractor pulled from the real message text.
    let line = read_some(&mut telnet, "Z").await;
    assert!(line.starts_with("DX de JTDX:"), "line: {line:?}");
    let spot = &state.recent_spots(1)[0];
    let call = spot.dx_callsign().expect("captured decode has a callsign");
    assert!(line.contains(&call), "line {line:?} missing call {call}");
    assert!(line.trim_end().ends_with('Z'));

    // And the ring recorded the spot with the Status-supplied dial.
    assert!(
        spot.dial_frequency_hz > 0,
        "dial should come from the Status"
    );
    assert_eq!(spot.source_name, "JTDX");
}

async fn read_some(stream: &mut TcpStream, needle: &str) -> String {
    let mut got = String::new();
    let mut buf = [0u8; 1024];
    while !got.contains(needle) {
        let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut buf))
            .await
            .expect("telnet read timed out")
            .expect("telnet read");
        assert!(n > 0, "server closed; got {got:?}");
        got.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    got
}

/// Reported from a Windows install running WSJT-X: local spots showed no
/// usable mode. The captured vectors say why — a real WSJT-X decode carries
/// its mode as the single character it prints (`~` for FT8), not as a name,
/// and DXCA passed that straight through to the Spots table.
///
/// Uses the genuine capture rather than a synthesised datagram, because the
/// whole point is what WSJT-X actually sends.
#[tokio::test]
async fn a_real_wsjtx_decode_reports_ft8_not_a_tilde() {
    let source_port = 48_335;
    let cfg = Config {
        telnet_port: 0,
        udp_sources: vec![UdpSource {
            name: "WSJTX".into(),
            port: source_port,
            enabled: true,
        }],
        broadcast_destinations: Vec::new(),
        ..Config::default()
    };
    let (state, _tx) = pipeline::start(&cfg, None).await.expect("pipeline start");

    let send = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    // Status first: it carries the dial frequency, and a proper mode name
    // that backs up the marker map.
    send.send_to(&vector("wsjtx", "type01-1.bin"), ("127.0.0.1", source_port))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    send.send_to(&vector("wsjtx", "type02-1.bin"), ("127.0.0.1", source_port))
        .await
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let spot = loop {
        assert!(std::time::Instant::now() < deadline, "no spot arrived");
        if let Some(s) = state.recent_spots(1).first().cloned() {
            break s;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };

    assert_eq!(spot.source_name, "WSJTX");
    assert_eq!(
        spot.mode, "FT8",
        "the `~` marker must resolve to a mode name"
    );
    assert!(
        !spot.mode_inferred,
        "WSJT-X told us the mode; nothing was guessed from the frequency"
    );
}

/// `CP6CL <...> RR73` was 7P8GE (hash unresolved) signing off with CP6CL,
/// yet DXCA spotted CP6CL — at 7P8GE's offset — and alerted on him. The
/// fix extracts no callsign from it; this pins the other half: with no
/// callsign the 1.x rule would rebroadcast it as `UNKNOWN`, so it must
/// reach neither the telnet feed nor an UNFILTERED cluster destination
/// (v2.23.2 still sent those `UNKNOWN`), while the next real exchange
/// reaches both. v2.23.3 stopped only that shape; `CQ EU` and free text
/// have no callsign either, and still went out as `UNKNOWN` until v2.23.4.
#[tokio::test]
async fn a_spot_with_no_callsign_never_reaches_the_cluster_feed() {
    use dxca_core::wsjtx::{Decode, Status, encode_decode, encode_status};

    // An unfiltered cluster-format destination takes every spot, whatever
    // the filters and dedupe say — the path a `passes = false` missed.
    let feed = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    let feed_port = feed.local_addr().unwrap().port();

    let source_port = 48_336;
    let cfg = Config {
        telnet_port: 0,
        udp_sources: vec![UdpSource {
            name: "JTDX".into(),
            port: source_port,
            enabled: true,
        }],
        broadcast_destinations: vec![BroadcastDestination {
            name: "aggregator".into(),
            ip: Ipv4Addr::LOCALHOST,
            port: feed_port,
            format: "cluster".into(),
            sources: Vec::new(),
            unfiltered: true,
            enabled: true,
        }],
        ..Config::default()
    };
    let (state, _tx) = pipeline::start(&cfg, None).await.expect("pipeline start");
    let mut telnet = TcpStream::connect(("127.0.0.1", state.telnet.local_port()))
        .await
        .unwrap();
    read_some(&mut telnet, "DXCA").await;

    let decode = |message: &str| Decode {
        client_id: "JTDX".into(),
        is_new: true,
        time_ms: 4_650_000,
        snr_db: -11,
        delta_time_s: 0.1,
        delta_frequency_hz: 408,
        mode: "~".into(),
        message: message.into(),
        low_confidence: false,
        off_air: false,
    };
    let status = Status {
        client_id: "JTDX".into(),
        dial_frequency_hz: 7_074_000,
        mode: "FT8".into(),
        ..Status::default()
    };
    let send = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    for bytes in [
        encode_status(2, &status),
        encode_decode(2, &decode("CP6CL <...> RR73")),
        encode_decode(2, &decode("CQ EU")),
        encode_decode(2, &decode("TNX 73 GL")),
        encode_decode(2, &decode("CP6CL 7P8GE -08")),
    ] {
        send.send_to(&bytes, ("127.0.0.1", source_port))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // The first spot line on the feed is the real one: the three decodes
    // before it produced nothing — neither CP6CL nor UNKNOWN.
    let line = read_some(&mut telnet, "Z").await;
    assert!(line.contains("7P8GE"), "line: {line:?}");
    assert!(!line.contains("CP6CL"), "line: {line:?}");
    assert!(!line.contains("UNKNOWN"), "line: {line:?}");

    // Same for the unfiltered destination: its first datagram is the real
    // spot, so the three sent nothing ahead of it.
    let mut buf = vec![0u8; 2048];
    let (n, _) = tokio::time::timeout(std::time::Duration::from_secs(5), feed.recv_from(&mut buf))
        .await
        .expect("the unfiltered destination got nothing")
        .unwrap();
    let datagram = String::from_utf8_lossy(&buf[..n]);
    assert!(datagram.contains("7P8GE"), "datagram: {datagram:?}");
    assert!(!datagram.contains("UNKNOWN"), "datagram: {datagram:?}");

    // All four decodes are still in the ring — the Spots table shows what
    // was decoded — but the first three name nobody.
    let ring = state.recent_spots(4);
    for message in ["CP6CL <...> RR73", "CQ EU", "TNX 73 GL"] {
        let kept = ring
            .iter()
            .find(|s| s.message == message)
            .unwrap_or_else(|| panic!("{message:?} is kept in the ring"));
        assert_eq!(kept.dx_callsign(), None, "message: {message:?}");
    }
}

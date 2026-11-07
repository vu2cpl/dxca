//! Retained MQTT status: what a dashboard needs to show DXCA's state without
//! logging in to it.
//!
//! The motivating consumer is Home Assistant. Its DXCC card used to read a
//! Node-RED flow that DXCA replaced, and the obvious substitute — HA polling
//! `/api/me/station` — needs a session, which means a DXCA password stored in
//! HA and a login that expires. Publishing the same numbers to the broker the
//! dashboard already reads needs neither, and costs one small message per
//! account per minute.
//!
//! Two kinds of topic, under each opted-in destination's base (see
//! `dxca_connect::mqtt`):
//!
//! * `<base>/status` — cluster-node health and per-source spot counts: the
//!   public half, the same facts `/api/status` serves without a session.
//! * `<base>/status/<CALL>` — one account's award totals, the ClubLog log's
//!   size and age, and its last few alerts: the `/api/me/station` and
//!   `/api/me/alerts` facts. Only for accounts with a log loaded; an account
//!   with no matrix has no totals, and publishing zeroes would read as a
//!   station that has worked nothing — the same reason the station card says
//!   "no log loaded" instead.
//!
//! Every payload carries `ts` so a consumer can tell a retained message from
//! a dead publisher (HA's `expire_after` keys on arrival, not on `ts`, but a
//! human reading the broker can see the age).

use crate::db::SentAlert;
use crate::nodes::{NodeManager, NodeStatus};
use crate::pipeline::PipelineState;
use crate::users::UserService;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// A minute: a dashboard tile wants to be roughly current, and every tick
/// walks each account's matrix for its totals. Faster buys nothing a human
/// would see on a DXCC count.
const TICK: Duration = Duration::from_secs(60);

/// How many alerts each account's status carries. A dashboard tile has room
/// for a handful; the full history is `/api/me/alerts`.
const RECENT_ALERTS: usize = 5;

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs() as i64
}

/// The server-wide payload. A pure function of its inputs so the shape can
/// be pinned by a test without a broker or a running pipeline.
pub fn server_payload(
    ts: i64,
    nodes: &HashMap<String, NodeStatus>,
    spots_per_source: &HashMap<String, u64>,
) -> serde_json::Value {
    let online = nodes.values().filter(|s| s.connected).count();
    serde_json::json!({
        "ts": ts,
        "version": env!("CARGO_PKG_VERSION"),
        "clusters": nodes,
        // Counted here rather than left to every consumer's template: "7/9"
        // is the number every tile shows, and a Jinja filter chain over a
        // map is where dashboards go wrong.
        "clusters_online": online,
        "clusters_total": nodes.len(),
        "spots_per_source": spots_per_source,
    })
}

/// One alert as the status topic carries it: what a tile shows, nothing
/// else. Delivery detail (channels, errors) stays in the API, where the
/// operator who can act on it is logged in.
fn alert_summary(a: &SentAlert) -> serde_json::Value {
    serde_json::json!({
        "time_unix": a.time_unix,
        "callsign": a.callsign,
        "band": a.band,
        "mode": a.mode,
        "dxcc_name": a.dxcc_name,
        "level": a.level,
    })
}

/// One account's payload, or `None` when it has no log loaded.
pub fn user_payload(
    ts: i64,
    callsign: &str,
    stats: Option<dxca_core::matrix::MatrixStats>,
    stats_current: Option<dxca_core::matrix::MatrixStats>,
    award_stats: Option<dxca_core::matrix::AwardStats>,
    meta: Option<(i64, i64)>,
    alerts: &[SentAlert],
) -> Option<serde_json::Value> {
    let stats = stats?;
    Some(serde_json::json!({
        "ts": ts,
        "callsign": callsign,
        "stats": stats,
        "stats_current": stats_current,
        "award_stats": award_stats,
        "qso_count": meta.map(|m| m.0),
        "last_refresh_unix": meta.map(|m| m.1),
        "recent_alerts": alerts.iter().map(alert_summary).collect::<Vec<_>>(),
    }))
}

/// Publish every topic once. Errors reading one account skip that account,
/// never the rest: one corrupt row must not blank every dashboard.
fn publish_once(users: &UserService, pipeline: &PipelineState, nodes: &NodeManager) {
    let mqtt = pipeline.mqtt();
    if !mqtt.wants_status() {
        return;
    }
    let ts = now_unix();
    let sources = pipeline.source_counts.lock().unwrap().clone();
    mqtt.publish_status(
        None,
        &server_payload(ts, &nodes.statuses(), &sources).to_string(),
    );

    let accounts = match users.db.users() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("dxca: status: could not list accounts: {e}");
            return;
        }
    };
    for user in accounts {
        let alerts = users
            .db
            .sent_alerts(user.id, RECENT_ALERTS)
            .unwrap_or_default();
        let meta = users.db.matrix_meta(user.id).ok().flatten();
        if let Some(body) = user_payload(
            ts,
            &user.callsign,
            users.stats(user.id),
            users.stats_current(user.id),
            users.award_stats(user.id),
            meta,
            &alerts,
        ) {
            mqtt.publish_status(Some(&user.callsign), &body.to_string());
        }
    }
}

/// Spawn the publisher. Does nothing on an install where no MQTT destination
/// has status ticked, which is every install until an admin ticks it.
pub fn spawn(users: Arc<UserService>, pipeline: Arc<PipelineState>, nodes: Arc<NodeManager>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        // The first tick fires at once. Unlike the health watch, that is
        // wanted here: a dashboard should not wait a minute after a restart
        // to learn the clusters are back — though the nodes will mostly
        // still be connecting, so the second tick is the one that says so.
        loop {
            ticker.tick().await;
            publish_once(&users, &pipeline, &nodes);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use dxca_core::matrix::MatrixStats;

    fn node(connected: bool) -> NodeStatus {
        NodeStatus {
            state: if connected { "Live" } else { "Reconnecting" }.into(),
            connected,
            proven: connected,
            spot_count: 10,
            last_spot_unix: Some(1_794_000_000),
            attempt: 0,
        }
    }

    /// The tile reads `clusters_online`/`clusters_total`; a refactor that
    /// counted `proven` or dropped the total would turn "8/9" into nonsense
    /// on every dashboard without any test noticing.
    #[test]
    fn server_payload_counts_connected_nodes() {
        let nodes = HashMap::from([
            ("VU2CPL".to_string(), node(true)),
            ("W3LPL".to_string(), node(true)),
            ("KST2Mac".to_string(), node(false)),
        ]);
        let v = server_payload(42, &nodes, &HashMap::from([("JTDX".to_string(), 5)]));
        assert_eq!(v["ts"], 42);
        assert_eq!(v["clusters_online"], 2);
        assert_eq!(v["clusters_total"], 3);
        assert_eq!(v["clusters"]["KST2Mac"]["connected"], false);
        assert_eq!(v["spots_per_source"]["JTDX"], 5);
    }

    /// An account with no log loaded must publish nothing rather than a
    /// row of zeroes, which a dashboard would show as "worked 0 entities".
    #[test]
    fn an_account_without_a_log_publishes_nothing() {
        assert!(user_payload(1, "VU2XX", None, None, None, None, &[]).is_none());
    }

    #[test]
    fn user_payload_carries_totals_and_trimmed_alerts() {
        let stats = MatrixStats {
            dxcc_worked: 300,
            dxcc_confirmed: 280,
            slots_worked: 1500,
            slots_confirmed: 1200,
            challenge_worked: 1100,
            challenge_confirmed: 900,
        };
        let alert = SentAlert {
            time_unix: 1_794_000_100,
            callsign: "3Y0J".into(),
            frequency_hz: 14_074_000,
            mode: "FT8".into(),
            band: "20M".into(),
            dxcc_name: "Bouvet".into(),
            level: "new_dxcc".into(),
            source: "VU2CPL".into(),
            spotter: String::new(),
            snr_db: Some(-12),
            award_ref: String::new(),
            delivered: true,
            error: String::new(),
            channels: Default::default(),
            offset_hz: None,
        };
        let v = user_payload(
            7,
            "VU2CPL",
            Some(stats),
            Some(stats),
            None,
            Some((25_000, 1_793_990_000)),
            &[alert],
        )
        .expect("a loaded log publishes");
        assert_eq!(v["callsign"], "VU2CPL");
        assert_eq!(v["stats"]["dxcc_worked"], 300);
        assert_eq!(v["stats"]["slots_confirmed"], 1200);
        assert_eq!(v["qso_count"], 25_000);
        assert_eq!(v["recent_alerts"][0]["callsign"], "3Y0J");
        assert_eq!(v["recent_alerts"][0]["dxcc_name"], "Bouvet");
        // Delivery detail stays behind the login.
        assert!(v["recent_alerts"][0].get("error").is_none());
        assert!(v["recent_alerts"][0].get("channels").is_none());
    }
}

//! MQTT spot publishing — the panadapter feed.
//!
//! A sibling of [`crate::broadcast`] rather than a variant of it: a UDP
//! destination is a fire-and-forget datagram to an address, while an MQTT
//! destination is a *connection* with credentials, a keepalive and a
//! reconnect story. Folding the two into one struct would have meant a
//! dummy IP on every MQTT row and a dummy topic on every UDP one.
//!
//! Each spot is published twice, to sibling topics under the configured
//! base:
//!
//! ```text
//! shack/dxca/spots/json      {"callsign":"K1JT","frequency_hz":14074000,...}
//! shack/dxca/spots/cluster   DX de DXCA:  14074.0  K1JT  FT8 -10 dB  1428Z
//! ```
//!
//! The JSON is for anything that wants structure (a Node-RED flow shaping it
//! for a panadapter overlay); the cluster line is for anything that already
//! parses the de-facto DX-Spider format. Publishing both costs one extra
//! small message and means no consumer is locked out.
//!
//! QoS 0 throughout, and `try_publish` rather than `publish`: a spot feed is
//! a live stream, so dropping one when the outbound queue is full is
//! correct — blocking the pipeline on a slow broker is not.
//!
//! A destination can also opt in to **status** (`status: true`), a slow,
//! retained sibling of the feed:
//!
//! ```text
//! shack/dxca/spots/status          cluster-node health, every minute
//! shack/dxca/spots/status/VU2CPL   one account's award totals + last alerts
//! ```
//!
//! Retained, because the consumers are dashboards (Home Assistant) that
//! subscribe at their own restart and must show the current state at once,
//! not a minute later. The spot topics stay unretained: a stale spot replayed
//! to a new subscriber is a lie about the band. What goes in the payloads is
//! the server's business (`dxca-server`'s `statuspub`); this module only owns
//! the topics and the delivery.

use rumqttc::{Client, MqttOptions, QoS};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Where the credentials live is a deliberate choice: these are stored in
/// `data/dxca.db` (0600), never in `config/dxca.toml` (0644), for the same
/// reason the ClubLog API key moved there.
#[derive(Debug, Clone)]
pub struct MqttDestinationConfig {
    /// Stable identifier for the counters.
    pub name: String,
    pub host: String,
    pub port: u16,
    /// Empty username = connect anonymously.
    pub username: String,
    pub password: String,
    /// Base topic. `/json` and `/cluster` are appended.
    pub topic: String,
    pub client_id: String,
    /// Source-name allowlist; empty = all sources.
    pub allowed_sources: HashSet<String>,
    /// Ignore the dedupe verdict and publish every spot.
    pub unfiltered: bool,
    /// Publish the retained status topics as well as spots.
    pub status: bool,
}

impl MqttDestinationConfig {
    fn json_topic(&self) -> String {
        format!("{}/json", self.topic.trim_end_matches('/'))
    }
    fn cluster_topic(&self) -> String {
        format!("{}/cluster", self.topic.trim_end_matches('/'))
    }
    /// `<base>/status`, or `<base>/status/<suffix>` for one account.
    fn status_topic(&self, suffix: Option<&str>) -> String {
        let base = format!("{}/status", self.topic.trim_end_matches('/'));
        match suffix {
            Some(s) => format!("{base}/{}", topic_segment(s)),
            None => base,
        }
    }
}

/// Make a callsign safe as ONE topic level.
///
/// `/` would split a portable call (`VU2CPL/P`) into two levels, so a
/// subscriber to `status/VU2CPL/P` and one to `status/+` would disagree about
/// whether the account exists; `+` and `#` are wildcards that a broker
/// refuses in a topic it is asked to publish to. All three become `_`, and the
/// call is uppercased so `vu2cpl` and `VU2CPL` cannot be two topics.
pub fn topic_segment(call: &str) -> String {
    call.trim()
        .chars()
        .map(|c| match c {
            '/' | '+' | '#' => '_',
            c => c.to_ascii_uppercase(),
        })
        .collect()
}

/// One spot, as MQTT publishes it. Deliberately flat and self-describing:
/// a consumer should not need DXCA's source to read it.
pub struct MqttSpot<'a> {
    pub cluster_line: &'a str,
    pub source_name: &'a str,
    pub callsign: Option<&'a str>,
    pub frequency_hz: u64,
    pub snr_db: i32,
    pub mode: &'a str,
    pub mode_inferred: bool,
    pub comment: &'a str,
    pub is_cq: bool,
    pub time_unix: i64,
}

impl MqttSpot<'_> {
    fn to_json(&self) -> String {
        // Band is derived here rather than carried: every consumer wants it
        // and only the frequency is authoritative.
        let band = dxca_core::bands::band_from_hz(self.frequency_hz);
        serde_json::json!({
            "callsign": self.callsign,
            "frequency_hz": self.frequency_hz,
            "frequency_khz": self.frequency_hz as f64 / 1000.0,
            "band": band,
            "mode": self.mode,
            "mode_inferred": self.mode_inferred,
            "snr_db": self.snr_db,
            "source": self.source_name,
            "comment": self.comment,
            "is_cq": self.is_cq,
            "time_unix": self.time_unix,
        })
        .to_string()
    }
}

struct Live {
    cfg: MqttDestinationConfig,
    client: Client,
    sent: AtomicU64,
    failed: AtomicU64,
}

#[derive(Debug, Default, Clone)]
pub struct MqttCounters {
    pub sent: HashMap<String, u64>,
    pub failed: HashMap<String, u64>,
    /// Configured destinations, so the UI can say "0 of 1 connected"
    /// rather than showing nothing at all.
    pub configured: usize,
}

impl MqttCounters {
    pub fn total_sent(&self) -> u64 {
        self.sent.values().sum()
    }
    pub fn total_failed(&self) -> u64 {
        self.failed.values().sum()
    }
}

pub struct MqttPublisher {
    dests: Vec<Live>,
}

impl MqttPublisher {
    /// Connect every configured destination. Never fails: rumqttc queues
    /// and retries in its own event loop, so a broker that is down at
    /// startup simply connects later — the alternative would be an
    /// installer that refuses to boot because a broker is rebooting.
    pub fn new(configs: Vec<MqttDestinationConfig>) -> Self {
        let dests = configs
            .into_iter()
            .map(|cfg| {
                let mut opts = MqttOptions::new(cfg.client_id.clone(), cfg.host.clone(), cfg.port);
                opts.set_keep_alive(Duration::from_secs(30));
                // Empty username means anonymous; the shack broker has
                // required credentials since 2026-08-21, so this is normally
                // set.
                if !cfg.username.is_empty() {
                    opts.set_credentials(cfg.username.clone(), cfg.password.clone());
                }
                let (client, mut connection) = Client::new(opts, 64);
                // The event loop MUST be driven or nothing is ever sent.
                // Errors are the reconnect path, not a reason to stop: the
                // iterator yields them and carries on.
                std::thread::Builder::new()
                    .name(format!("dxca-mqtt-{}", cfg.name))
                    .spawn(move || {
                        for _event in connection.iter() {
                            // Draining is the point; the events themselves
                            // are acks and pings we do not act on.
                        }
                    })
                    .expect("spawn mqtt event loop");
                Live {
                    cfg,
                    client,
                    sent: AtomicU64::new(0),
                    failed: AtomicU64::new(0),
                }
            })
            .collect();
        MqttPublisher { dests }
    }

    pub fn counters(&self) -> MqttCounters {
        let mut c = MqttCounters {
            configured: self.dests.len(),
            ..Default::default()
        };
        for d in &self.dests {
            c.sent
                .insert(d.cfg.name.clone(), d.sent.load(Ordering::Relaxed));
            c.failed
                .insert(d.cfg.name.clone(), d.failed.load(Ordering::Relaxed));
        }
        c
    }

    /// Publish one retained status message to every destination that opted
    /// in. `suffix` = `None` for the server-wide topic, `Some(callsign)` for
    /// one account's.
    ///
    /// QoS 1 where spots use 0: this is one small message a minute, and a
    /// dropped one leaves a dashboard on stale numbers until the next tick —
    /// worth the ack. Still `try_publish`, so a dead broker never blocks the
    /// ticker that calls this.
    pub fn publish_status(&self, suffix: Option<&str>, body: &str) {
        for d in self.dests.iter().filter(|d| d.cfg.status) {
            let topic = d.cfg.status_topic(suffix);
            if d.client
                .try_publish(topic, QoS::AtLeastOnce, true, body.as_bytes())
                .is_ok()
            {
                d.sent.fetch_add(1, Ordering::Relaxed);
            } else {
                d.failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Whether any destination wants status — lets the ticker skip building
    /// payloads (a walk of every account's matrix) when nobody will read them.
    pub fn wants_status(&self) -> bool {
        self.dests.iter().any(|d| d.cfg.status)
    }

    /// Publish one spot to every destination that wants it.
    ///
    /// `passes_filters` is the pipeline's dedupe verdict, honoured exactly
    /// as the UDP path honours it: an `unfiltered` destination ignores it.
    pub fn publish_spot(&self, spot: &MqttSpot<'_>, passes_filters: bool) {
        if self.dests.is_empty() {
            return;
        }
        let mut json: Option<String> = None;
        for d in &self.dests {
            if !d.cfg.unfiltered && !passes_filters {
                continue;
            }
            if !d.cfg.allowed_sources.is_empty()
                && !d.cfg.allowed_sources.contains(spot.source_name)
            {
                continue;
            }
            // Serialised once, and only if some destination actually wants
            // this spot.
            let body = json.get_or_insert_with(|| spot.to_json());

            let mut ok = true;
            ok &= d
                .client
                .try_publish(d.cfg.json_topic(), QoS::AtMostOnce, false, body.as_bytes())
                .is_ok();
            ok &= d
                .client
                .try_publish(
                    d.cfg.cluster_topic(),
                    QoS::AtMostOnce,
                    false,
                    spot.cluster_line.as_bytes(),
                )
                .is_ok();
            if ok {
                d.sent.fetch_add(1, Ordering::Relaxed);
            } else {
                // Queue full or the client is shutting down. Counted, never
                // fatal: a live feed drops rather than stalls.
                d.failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot() -> MqttSpot<'static> {
        MqttSpot {
            cluster_line: "DX de DXCA:  14074.0  K1JT  FT8 -10 dB  1428Z",
            source_name: "VU2OY",
            callsign: Some("K1JT"),
            frequency_hz: 14_074_000,
            snr_db: -10,
            mode: "FT8",
            mode_inferred: false,
            comment: "FT8 -10 dB",
            is_cq: true,
            time_unix: 1_787_745_000,
        }
    }

    #[test]
    fn topics_are_siblings_of_the_configured_base() {
        let cfg = MqttDestinationConfig {
            name: "panadapter".into(),
            host: "192.168.1.169".into(),
            port: 1883,
            username: "svc".into(),
            password: "x".into(),
            // A trailing slash is the obvious typo; it must not produce
            // "shack/dxca/spots//json".
            topic: "shack/dxca/spots/".into(),
            client_id: "dxca".into(),
            allowed_sources: HashSet::new(),
            unfiltered: false,
            status: true,
        };
        assert_eq!(cfg.json_topic(), "shack/dxca/spots/json");
        assert_eq!(cfg.cluster_topic(), "shack/dxca/spots/cluster");
        assert_eq!(cfg.status_topic(None), "shack/dxca/spots/status");
        assert_eq!(
            cfg.status_topic(Some("VU2CPL")),
            "shack/dxca/spots/status/VU2CPL"
        );
    }

    /// A portable call must stay one topic level, and a wildcard character
    /// must never reach a publish — the broker would reject it and the
    /// account's status would silently never appear.
    #[test]
    fn a_callsign_becomes_exactly_one_safe_topic_level() {
        assert_eq!(topic_segment("vu2cpl/p"), "VU2CPL_P");
        assert_eq!(topic_segment(" A+B#C "), "A_B_C");
        assert_eq!(topic_segment("VU2OY"), "VU2OY");
    }

    #[test]
    fn json_carries_the_band_derived_from_frequency() {
        let v: serde_json::Value = serde_json::from_str(&spot().to_json()).unwrap();
        assert_eq!(v["callsign"], "K1JT");
        assert_eq!(v["band"], "20M", "derived, not carried");
        assert_eq!(v["frequency_khz"], 14074.0);
        assert_eq!(v["mode"], "FT8");
        assert_eq!(v["is_cq"], true);
        assert_eq!(v["comment"], "FT8 -10 dB");
    }

    #[test]
    fn a_spot_off_any_band_reports_a_null_band_rather_than_guessing() {
        let mut s = spot();
        s.frequency_hz = 13_999_000;
        let v: serde_json::Value = serde_json::from_str(&s.to_json()).unwrap();
        assert!(v["band"].is_null());
    }
}

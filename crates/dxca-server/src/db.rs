//! SQLite persistence (plan §4/§5): users, sessions, per-user config, and
//! per-user matrix cache. One bundled-SQLite connection behind a mutex —
//! shack-scale traffic, no pool needed. Secrets (ClubLog app password,
//! Telegram token) live here in plain text by design; the file is created
//! 0600 and the trade-off is documented in the README (plan §5).

use dxca_core::classify::{AlertConfig, AlertLevel};
use dxca_core::matrix::LogMatrix;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Per-user ClubLog settings — the 1.x `ClubLogConfig` fields that matter
/// server-side, with the alert toggles flattened in.
///
/// **No API key here.** It was only ever used to fetch cty.xml, which is one
/// shared file backing one shared resolver, so it moved to a server-wide
/// setting (`Db::clublog_api_key`). What remains is genuinely personal: the
/// credentials that download *this operator's* log. Stored rows may still
/// carry the old `api_key`; serde ignores it, and `adopt_legacy_api_key`
/// lifts it to the server setting once at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClubLogUserConfig {
    pub callsign: String,
    pub email: String,
    pub app_password: String,
    /// Automatic re-download interval in hours; **0 = manual only**.
    /// Per-user because each account pulls its own log with its own
    /// credentials — unlike the LoTW list, which is one shared file.
    #[serde(default = "default_refresh_hours")]
    pub refresh_hours: i64,
    // docs/AWARDS.md phase 3: the LoTW **web login**, for the QSL report
    // that carries STATE/GRIDSQUARE/IOTA — the confirmed side of WAS, VUCC
    // and IOTA, which ClubLog's export cannot provide. Log credentials, so
    // they live with the other log credentials (README §Secrets). Both
    // empty = no LoTW report, and the three awards run worked-side only.
    #[serde(default)]
    pub lotw_login: String,
    #[serde(default)]
    pub lotw_password: String,
    #[serde(flatten)]
    pub alerts: AlertConfigOpt,
}

/// Daily. A log that only moves when someone remembers the button means
/// today's QSOs keep alerting as New DXCC tomorrow; ClubLog's own ADIF
/// export is not something to pull much harder than this.
fn default_refresh_hours() -> i64 {
    24
}

/// For serde defaults on fields that must read `true` when a stored row
/// predates them — a plain `#[serde(default)]` would read `false`.
fn default_true() -> bool {
    true
}

// Hand-written rather than derived: `Default` is what a brand-new account
// gets, and serde's per-field default is what an OLD stored row gets for a
// key it predates. Deriving would have made those disagree — 0 (manual) for
// the new user, 24 for the existing one.
impl Default for ClubLogUserConfig {
    fn default() -> Self {
        ClubLogUserConfig {
            callsign: String::new(),
            email: String::new(),
            app_password: String::new(),
            refresh_hours: default_refresh_hours(),
            lotw_login: String::new(),
            lotw_password: String::new(),
            alerts: AlertConfigOpt::default(),
        }
    }
}

/// AlertConfig with serde defaults matching 1.x for the `New*` half; the
/// `Unconf*` half defaults off, so an existing account behaves exactly as it
/// did until the operator ticks something.
///
/// The 1.x `alert_unconfirmed` switch is **gone**. Stored rows may still
/// carry the key — serde ignores unknown fields, so they deserialize fine —
/// and it needs no migration: it swapped the whole comparison to the
/// confirmed sets, which the four `alert_unconf_*` levels now express
/// directly and, unlike the switch, alongside the `New*` half.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertConfigOpt {
    pub alert_new_dxcc: bool,
    pub alert_new_slot: bool,
    pub alert_new_band: bool,
    pub alert_new_mode: bool,
    pub alert_unconf_dxcc: bool,
    pub alert_unconf_slot: bool,
    pub alert_unconf_band: bool,
    pub alert_unconf_mode: bool,
    // docs/AWARDS.md phases 2–4: the award axes. A pair ticked here IS the
    // award selector — off by default, so nothing classifies differently
    // until an operator opts an award in.
    pub alert_new_grid: bool,
    pub alert_unconf_grid: bool,
    pub alert_new_state: bool,
    pub alert_unconf_state: bool,
    /// Which WAS the State levels hunt: mixed, Triple Play, or per band.
    /// Serde-defaults to mixed, so an account saved before the choice
    /// existed keeps behaving exactly as it did.
    #[serde(default)]
    pub was_scope: dxca_core::classify::WasScope,
    pub alert_new_iota: bool,
    pub alert_unconf_iota: bool,
    #[serde(default)]
    pub alert_new_zone: bool,
    #[serde(default)]
    pub alert_unconf_zone: bool,
    #[serde(default)]
    pub waz_scope: dxca_core::classify::WazScope,
    #[serde(default)]
    pub alert_marathon: bool,
}

impl Default for AlertConfigOpt {
    fn default() -> Self {
        let d = AlertConfig::default();
        AlertConfigOpt {
            alert_new_dxcc: d.alert_new_dxcc,
            alert_new_slot: d.alert_new_slot,
            alert_new_band: d.alert_new_band,
            alert_new_mode: d.alert_new_mode,
            alert_unconf_dxcc: d.alert_unconf_dxcc,
            alert_unconf_slot: d.alert_unconf_slot,
            alert_unconf_band: d.alert_unconf_band,
            alert_unconf_mode: d.alert_unconf_mode,
            alert_new_grid: d.alert_new_grid,
            alert_unconf_grid: d.alert_unconf_grid,
            alert_new_state: d.alert_new_state,
            alert_unconf_state: d.alert_unconf_state,
            was_scope: d.was_scope,
            alert_new_iota: d.alert_new_iota,
            alert_unconf_iota: d.alert_unconf_iota,
            alert_new_zone: d.alert_new_zone,
            alert_unconf_zone: d.alert_unconf_zone,
            waz_scope: d.waz_scope,
            alert_marathon: d.alert_marathon,
        }
    }
}

impl From<&AlertConfigOpt> for AlertConfig {
    fn from(o: &AlertConfigOpt) -> AlertConfig {
        AlertConfig {
            alert_new_dxcc: o.alert_new_dxcc,
            alert_new_slot: o.alert_new_slot,
            alert_new_band: o.alert_new_band,
            alert_new_mode: o.alert_new_mode,
            alert_unconf_dxcc: o.alert_unconf_dxcc,
            alert_unconf_slot: o.alert_unconf_slot,
            alert_unconf_band: o.alert_unconf_band,
            alert_unconf_mode: o.alert_unconf_mode,
            alert_new_grid: o.alert_new_grid,
            alert_unconf_grid: o.alert_unconf_grid,
            alert_new_state: o.alert_new_state,
            alert_unconf_state: o.alert_unconf_state,
            was_scope: o.was_scope,
            alert_new_iota: o.alert_new_iota,
            alert_unconf_iota: o.alert_unconf_iota,
            alert_new_zone: o.alert_new_zone,
            alert_unconf_zone: o.alert_unconf_zone,
            waz_scope: o.waz_scope,
            alert_marathon: o.alert_marathon,
        }
    }
}

/// One MQTT destination: where to publish spots for a panadapter overlay.
///
/// Server-wide, admin-edited, and stored in the database rather than
/// `config/dxca.toml` — see `Db::mqtt_destinations` for why.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MqttDestination {
    pub name: String,
    pub host: String,
    pub port: u16,
    /// Empty = connect anonymously. The shack broker has required
    /// credentials since 2026-08-21.
    pub username: String,
    pub password: String,
    /// Base topic; `/json` and `/cluster` are appended by the publisher.
    pub topic: String,
    pub client_id: String,
    /// Source-name allowlist; empty = every source.
    pub sources: Vec<String>,
    /// Publish every spot, ignoring the dedupe verdict.
    pub unfiltered: bool,
    pub enabled: bool,
}

impl Default for MqttDestination {
    fn default() -> Self {
        MqttDestination {
            name: String::new(),
            host: String::new(),
            // The shack broker's plain port. TLS (8883) would need rumqttc's
            // rustls feature turning back on — see the workspace manifest.
            port: 1883,
            username: String::new(),
            password: String::new(),
            topic: "shack/dxca/spots".into(),
            client_id: "dxca".into(),
            sources: Vec::new(),
            unfiltered: false,
            enabled: true,
        }
    }
}

/// Where the operator is — the one thing the phase-rotation spot mask
/// needs that DXCA did not already know (`docs/PHASE-ROTATION-MASK.md`).
///
/// Its own blob rather than a field on the ClubLog credentials: a locator
/// is station data, not a credential, and it will gain company as the mask
/// grows. Empty means **no mask at all** — the feature is opt-in and an
/// account that never sets a locator behaves exactly as it always has.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StationConfig {
    /// Maidenhead locator, 4 or 6 characters. Validated on write; an
    /// unparseable value simply disables the mask rather than guessing a
    /// position.
    pub locator: String,
    /// Minutes either side of sunrise and sunset counted as grey line.
    ///
    /// The operator's to set, because how long the grey line stays useful
    /// genuinely varies — with the band, the season, the path and the
    /// station. 45 is the default, matching Meridian's greyline scheduler so
    /// the two programs agree about what phase it is.
    ///
    /// `#[serde(default)]` on this struct would make a missing value 0,
    /// which would abolish the grey line rather than default it — hence the
    /// explicit default below.
    #[serde(default = "default_greyline_window_min")]
    pub greyline_window_min: u32,
}

/// 45 minutes, as in Meridian.
pub fn default_greyline_window_min() -> u32 {
    45
}

impl Default for StationConfig {
    fn default() -> Self {
        Self {
            locator: String::new(),
            greyline_window_min: default_greyline_window_min(),
        }
    }
}

/// One radio to put this account's alerts on — a FlexRadio panadapter or an
/// ExpertSDR3 panorama.
///
/// One type for both, because the two are the same three facts and a second
/// copy would quietly drift, the way the ARGB palette would have. What
/// differs between them is the default port, which is why it is an argument
/// to [`NotifyUserConfig::tci_targets`] and its Flex twin rather than a
/// field here.
///
/// A station can run more than one of either — a second rig on the bench, a
/// separate ExpertSDR3 instance driving its own panorama — and neither
/// protocol makes the second harder than the first: each radio is its own
/// session, and both client pools in `users.rs` were already keyed by
/// address. What was single was only ever the *setting*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioDevice {
    /// The radio's address. Empty switches this entry off however
    /// `enabled` is set, exactly as the single-radio field it replaces did.
    pub host: String,
    /// ExpertSDR3's TCI port. 0 means the 40001 default, so a device saved
    /// with the box cleared still reaches a stock radio.
    pub port: u16,
    /// Off keeps the address without sending to it — the way to silence one
    /// radio for an evening without retyping its IP.
    pub enabled: bool,
}

impl Default for RadioDevice {
    fn default() -> Self {
        // `enabled` true and not false: every path that creates a device —
        // the UI's Add button, the adoption below — means "use this one".
        // A serde default of false would switch a device off the moment a
        // future field made an old entry parse through `#[serde(default)]`.
        RadioDevice {
            host: String::new(),
            port: 0,
            enabled: true,
        }
    }
}

/// Per-user notification settings — the 1.x `NotificationConfig` minus the
/// macOS system notifications (headless server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotifyUserConfig {
    pub telegram_enabled: bool,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub cooldown_minutes: i64,
    pub notify_new_dxcc: bool,
    pub notify_new_slot: bool,
    pub notify_new_band: bool,
    pub notify_new_mode: bool,
    // DXCA 2.1: the confirmation-hunting half, off by default like the
    // classifier's. A level ticked here still only fires if the classifier is
    // allowed to flag it at all (My ClubLog) — notify narrows, never widens.
    pub notify_unconf_dxcc: bool,
    pub notify_unconf_slot: bool,
    pub notify_unconf_band: bool,
    pub notify_unconf_mode: bool,
    // docs/AWARDS.md phases 2–4: the award levels' Telegram narrowing.
    // Default ON, unlike their classifier flags: the classifier pair is the
    // award selector, and an operator who has just opted an award in wants
    // its pings — narrowing them away again is a second, separate choice.
    #[serde(default = "default_true")]
    pub notify_new_grid: bool,
    #[serde(default = "default_true")]
    pub notify_unconf_grid: bool,
    #[serde(default = "default_true")]
    pub notify_new_state: bool,
    #[serde(default = "default_true")]
    pub notify_unconf_state: bool,
    #[serde(default = "default_true")]
    pub notify_new_iota: bool,
    #[serde(default = "default_true")]
    pub notify_unconf_iota: bool,
    #[serde(default = "default_true")]
    pub notify_new_zone: bool,
    #[serde(default = "default_true")]
    pub notify_unconf_zone: bool,
    #[serde(default = "default_true")]
    pub notify_marathon: bool,
    // docs/AWARDS.md phase 1: the confirmation-path gate on the four `?`
    // levels. An unconfirmed entity otherwise pings for every spot —
    // including the very station that never QSLed the first QSO. Some
    // operators simply refuse to QSL, and re-working one cannot turn the
    // entity green; the ping is only worth the interruption for a station
    // that can be worked AND will confirm. Both default off, so an account
    // that has not opted in behaves exactly as before.
    /// Hold `?` pings for calls already in the log — a call worked but
    /// never confirmed is a demonstrated non-QSLer.
    #[serde(default)]
    pub notify_unconf_skip_worked: bool,
    /// Hold `?` pings for calls not on the LoTW users list — a LoTW user
    /// is the fast path to a confirmation.
    #[serde(default)]
    pub notify_unconf_lotw_only: bool,
    // DXCA 2.1: band / mode-class narrowing for Telegram only. **Empty means
    // ALL** — the same convention `broadcast_destinations.sources` uses, and
    // the reason a fresh account is not silent. Bands are resolver names
    // ("20M"), modes are award buckets ("CW"/"PHONE"/"DATA").
    pub notify_bands: Vec<String>,
    pub notify_modes: Vec<String>,
    /// LEGACY. Superseded by [`Self::notify_spotter_kind`], and kept only so
    /// an account configured before the three-way control existed is adopted
    /// rather than silently reset. Always written in step with the new field
    /// (`true` exactly when the kind is `human`), so the adoption below can
    /// never fire twice or fight a deliberate choice.
    #[serde(default)]
    pub notify_manual_only: bool,

    /// Who has to have made the spot for it to ping: `all`, `human` or
    /// `skimmer`.
    ///
    /// Three-way rather than the old "manual only" boolean, which could only
    /// ever take skimmers AWAY. Skimmers are most of the feed on a busy band,
    /// so "wake me only for what the machines heard" is as real a request as
    /// its opposite — a CW skimmer sweep is where a rare prefix usually
    /// surfaces first.
    ///
    /// Defaults to EMPTY, not to `all`: empty means "this account predates the
    /// field", which is what lets `notify_config` adopt the old boolean. A
    /// saved config always carries a real value.
    ///
    /// The Telegram half of the Spots screen's "Spotted by", and independent
    /// of it on purpose: watch everything on screen, be woken for one slice.
    #[serde(default)]
    pub notify_spotter_kind: String,
    /// Apply the phase-rotation band mask to Telegram alerts too.
    pub notify_respect_band_mask: bool,

    /// Telegram me when **no spots at all** have reached DXCA for this many
    /// minutes. `0` is off, and is the default — a new account stays silent
    /// until its operator asks for this.
    ///
    /// Catches the failure that is otherwise invisible: DXCA up, web GUI
    /// answering, and nothing arriving because the decoders were closed or
    /// every node dropped at once. It cannot report its own host dying —
    /// nothing running there could — and it cannot get out at all if the
    /// internet is what failed, since Telegram needs it.
    #[serde(default)]
    pub notify_feed_quiet_minutes: u64,

    /// Telegram me when a cluster node has been **disconnected** this many
    /// minutes. `0` is off, and is the default.
    ///
    /// Deliberately keyed on the connection, not on traffic. "Connected but
    /// no spots" is a normal state for a node — Hamalert and KST2Mac sit
    /// live with `spot_count: 0` for hours — so alerting on quiet would cry
    /// wolf on a healthy feed. A dropped connection is unambiguous.
    #[serde(default)]
    pub notify_node_down_minutes: u64,

    /// Put this account's alerts on a **FlexRadio panadapter**, via the
    /// SmartSDR API on TCP 4992.
    ///
    /// Independent of `telegram_enabled`: the two are separate sinks for the
    /// same alerts, and wanting spots on the radio without a phone buzzing
    /// is an entirely reasonable way to run. Everything that narrows
    /// Telegram — levels, bands, modes, spotter kind, band mask, cooldown —
    /// narrows this too, so one set of choices governs both.
    #[serde(default)]
    pub flex_enabled: bool,
    /// The radio's address. Empty switches the sink off however
    /// `flex_enabled` is set.
    #[serde(default)]
    pub flex_host: String,
    /// SmartSDR's API port. 0 means the 4992 default.
    #[serde(default)]
    pub flex_port: u16,

    /// Every FlexRadio this account's alerts go to.
    ///
    /// The `tci_devices` twin below, with the same contract: **this list is
    /// the truth**, `flex_host`/`flex_port` above are the single-radio
    /// fields it grew out of and are kept in step by `set_notify_config`,
    /// and an account saved before the list existed has its one address
    /// adopted into it by `notify_config`.
    #[serde(default)]
    pub flex_devices: Vec<RadioDevice>,

    // How long a spot stays on the panadapter, per level. 0 means the
    // default shown against each.
    //
    // The ladder matters more than the numbers. A **New DXCC** is worth
    // leaving up for an hour — you may be mid-QSO when it appears and still
    // want to find it afterwards. A **New Band or Mode** is worth about as
    // long as you would stay on a band looking for it. Everything below
    // that — New Slot and the four worked-but-unconfirmed levels — is worth
    // knowing about only while the station is still calling.
    //
    // That last floor is what keeps the display usable: those levels are
    // most of the alert traffic, and at nine nodes a twenty-minute life
    // would paint the whole band inside an hour, burying the one red mark
    // the feature exists to show.
    /// New DXCC. Default 60.
    #[serde(default)]
    pub flex_life_dxcc_minutes: u64,
    /// New Band and New Mode. Default 15.
    #[serde(default)]
    pub flex_life_band_mode_minutes: u64,
    /// New Slot and the four `?` levels. Default 1.
    #[serde(default)]
    pub flex_life_other_minutes: u64,

    /// Put this account's alerts on an **ExpertSDR3 panorama**, via the TCI
    /// protocol on WebSocket 40001.
    ///
    /// The Flex fields above for a different make of radio, and independent
    /// of them in every direction: a station can run one, the other, both,
    /// or neither, and either without Telegram. Everything that narrows
    /// Telegram — levels, bands, modes, spotter kind, band mask, cooldown —
    /// narrows this too, so one set of choices governs all three sinks.
    #[serde(default)]
    pub tci_enabled: bool,
    /// The radio's address. Empty switches the sink off however
    /// `tci_enabled` is set.
    #[serde(default)]
    pub tci_host: String,
    /// ExpertSDR3's TCI port. 0 means the 40001 default.
    #[serde(default)]
    pub tci_port: u16,

    /// Every ExpertSDR3 radio this account's alerts go to.
    ///
    /// **This list is the truth**; `tci_host`/`tci_port` above are the
    /// single-radio fields it grew out of, kept in step by
    /// `set_notify_config` so a row stays readable by a DXCA that predates
    /// the list. An account saved before this existed carries no list at
    /// all, and `notify_config` adopts its one address into a single entry —
    /// the same move `notify_spotter_kind` makes for the old boolean beside
    /// it, and for the same reason: an operator who configured a radio must
    /// not find it switched off because the setting learned to count past
    /// one.
    #[serde(default)]
    pub tci_devices: Vec<RadioDevice>,

    // How long a spot stays on the panorama, per level — the same ladder as
    // the Flex fields above, and for the same reason: New Slot and the four
    // `?` levels are most of the alert traffic, and a generous life on them
    // paints the whole band inside an hour.
    //
    // Kept separate from the Flex numbers rather than shared. A station with
    // both radios has two displays of different sizes and habits, and the
    // day someone wants an hour on one and ten minutes on the other, shared
    // fields would be a migration instead of a number.
    //
    // These are enforced by DXCA, not by the radio: TCI's `SPOT` has no
    // lifetime argument, so the client sends `SPOT_DELETE` when the time is
    // up. A spot therefore outlives its deadline if DXCA is restarted in
    // between — the server has no record of what is on the panorama.
    /// New DXCC. Default 60.
    #[serde(default)]
    pub tci_life_dxcc_minutes: u64,
    /// New Band and New Mode. Default 15.
    #[serde(default)]
    pub tci_life_band_mode_minutes: u64,
    /// New Slot and the four `?` levels. Default 1.
    #[serde(default)]
    pub tci_life_other_minutes: u64,
}

impl Default for NotifyUserConfig {
    fn default() -> Self {
        NotifyUserConfig {
            telegram_enabled: false,
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            cooldown_minutes: 15,
            notify_new_dxcc: true,
            notify_new_slot: true,
            notify_new_band: true,
            notify_new_mode: true,
            notify_unconf_dxcc: false,
            notify_unconf_slot: false,
            notify_unconf_band: false,
            notify_unconf_mode: false,
            notify_new_grid: true,
            notify_unconf_grid: true,
            notify_new_state: true,
            notify_unconf_state: true,
            notify_new_iota: true,
            notify_unconf_iota: true,
            notify_new_zone: true,
            notify_unconf_zone: true,
            notify_marathon: true,
            notify_unconf_skip_worked: false,
            notify_unconf_lotw_only: false,
            notify_bands: Vec::new(),
            notify_modes: Vec::new(),
            notify_manual_only: false,
            notify_spotter_kind: SPOTTER_ALL.into(),
            notify_respect_band_mask: false,
            // Off. Health alerts are the kind that become wallpaper if they
            // arrive uninvited, and an operator who has not asked for one
            // has no threshold in mind either.
            notify_feed_quiet_minutes: 0,
            notify_node_down_minutes: 0,
            flex_enabled: false,
            flex_host: String::new(),
            flex_port: 0,
            flex_devices: Vec::new(),
            flex_life_dxcc_minutes: 0,
            flex_life_band_mode_minutes: 0,
            flex_life_other_minutes: 0,
            tci_enabled: false,
            tci_host: String::new(),
            tci_port: 0,
            tci_devices: Vec::new(),
            tci_life_dxcc_minutes: 0,
            tci_life_band_mode_minutes: 0,
            tci_life_other_minutes: 0,
        }
    }
}

/// The addresses in `devices` worth sending to, with the port resolved and
/// duplicates dropped.
///
/// Resolving BEFORE deduping is the point: `192.168.1.60` with the port box
/// left blank and `192.168.1.60:40001` are one radio typed two ways, and a
/// station that ends up with both would otherwise get two marks per alert —
/// and, on TCI, two deletions racing to remove one of them.
///
/// Hosts are trimmed and compared case-insensitively: a hostname is not
/// case-sensitive, and a stray space is a typo, not a second radio. A
/// different port on the same host IS a second radio — two SmartSDR or
/// ExpertSDR3 instances on one machine.
///
/// Shared by both radios rather than written twice. The dedupe rule is the
/// part that would drift, and its drift would be invisible until someone got
/// two marks per spot on one of the two.
fn radio_targets(devices: &[RadioDevice], default_port: u16) -> Vec<(String, u16)> {
    let mut out: Vec<(String, u16)> = Vec::new();
    for d in devices {
        let host = d.host.trim();
        if !d.enabled || host.is_empty() {
            continue;
        }
        let port = if d.port == 0 { default_port } else { d.port };
        if out
            .iter()
            .any(|(h, p)| *p == port && h.eq_ignore_ascii_case(host))
        {
            continue;
        }
        out.push((host.to_string(), port));
    }
    out
}

impl NotifyUserConfig {
    /// Every ExpertSDR3 address this account's alerts should reach.
    ///
    /// See [`radio_targets`] for what "resolved and deduped" means and why.
    pub fn tci_targets(&self, default_port: u16) -> Vec<(String, u16)> {
        radio_targets(&self.tci_devices, default_port)
    }

    /// Every FlexRadio address this account's alerts should reach.
    ///
    /// The [`Self::tci_targets`] twin, over the same helper: the two radios
    /// differ in what they send, not in which addresses are worth sending
    /// to.
    pub fn flex_targets(&self, default_port: u16) -> Vec<(String, u16)> {
        radio_targets(&self.flex_devices, default_port)
    }

    /// Does this spot's band/mode survive the Telegram narrowing? Empty list
    /// = no narrowing on that axis.
    /// Should a spot with this provenance ping the account?
    ///
    /// Narrows like `passes_band_mode`. An unrecognised kind — including the
    /// empty string an unadopted config would carry — reads as `all`, so a
    /// malformed value can never silence Telegram. Failing OPEN is the rule
    /// throughout this gate: a suppressed alert is a spot never learned about.
    pub fn passes_spotter(&self, is_skimmer: bool) -> bool {
        match self.notify_spotter_kind.as_str() {
            SPOTTER_HUMAN => !is_skimmer,
            SPOTTER_SKIMMER => is_skimmer,
            _ => true,
        }
    }

    /// The band mask, applied to Telegram (`docs/PHASE-ROTATION-MASK.md`
    /// milestone 4). Off by default and narrowed separately from the Spots
    /// screen, exactly as the band/mode and manual-only narrowings are:
    /// watch everything on screen, be woken only for what is workable.
    ///
    /// `band_open` is `None` when there is no locator or the band is one the
    /// model says nothing about. That is "no opinion", and no opinion never
    /// suppresses an alert — the same fail-open rule the rest of the mask
    /// follows, and it matters more here, because a suppressed Telegram is a
    /// spot the operator never learns about at all.
    pub fn passes_band_mask(&self, band_open: Option<bool>) -> bool {
        !(self.notify_respect_band_mask && band_open == Some(false))
    }

    pub fn passes_band_mode(&self, band: Option<&str>, mode_class: &str) -> bool {
        let band_ok = self.notify_bands.is_empty()
            || band.is_some_and(|b| self.notify_bands.iter().any(|x| x == b));
        let mode_ok =
            self.notify_modes.is_empty() || self.notify_modes.iter().any(|x| x == mode_class);
        band_ok && mode_ok
    }

    /// The confirmation-path gate (docs/AWARDS.md, phase 1), on the four
    /// `?` levels only. Unlike the rest of this family it narrows on the
    /// **call**, not the spot's provenance: a call already in the log is a
    /// demonstrated non-QSLer, and a call not on LoTW has no fast path to
    /// confirming, so either tick holds `?` pings for stations that cannot
    /// turn the entity green. The `New*` levels pass untouched — an ATNO
    /// is worth working whatever the QSL prospects.
    pub fn passes_unconf_gate(
        &self,
        level: AlertLevel,
        already_worked: bool,
        is_lotw: bool,
    ) -> bool {
        if !level.is_unconfirmed() {
            return true;
        }
        if self.notify_unconf_skip_worked && already_worked {
            return false;
        }
        if self.notify_unconf_lotw_only && !is_lotw {
            return false;
        }
        true
    }

    /// Which `notify_*` field gates a level — the same pairing
    /// `wants_level` applies, as a name the UI can bind to.
    ///
    /// It exists because the Alerts tab used to keep its OWN key → field
    /// table in Svelte, and that table was never extended when WAZ and the
    /// Marathon arrived in 2.19.0. The three new rows bound to
    /// `cfg[undefined]` — one shared slot, so ticking DX Marathon appeared
    /// to tick both Zone rows — and no save ever carried their fields, so
    /// `#[serde(default = "default_true")]` switched them back on every
    /// time. Serving the name kills that whole class: a level the server
    /// flags cannot reach the ladder without the field that turns it off.
    ///
    /// Keep in step with `wants_level` below — the test at the foot of this
    /// file fails if either forgets a level.
    pub fn notify_field(level: AlertLevel) -> Option<&'static str> {
        Some(match level {
            AlertLevel::NewDxcc => "notify_new_dxcc",
            AlertLevel::NewSlot => "notify_new_slot",
            AlertLevel::NewBand => "notify_new_band",
            AlertLevel::NewMode => "notify_new_mode",
            AlertLevel::UnconfDxcc => "notify_unconf_dxcc",
            AlertLevel::UnconfSlot => "notify_unconf_slot",
            AlertLevel::UnconfBand => "notify_unconf_band",
            AlertLevel::UnconfMode => "notify_unconf_mode",
            AlertLevel::NewGrid => "notify_new_grid",
            AlertLevel::UnconfGrid => "notify_unconf_grid",
            AlertLevel::NewState => "notify_new_state",
            AlertLevel::UnconfState => "notify_unconf_state",
            AlertLevel::NewIota => "notify_new_iota",
            AlertLevel::UnconfIota => "notify_unconf_iota",
            AlertLevel::NewZone => "notify_new_zone",
            AlertLevel::UnconfZone => "notify_unconf_zone",
            AlertLevel::Marathon => "notify_marathon",
            AlertLevel::Worked | AlertLevel::None => return None,
        })
    }

    /// Whether this level is wanted, over all seventeen flaggable levels.
    pub fn wants_level(&self, level: AlertLevel) -> bool {
        match level {
            AlertLevel::NewDxcc => self.notify_new_dxcc,
            AlertLevel::NewSlot => self.notify_new_slot,
            AlertLevel::NewBand => self.notify_new_band,
            AlertLevel::NewMode => self.notify_new_mode,
            AlertLevel::UnconfDxcc => self.notify_unconf_dxcc,
            AlertLevel::UnconfSlot => self.notify_unconf_slot,
            AlertLevel::UnconfBand => self.notify_unconf_band,
            AlertLevel::UnconfMode => self.notify_unconf_mode,
            AlertLevel::NewGrid => self.notify_new_grid,
            AlertLevel::UnconfGrid => self.notify_unconf_grid,
            AlertLevel::NewState => self.notify_new_state,
            AlertLevel::UnconfState => self.notify_unconf_state,
            AlertLevel::NewIota => self.notify_new_iota,
            AlertLevel::UnconfIota => self.notify_unconf_iota,
            AlertLevel::NewZone => self.notify_new_zone,
            AlertLevel::UnconfZone => self.notify_unconf_zone,
            AlertLevel::Marathon => self.notify_marathon,
            AlertLevel::Worked | AlertLevel::None => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub callsign: String,
    pub display_name: String,
    pub role: String,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

pub struct Db {
    conn: Mutex<Connection>,
}

/// How many sent alerts to keep per user. A shack roster alerts a few dozen
/// times a day, so this is weeks of history and still trivial to query.
const ALERT_HISTORY_MAX: i64 = 500;

/// One Telegram alert as it was sent — or as it failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentAlert {
    pub time_unix: i64,
    pub callsign: String,
    pub frequency_hz: i64,
    pub mode: String,
    pub band: String,
    pub dxcc_name: String,
    /// The serialized `AlertLevel`, so the UI reuses the same label and
    /// colour table the spots feed uses.
    pub level: String,
    pub source: String,
    /// The station that spotted it, when a relaying node named one. Empty
    /// for locally decoded spots, where `source` already names the receiver.
    #[serde(default)]
    pub spotter: String,
    /// Signal-to-noise as reported, so the Alerts history can carry the same
    /// dB column the spots feed does.
    ///
    /// `Option` rather than a plain integer with a default, because rows
    /// written before this column existed genuinely have no reading — and 0 dB
    /// is a real, rather good signal report. Storing 0 for "we never knew"
    /// would put a plausible lie in the history; `None` renders as an em dash.
    #[serde(default)]
    pub snr_db: Option<i64>,
    /// The award key an award-level alert fired on — the grid square, state
    /// or IOTA reference. Empty for the DXCC levels and for rows written
    /// before the award axes existed.
    #[serde(default)]
    pub award_ref: String,
    pub delivered: bool,
    /// Telegram's complaint when `delivered` is false; empty otherwise.
    pub error: String,
}

/// The three answers to "who has to have made the spot". Named here so the
/// gate, the validator and the adoption cannot drift from one another.
pub const SPOTTER_ALL: &str = "all";
pub const SPOTTER_HUMAN: &str = "human";
pub const SPOTTER_SKIMMER: &str = "skimmer";

/// `meta` key holding the MQTT destination list as a JSON array.
const MQTT_DESTINATIONS: &str = "mqtt_destinations";

/// `meta` key holding the server-wide ClubLog API key (cty.xml downloads).
const CLUBLOG_API_KEY: &str = "clublog_api_key";
/// Marks the one-time lift of a pre-2.1 per-user key. Separate from the key
/// itself so that clearing the key is not mistaken for "never migrated".
const CLUBLOG_KEY_ADOPTED: &str = "clublog_api_key_adopted";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    callsign TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    pass_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_unix INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_unix INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS user_configs (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    clublog_json TEXT NOT NULL DEFAULT '{}',
    notify_json TEXT NOT NULL DEFAULT '{}',
    station_json TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS matrices (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    matrix_json TEXT NOT NULL,
    qso_count INTEGER NOT NULL,
    last_refresh_unix INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS blacklist (
    callsign TEXT PRIMARY KEY,
    added_unix INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS alerts_sent (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    time_unix INTEGER NOT NULL,
    callsign TEXT NOT NULL,
    frequency_hz INTEGER NOT NULL,
    mode TEXT NOT NULL,
    band TEXT NOT NULL,
    dxcc_name TEXT NOT NULL,
    level TEXT NOT NULL,
    source TEXT NOT NULL,
    delivered INTEGER NOT NULL,
    error TEXT NOT NULL DEFAULT '',
    spotter TEXT NOT NULL DEFAULT '',
    snr_db INTEGER,
    award_ref TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS alerts_sent_user_time
    ON alerts_sent (user_id, time_unix DESC);
";

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs() as i64
}

type DbResult<T> = Result<T, String>;

fn db_err<E: std::fmt::Display>(e: E) -> String {
    format!("db: {e}")
}

/// Columns added to tables that already exist in the field.
///
/// `CREATE TABLE IF NOT EXISTS` is a no-op on a database that already has
/// the table, so a new column in [`SCHEMA`] reaches fresh installs only —
/// every existing install silently keeps the old shape and then fails at
/// the first query naming the column. This closes that gap.
///
/// Additive only, and deliberately so: `ADD COLUMN` is the one schema change
/// SQLite performs without rewriting the table, and a column with a default
/// cannot invalidate a row that is already there. Anything that needs to
/// drop, rename or retype belongs in a real versioned migration, not here.
const ADDED_COLUMNS: &[(&str, &str, &str)] = &[
    // table, column, full DDL for ALTER TABLE ... ADD COLUMN
    ("alerts_sent", "spotter", "spotter TEXT NOT NULL DEFAULT ''"),
    (
        "user_configs",
        "station_json",
        "station_json TEXT NOT NULL DEFAULT '{}'",
    ),
    // Nullable with no default on purpose: every row already in the table was
    // written without an SNR, and NULL is the only value that says so. A
    // `DEFAULT 0` would silently claim every historical alert was a 0 dB
    // report.
    ("alerts_sent", "snr_db", "snr_db INTEGER"),
    // Empty for every historical row — the DXCC levels never carry a key,
    // so '' is the truthful backfill, unlike snr_db's NULL above.
    (
        "alerts_sent",
        "award_ref",
        "award_ref TEXT NOT NULL DEFAULT ''",
    ),
];

/// Bring an existing database up to the current shape. Runs on every open;
/// each step is skipped when its column is already present, so it is cheap
/// and safe to repeat.
fn migrate(conn: &Connection) -> DbResult<()> {
    for (table, column, ddl) in ADDED_COLUMNS {
        let present = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db_err)?
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db_err)?
            .filter_map(Result::ok)
            .any(|name| name == *column);
        if !present {
            conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {ddl};"))
                .map_err(db_err)?;
        }
    }
    Ok(())
}

impl Db {
    pub fn open(path: &Path) -> DbResult<Db> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(db_err)?;
        }
        let conn = Connection::open(path).map_err(db_err)?;
        conn.execute_batch(SCHEMA).map_err(db_err)?;
        migrate(&conn)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(db_err)?;
        // Secrets at rest: owner-only, plan §5.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    pub fn user_count(&self) -> DbResult<i64> {
        self.conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .map_err(db_err)
    }

    pub fn create_user(
        &self,
        callsign: &str,
        display_name: &str,
        pass_hash: &str,
        role: &str,
    ) -> DbResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (callsign, display_name, pass_hash, role, created_unix)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                callsign.to_uppercase(),
                display_name,
                pass_hash,
                role,
                now_unix()
            ],
        )
        .map_err(|e| format!("create user: {e}"))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn users(&self) -> DbResult<Vec<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, callsign, display_name, role FROM users ORDER BY id")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(User {
                    id: r.get(0)?,
                    callsign: r.get(1)?,
                    display_name: r.get(2)?,
                    role: r.get(3)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    /// (user, stored password hash) by callsign, case-insensitive.
    pub fn user_by_callsign(&self, callsign: &str) -> DbResult<Option<(User, String)>> {
        self.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT id, callsign, display_name, role, pass_hash FROM users WHERE callsign = ?1",
                params![callsign.to_uppercase()],
                |r| {
                    Ok((
                        User {
                            id: r.get(0)?,
                            callsign: r.get(1)?,
                            display_name: r.get(2)?,
                            role: r.get(3)?,
                        },
                        r.get(4)?,
                    ))
                },
            )
            .optional()
            .map_err(db_err)
    }

    pub fn user_by_id(&self, id: i64) -> DbResult<Option<User>> {
        self.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT id, callsign, display_name, role FROM users WHERE id = ?1",
                params![id],
                |r| {
                    Ok(User {
                        id: r.get(0)?,
                        callsign: r.get(1)?,
                        display_name: r.get(2)?,
                        role: r.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(db_err)
    }

    /// How many accounts hold the admin role. The API uses this to refuse
    /// the two edits that cannot be undone from the web UI: removing or
    /// demoting the last admin while other accounts still exist.
    pub fn admin_count(&self) -> DbResult<i64> {
        self.conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM users WHERE role = 'admin'", [], |r| {
                r.get(0)
            })
            .map_err(db_err)
    }

    /// Delete an account. Its sessions, per-user config and worked matrix go
    /// with it through `ON DELETE CASCADE` — which is live because `open`
    /// turns `PRAGMA foreign_keys` on. (Hand-editing the file with the
    /// sqlite3 CLI does NOT: that defaults to off and orphans the children.)
    /// Returns false when no such id.
    pub fn delete_user(&self, id: i64) -> DbResult<bool> {
        let n = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM users WHERE id = ?1", params![id])
            .map_err(db_err)?;
        Ok(n > 0)
    }

    /// Patch the mutable identity fields; `None` leaves one alone. Callsign
    /// is uppercased like `create_user` does, so a rename cannot produce a
    /// row that `user_by_callsign` (which uppercases its argument) can never
    /// match — that would be an account nobody could log into.
    ///
    /// Renaming is safe for the rest of the schema: user_configs, matrices
    /// and sessions all key on user_id, and ClubLogUserConfig carries its
    /// own callsign for the ADIF download, independent of the login name.
    pub fn update_user(
        &self,
        id: i64,
        callsign: Option<&str>,
        display_name: Option<&str>,
        role: Option<&str>,
    ) -> DbResult<bool> {
        let conn = self.conn.lock().unwrap();
        let mut changed = false;
        if let Some(c) = callsign {
            conn.execute(
                "UPDATE users SET callsign = ?2 WHERE id = ?1",
                params![id, c.trim().to_uppercase()],
            )
            .map_err(|e| format!("rename user: {e}"))?;
            changed = true;
        }
        if let Some(d) = display_name {
            conn.execute(
                "UPDATE users SET display_name = ?2 WHERE id = ?1",
                params![id, d],
            )
            .map_err(db_err)?;
            changed = true;
        }
        if let Some(r) = role {
            conn.execute("UPDATE users SET role = ?2 WHERE id = ?1", params![id, r])
                .map_err(db_err)?;
            changed = true;
        }
        Ok(changed)
    }

    pub fn set_pass_hash(&self, id: i64, hash: &str) -> DbResult<()> {
        self.conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE users SET pass_hash = ?2 WHERE id = ?1",
                params![id, hash],
            )
            .map_err(db_err)?;
        Ok(())
    }

    // --- sent alerts ------------------------------------------------------
    //
    // A log of what actually went to Telegram, per user. Kept because
    // "did it alert me?" was otherwise unanswerable: the fan-out is
    // fire-and-forget on a background thread, so a spot that was flagged,
    // narrowed out, held by the cooldown or rejected by Telegram all looked
    // identical from the UI — silence.
    //
    // Failures are recorded too, with the error. A Telegram send that was
    // refused is the single most useful row on that screen and the one a
    // "sent" log that only stored successes would hide.

    /// Record one alert and prune the user's history to `ALERT_HISTORY_MAX`.
    pub fn record_sent_alert(&self, user_id: i64, a: &SentAlert) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO alerts_sent
               (user_id, time_unix, callsign, frequency_hz, mode, band,
                dxcc_name, level, source, spotter, snr_db, award_ref,
                delivered, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                user_id,
                a.time_unix,
                a.callsign,
                a.frequency_hz,
                a.mode,
                a.band,
                a.dxcc_name,
                a.level,
                a.source,
                a.spotter,
                a.snr_db,
                a.award_ref,
                a.delivered as i64,
                a.error,
            ],
        )
        .map_err(|e| format!("record alert: {e}"))?;
        // Bounded per user, not globally: one busy operator must not evict
        // another's history.
        conn.execute(
            "DELETE FROM alerts_sent WHERE user_id = ?1 AND id NOT IN
               (SELECT id FROM alerts_sent WHERE user_id = ?1
                ORDER BY id DESC LIMIT ?2)",
            params![user_id, ALERT_HISTORY_MAX],
        )
        .map_err(db_err)?;
        Ok(())
    }

    pub fn sent_alerts(&self, user_id: i64, limit: usize) -> DbResult<Vec<SentAlert>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT time_unix, callsign, frequency_hz, mode, band,
                        dxcc_name, level, source, spotter, snr_db, award_ref,
                        delivered, error
                 FROM alerts_sent WHERE user_id = ?1
                 ORDER BY time_unix DESC, id DESC LIMIT ?2",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![user_id, limit as i64], |r| {
                Ok(SentAlert {
                    time_unix: r.get(0)?,
                    callsign: r.get(1)?,
                    frequency_hz: r.get(2)?,
                    mode: r.get(3)?,
                    band: r.get(4)?,
                    dxcc_name: r.get(5)?,
                    level: r.get(6)?,
                    source: r.get(7)?,
                    spotter: r.get(8)?,
                    snr_db: r.get(9)?,
                    award_ref: r.get(10)?,
                    delivered: r.get::<_, i64>(11)? != 0,
                    error: r.get(12)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    // --- MQTT destinations ------------------------------------------------
    //
    // Stored here, NOT in config/dxca.toml, because a broker password is a
    // secret and that file is installed 0644 while this database is 0600 —
    // exactly the reasoning that moved the ClubLog API key. Kept as one JSON
    // blob in `meta`: it is a short list edited as a whole, like the alert
    // configs, and a table would buy nothing.

    pub fn mqtt_destinations(&self) -> DbResult<Vec<MqttDestination>> {
        let raw = self.meta_get(MQTT_DESTINATIONS)?.unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&raw).map_err(|e| format!("parse mqtt destinations: {e}"))
    }

    pub fn set_mqtt_destinations(&self, dests: &[MqttDestination]) -> DbResult<()> {
        let json = serde_json::to_string(dests)
            .map_err(|e| format!("serialize mqtt destinations: {e}"))?;
        self.meta_set(MQTT_DESTINATIONS, &json)
    }

    // --- blacklist --------------------------------------------------------
    //
    // Server-wide and admin-managed by design: a matching spot is dropped in
    // the pipeline, before the ring, so it is gone from the Spots table, the
    // telnet cluster server, the UDP fan-out and Telegram for every account
    // at once. That is only coherent as one shared list — a per-user drop
    // cannot exist, because the ring is shared.
    //
    // Callsigns are stored uppercase and matched exactly.

    pub fn blacklist(&self) -> DbResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT callsign FROM blacklist ORDER BY callsign")
            .map_err(db_err)?;
        let rows = stmt.query_map([], |r| r.get(0)).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    /// Returns false when the call was already listed.
    pub fn blacklist_add(&self, callsign: &str) -> DbResult<bool> {
        let n = self
            .conn
            .lock()
            .unwrap()
            .execute(
                "INSERT OR IGNORE INTO blacklist (callsign, added_unix) VALUES (?1, ?2)",
                params![callsign.trim().to_uppercase(), now_unix()],
            )
            .map_err(|e| format!("blacklist add: {e}"))?;
        Ok(n > 0)
    }

    /// Returns false when the call was not listed.
    pub fn blacklist_remove(&self, callsign: &str) -> DbResult<bool> {
        let n = self
            .conn
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM blacklist WHERE callsign = ?1",
                params![callsign.trim().to_uppercase()],
            )
            .map_err(db_err)?;
        Ok(n > 0)
    }

    // --- sessions ---------------------------------------------------------

    pub fn create_session(&self, token_hash: &str, user_id: i64, ttl_secs: i64) -> DbResult<()> {
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO sessions (token_hash, user_id, expires_unix) VALUES (?1, ?2, ?3)",
                params![token_hash, user_id, now_unix() + ttl_secs],
            )
            .map(|_| ())
            .map_err(db_err)
    }

    pub fn session_user(&self, token_hash: &str) -> DbResult<Option<User>> {
        self.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT u.id, u.callsign, u.display_name, u.role
                 FROM sessions s JOIN users u ON u.id = s.user_id
                 WHERE s.token_hash = ?1 AND s.expires_unix > ?2",
                params![token_hash, now_unix()],
                |r| {
                    Ok(User {
                        id: r.get(0)?,
                        callsign: r.get(1)?,
                        display_name: r.get(2)?,
                        role: r.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(db_err)
    }

    pub fn delete_session(&self, token_hash: &str) -> DbResult<()> {
        self.conn
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM sessions WHERE token_hash = ?1",
                params![token_hash],
            )
            .map(|_| ())
            .map_err(db_err)
    }

    // --- per-user config --------------------------------------------------

    pub fn clublog_config(&self, user_id: i64) -> DbResult<ClubLogUserConfig> {
        self.config_json(user_id, "clublog_json")
    }

    pub fn set_clublog_config(&self, user_id: i64, cfg: &ClubLogUserConfig) -> DbResult<()> {
        self.set_config_json(user_id, "clublog_json", cfg)
    }

    pub fn notify_config(&self, user_id: i64) -> DbResult<NotifyUserConfig> {
        let mut cfg: NotifyUserConfig = self.config_json(user_id, "notify_json")?;
        // An account saved before the three-way control existed carries no
        // `notify_spotter_kind` at all. Adopt what it DID say rather than
        // resetting it to "all" — someone who asked for human spots only must
        // not start being woken by skimmers because the control grew a third
        // option.
        if cfg.notify_spotter_kind.is_empty() {
            cfg.notify_spotter_kind = if cfg.notify_manual_only {
                SPOTTER_HUMAN.into()
            } else {
                SPOTTER_ALL.into()
            };
        }
        // An account saved before TCI could count past one radio carries an
        // address in `tci_host` and no list. Adopt it rather than reading it
        // as "no radios configured", which would silently stop a working
        // panorama on upgrade. An empty host adopts to nothing, which is
        // right: that was already the off switch.
        if cfg.tci_devices.is_empty() && !cfg.tci_host.trim().is_empty() {
            cfg.tci_devices.push(RadioDevice {
                host: cfg.tci_host.clone(),
                port: cfg.tci_port,
                enabled: true,
            });
        }
        // The same adoption for the other radio, and for the same reason.
        if cfg.flex_devices.is_empty() && !cfg.flex_host.trim().is_empty() {
            cfg.flex_devices.push(RadioDevice {
                host: cfg.flex_host.clone(),
                port: cfg.flex_port,
                enabled: true,
            });
        }
        Ok(cfg)
    }

    pub fn station_config(&self, user_id: i64) -> DbResult<StationConfig> {
        self.config_json(user_id, "station_json")
    }

    pub fn set_station_config(&self, user_id: i64, cfg: &StationConfig) -> DbResult<()> {
        self.set_config_json(user_id, "station_json", cfg)
    }

    /// Writes both the new field and the legacy boolean, in step. Keeping
    /// them consistent is what stops the adoption in `notify_config` from
    /// re-firing and overriding a deliberate `all` or `skimmer`.
    pub fn set_notify_config(&self, user_id: i64, cfg: &NotifyUserConfig) -> DbResult<()> {
        let mut cfg = cfg.clone();
        if cfg.notify_spotter_kind.is_empty() {
            cfg.notify_spotter_kind = SPOTTER_ALL.into();
        }
        cfg.notify_manual_only = cfg.notify_spotter_kind == SPOTTER_HUMAN;
        // Keep the single-radio fields pointing at the first device, for the
        // same reason `notify_manual_only` is kept in step above: the
        // adoption in `notify_config` fires on an EMPTY list, so a row whose
        // list was deliberately emptied must not read its old `tci_host`
        // back the next time it is loaded. Writing the pair here also leaves
        // the row intelligible to a DXCA build that predates the list.
        match cfg.tci_devices.first() {
            Some(d) => {
                cfg.tci_host = d.host.clone();
                cfg.tci_port = d.port;
            }
            None => {
                cfg.tci_host = String::new();
                cfg.tci_port = 0;
            }
        }
        match cfg.flex_devices.first() {
            Some(d) => {
                cfg.flex_host = d.host.clone();
                cfg.flex_port = d.port;
            }
            None => {
                cfg.flex_host = String::new();
                cfg.flex_port = 0;
            }
        }
        let cfg = &cfg;
        self.set_config_json(user_id, "notify_json", cfg)
    }

    fn config_json<T: serde::de::DeserializeOwned + Default>(
        &self,
        user_id: i64,
        column: &str,
    ) -> DbResult<T> {
        let sql = format!("SELECT {column} FROM user_configs WHERE user_id = ?1");
        let json: Option<String> = self
            .conn
            .lock()
            .unwrap()
            .query_row(&sql, params![user_id], |r| r.get(0))
            .optional()
            .map_err(db_err)?;
        match json {
            Some(j) => serde_json::from_str(&j).map_err(db_err),
            None => Ok(T::default()),
        }
    }

    fn set_config_json<T: Serialize>(&self, user_id: i64, column: &str, cfg: &T) -> DbResult<()> {
        let json = serde_json::to_string(cfg).map_err(db_err)?;
        let sql = format!(
            "INSERT INTO user_configs (user_id, {column}) VALUES (?1, ?2)
             ON CONFLICT(user_id) DO UPDATE SET {column} = ?2"
        );
        self.conn
            .lock()
            .unwrap()
            .execute(&sql, params![user_id, json])
            .map(|_| ())
            .map_err(db_err)
    }

    // --- per-user matrix cache -------------------------------------------

    pub fn set_matrix(&self, user_id: i64, matrix: &LogMatrix, qso_count: usize) -> DbResult<()> {
        let json = serde_json::to_string(matrix).map_err(db_err)?;
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO matrices (user_id, matrix_json, qso_count, last_refresh_unix)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(user_id) DO UPDATE SET
                   matrix_json = ?2, qso_count = ?3, last_refresh_unix = ?4",
                params![user_id, json, qso_count as i64, now_unix()],
            )
            .map(|_| ())
            .map_err(db_err)
    }

    // --- meta: small server-wide bookkeeping ------------------------------
    // Refresh timestamps live here rather than on a file's mtime, because
    // `install -m 600` rewrites mtimes on every deploy and would silently
    // reset the LoTW clock each time.

    pub fn meta_get(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
            r.get(0)
        })
        .optional()
        .map_err(db_err)
    }

    pub fn meta_set(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
               ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )
        .map(|_| ())
        .map_err(db_err)
    }

    /// A unix stamp in `meta`, or 0 when never recorded.
    pub fn meta_unix(&self, key: &str) -> i64 {
        self.meta_get(key)
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    pub fn meta_set_now(&self, key: &str) -> DbResult<()> {
        self.meta_set(key, &now_unix().to_string())
    }

    /// The server-wide ClubLog API key (for cty.xml). Stored in the database
    /// rather than `config/dxca.toml` because that file is installed 0644
    /// while the database is 0600 — a secret belongs with the other secrets.
    pub fn clublog_api_key(&self) -> String {
        self.meta_get(CLUBLOG_API_KEY)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn set_clublog_api_key(&self, key: &str) -> DbResult<()> {
        self.meta_set(CLUBLOG_API_KEY, key)
    }

    /// Remember that ClubLog answered **403** for a particular set of
    /// credentials, so nothing retries them.
    ///
    /// ClubLog ask that a 403 stop further requests immediately — their
    /// reactive firewall blocks the source IP for repeated bad-credential
    /// traffic, which would take out every ClubLog feature on the host, for
    /// every account, not just the one with the wrong password. The automatic
    /// jobs here run on timers, so without a latch a single wrong key becomes
    /// a request every refresh interval, for ever.
    ///
    /// What is stored is a **fingerprint of the credentials, not a timestamp
    /// or a flag** — that is what makes the latch self-clearing. Change the
    /// key and the fingerprint no longer matches, so requests resume with no
    /// "reset" button to find and no way to be stuck after a fix. The
    /// fingerprint is a hash: this is a 0600 database, but there is no reason
    /// for a second copy of a secret to sit in it.
    pub fn set_credentials_rejected(&self, scope: &str, fingerprint: &str) -> DbResult<()> {
        self.meta_set(&format!("clublog_403:{scope}"), fingerprint)
    }

    /// Clear a 403 latch — on any successful download, so a key that starts
    /// working again (ClubLog side fixed, quota restored) is not held down by
    /// a stale fingerprint.
    pub fn clear_credentials_rejected(&self, scope: &str) -> DbResult<()> {
        self.meta_set(&format!("clublog_403:{scope}"), "")
    }

    /// True when these exact credentials have already been rejected with a
    /// 403 and must not be sent again.
    pub fn credentials_rejected(&self, scope: &str, fingerprint: &str) -> bool {
        !fingerprint.is_empty()
            && self
                .meta_get(&format!("clublog_403:{scope}"))
                .ok()
                .flatten()
                .is_some_and(|stored| stored == fingerprint)
    }

    /// One-time adoption of a per-user key from before the setting moved, so
    /// an operator who had one in their ClubLog tab keeps working with no
    /// manual step. Returns the callsign it took the key from, for the log.
    ///
    /// Guarded by its own "already ran" flag rather than by "is the server
    /// key empty?". Those look equivalent and are not: an admin who
    /// deliberately CLEARS the key leaves it empty, and an emptiness check
    /// would re-adopt the stale key from the user row on the next restart —
    /// silently undoing them, forever. The flag is set even when no legacy
    /// key is found, so the scan happens exactly once per database.
    pub fn adopt_legacy_api_key(&self) -> DbResult<Option<String>> {
        if self.meta_get(CLUBLOG_KEY_ADOPTED)?.is_some() {
            return Ok(None);
        }
        self.meta_set(CLUBLOG_KEY_ADOPTED, "1")?;
        if !self.clublog_api_key().is_empty() {
            return Ok(None);
        }
        for user in self.users()? {
            // The field is gone from ClubLogUserConfig, so read the raw JSON.
            let raw: Option<String> = {
                let conn = self.conn.lock().unwrap();
                conn.query_row(
                    "SELECT clublog_json FROM user_configs WHERE user_id = ?1",
                    params![user.id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db_err)?
            };
            let Some(raw) = raw else { continue };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            let key = v.get("api_key").and_then(|k| k.as_str()).unwrap_or("");
            if !key.is_empty() {
                self.set_clublog_api_key(key)?;
                return Ok(Some(user.callsign));
            }
        }
        Ok(None)
    }

    /// Write a raw clublog_json blob — tests only, to forge a row in the
    /// shape an older build would have stored.
    #[cfg(test)]
    pub fn set_clublog_json_raw(&self, user_id: i64, json: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_configs (user_id, clublog_json) VALUES (?1, ?2)
               ON CONFLICT(user_id) DO UPDATE SET clublog_json = ?2",
            params![user_id, json],
        )
        .map(|_| ())
        .map_err(db_err)
    }

    /// One user's log provenance: (qso_count, last_refresh_unix). The matrix
    /// itself is already in memory, so the station card only needs these two.
    pub fn matrix_meta(&self, user_id: i64) -> DbResult<Option<(i64, i64)>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT qso_count, last_refresh_unix FROM matrices WHERE user_id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(db_err)
    }

    /// Every stored matrix: (user_id, matrix, qso_count, last_refresh).
    pub fn matrices(&self) -> DbResult<Vec<(i64, LogMatrix, i64, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT user_id, matrix_json, qso_count, last_refresh_unix FROM matrices")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get(2)?,
                    r.get(3)?,
                ))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, json, count, refresh) = row.map_err(db_err)?;
            let matrix = serde_json::from_str(&json).map_err(db_err)?;
            out.push((id, matrix, count, refresh));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flaggable level must name a `notify_*` field that really
    /// exists on the struct, and that field must be what `wants_level`
    /// reads. The bug this pins: WAZ and the Marathon reached the Alerts
    /// ladder in 2.19.0 while the UI's own key → field table stayed at
    /// fourteen entries, so their rows bound to nothing, shared one slot
    /// between them, and could never be switched off — the pings stopped
    /// only by dropping the award itself on Settings › My station ›
    /// Awards.
    #[test]
    fn every_flaggable_level_has_a_notify_field_that_gates_it() {
        let json = serde_json::to_value(NotifyUserConfig::default()).expect("serializes");
        let obj = json.as_object().expect("an object");
        for level in AlertLevel::FLAGGABLE {
            let field = NotifyUserConfig::notify_field(level)
                .unwrap_or_else(|| panic!("{} has no notify field", level.key()));
            assert!(
                obj.contains_key(field),
                "{} names {field}, which is not a field on NotifyUserConfig",
                level.key()
            );
            // The name is not merely present — it is the one the gate reads.
            let mut cfg = NotifyUserConfig::default();
            let mut row = serde_json::to_value(&cfg).expect("serializes");
            row[field] = serde_json::Value::Bool(true);
            cfg = serde_json::from_value(row.clone()).expect("parses");
            assert!(
                cfg.wants_level(level),
                "{field} on must want {}",
                level.key()
            );
            row[field] = serde_json::Value::Bool(false);
            cfg = serde_json::from_value(row).expect("parses");
            assert!(
                !cfg.wants_level(level),
                "{field} off must silence {} — a control that cannot say no",
                level.key()
            );
        }
    }

    /// Two levels sharing one field is the same fault wearing a disguise:
    /// the Marathon row appeared to tick both Zone rows because all three
    /// bound to the same (missing) slot.
    #[test]
    fn no_two_levels_share_a_notify_field() {
        let mut seen = std::collections::HashMap::new();
        for level in AlertLevel::FLAGGABLE {
            let field = NotifyUserConfig::notify_field(level).expect("has a field");
            if let Some(other) = seen.insert(field, level.key()) {
                panic!("{field} gates both {other} and {}", level.key());
            }
        }
    }

    /// An account stored before this key existed must deserialize to
    /// "off", or an upgrade would silently start suppressing alerts that
    /// used to arrive.
    #[test]
    fn manual_only_defaults_off_for_a_stored_row_that_predates_it() {
        let old_row = r#"{"telegram_enabled":true,"telegram_bot_token":"t",
            "telegram_chat_id":"c","cooldown_minutes":15,
            "notify_new_dxcc":true,"notify_bands":[],"notify_modes":[]}"#;
        let cfg: NotifyUserConfig = serde_json::from_str(old_row).expect("old row parses");
        assert!(!cfg.notify_manual_only, "must default off");
        assert!(cfg.telegram_enabled, "the rest of the row still loads");
        assert_eq!(cfg.cooldown_minutes, 15);
    }

    /// Every existing install's `notify_json` predates the TCI fields, so
    /// the upgrade must read as "off" and leave the rest of the row — the
    /// Flex settings in particular — exactly as they were. A missing key
    /// that deserialized to anything but off would start pushing an
    /// operator's alerts at an address they never entered.
    #[test]
    fn tci_defaults_off_for_a_stored_row_that_predates_it() {
        let old_row = r#"{"telegram_enabled":true,"telegram_bot_token":"t",
            "telegram_chat_id":"c","cooldown_minutes":15,
            "notify_new_dxcc":true,"notify_bands":[],"notify_modes":[],
            "flex_enabled":true,"flex_host":"192.168.1.148","flex_port":4992}"#;
        let cfg: NotifyUserConfig = serde_json::from_str(old_row).expect("old row parses");
        assert!(!cfg.tci_enabled, "must default off");
        assert_eq!(cfg.tci_host, "");
        // 0 is the sentinel for "use the default port", not a real port.
        assert_eq!(cfg.tci_port, 0);
        assert_eq!(cfg.tci_life_dxcc_minutes, 0);
        // The neighbouring radio is untouched.
        assert!(cfg.flex_enabled);
        assert_eq!(cfg.flex_host, "192.168.1.148");
    }

    /// The upgrade that matters most here: an account that configured its
    /// one radio before the list existed must keep sending to it. Reading a
    /// missing list as "no radios" would silently stop a working panorama on
    /// upgrade, which is the failure an operator would blame on the radio.
    #[test]
    fn a_single_radio_saved_before_the_list_is_adopted_into_it() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        // Written the way the first TCI build would have: host and port, no
        // list, and the Flex radio beside it configured too.
        db.set_config_json(
            uid,
            "notify_json",
            &serde_json::json!({
                "tci_enabled": true,
                "tci_host": "192.168.1.60",
                "tci_port": 40001,
                "flex_enabled": true,
                "flex_host": "192.168.1.148",
            }),
        )
        .unwrap();

        let cfg = db.notify_config(uid).unwrap();
        assert_eq!(cfg.tci_devices.len(), 1, "the one radio must survive");
        assert_eq!(cfg.tci_devices[0].host, "192.168.1.60");
        assert_eq!(cfg.tci_devices[0].port, 40001);
        assert!(cfg.tci_devices[0].enabled, "an adopted radio is on");
        assert_eq!(
            cfg.tci_targets(40001),
            vec![("192.168.1.60".to_string(), 40001)]
        );
        // The neighbouring radio is untouched, as in the pre-list test.
        assert!(cfg.flex_enabled);
        assert_eq!(cfg.flex_host, "192.168.1.148");
    }

    /// An account with no TCI at all stays at no TCI — the adoption must not
    /// invent a device out of the empty host that means "off".
    #[test]
    fn an_empty_host_adopts_to_no_devices() {
        let old_row = r#"{"telegram_enabled":true,"tci_enabled":false,"tci_host":""}"#;
        let cfg: NotifyUserConfig = serde_json::from_str(old_row).expect("old row parses");
        assert!(cfg.tci_devices.is_empty());
        assert!(cfg.tci_targets(40001).is_empty());
    }

    /// Deleting the last radio has to stick. The adoption fires on an EMPTY
    /// list, so unless the write clears the single-radio fields with it, the
    /// next load reads the old address straight back and the operator finds
    /// the radio they removed still being spotted at.
    #[test]
    fn removing_every_radio_does_not_resurrect_the_old_one() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        let mut cfg = NotifyUserConfig {
            tci_enabled: true,
            tci_devices: vec![RadioDevice {
                host: "192.168.1.60".into(),
                port: 40001,
                enabled: true,
            }],
            ..Default::default()
        };
        db.set_notify_config(uid, &cfg).unwrap();
        assert_eq!(db.notify_config(uid).unwrap().tci_devices.len(), 1);

        cfg.tci_devices.clear();
        db.set_notify_config(uid, &cfg).unwrap();

        let back = db.notify_config(uid).unwrap();
        assert!(back.tci_devices.is_empty(), "the removal must stick");
        assert_eq!(back.tci_host, "", "the legacy field is cleared with it");
        assert!(back.tci_targets(40001).is_empty());
    }

    /// Several radios round-trip, and the legacy pair tracks the first so a
    /// build that predates the list still reads something sensible.
    #[test]
    fn several_radios_roundtrip_and_the_legacy_pair_follows_the_first() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        let cfg = NotifyUserConfig {
            tci_enabled: true,
            tci_devices: vec![
                RadioDevice {
                    host: "192.168.1.60".into(),
                    port: 0,
                    enabled: true,
                },
                RadioDevice {
                    host: "192.168.1.61".into(),
                    port: 40002,
                    enabled: true,
                },
            ],
            ..Default::default()
        };
        db.set_notify_config(uid, &cfg).unwrap();

        let back = db.notify_config(uid).unwrap();
        assert_eq!(back.tci_devices.len(), 2);
        assert_eq!(back.tci_host, "192.168.1.60", "legacy follows the first");
        assert_eq!(back.tci_port, 0, "including its unset port");
        // 0 resolves to the default; the explicit port is left alone.
        assert_eq!(
            back.tci_targets(40001),
            vec![
                ("192.168.1.60".to_string(), 40001),
                ("192.168.1.61".to_string(), 40002),
            ]
        );
    }

    /// `tci_targets` is the whole gate: disabled rows, blank rows and the
    /// same radio typed twice must not each earn a mark. The duplicate is
    /// the one worth pinning — two entries for one radio would put two spots
    /// on it per alert and race two `SPOT_DELETE`s to remove one of them.
    #[test]
    fn targets_drop_the_off_the_blank_and_the_duplicated() {
        let cfg = NotifyUserConfig {
            tci_devices: vec![
                RadioDevice {
                    host: "192.168.1.60".into(),
                    port: 0,
                    enabled: true,
                },
                // The same radio, port written out in full.
                RadioDevice {
                    host: "192.168.1.60".into(),
                    port: 40001,
                    enabled: true,
                },
                // The same radio again, with a typo's worth of whitespace
                // and a hostname in the other case.
                RadioDevice {
                    host: "  Radio.local ".into(),
                    port: 40001,
                    enabled: true,
                },
                RadioDevice {
                    host: "radio.local".into(),
                    port: 40001,
                    enabled: true,
                },
                // Configured but switched off for the evening.
                RadioDevice {
                    host: "192.168.1.61".into(),
                    port: 40001,
                    enabled: false,
                },
                // A row someone added and never filled in.
                RadioDevice {
                    host: "   ".into(),
                    port: 40001,
                    enabled: true,
                },
                // A genuinely different port on the same host: two
                // ExpertSDR3 instances on one machine, and two real targets.
                RadioDevice {
                    host: "192.168.1.60".into(),
                    port: 40002,
                    enabled: true,
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            cfg.tci_targets(40001),
            vec![
                ("192.168.1.60".to_string(), 40001),
                ("Radio.local".to_string(), 40001),
                ("192.168.1.60".to_string(), 40002),
            ]
        );
    }

    /// The Flex half of the upgrade, and the risk the shared code introduces:
    /// two adoptions now run in one function, so a row carrying only ONE
    /// radio's address must not grow the other a device it never had.
    #[test]
    fn each_radio_adopts_its_own_address_and_not_the_others() {
        let (db, _p) = temp_db();

        // Flex configured, TCI never touched.
        let a = db.create_user("VU2CPL", "h", "", "admin").unwrap();
        db.set_config_json(
            a,
            "notify_json",
            &serde_json::json!({
                "flex_enabled": true,
                "flex_host": "192.168.1.148",
                "flex_port": 4992,
            }),
        )
        .unwrap();
        let cfg = db.notify_config(a).unwrap();
        assert_eq!(cfg.flex_devices.len(), 1, "the Flex radio is adopted");
        assert_eq!(cfg.flex_devices[0].host, "192.168.1.148");
        assert!(cfg.flex_devices[0].enabled);
        assert!(cfg.tci_devices.is_empty(), "TCI must not grow a device");
        assert_eq!(
            cfg.flex_targets(4992),
            vec![("192.168.1.148".to_string(), 4992)]
        );
        assert!(cfg.tci_targets(40001).is_empty());

        // And the mirror image: TCI configured, Flex never touched.
        let b = db.create_user("K1ABC", "h", "", "user").unwrap();
        db.set_config_json(
            b,
            "notify_json",
            &serde_json::json!({ "tci_enabled": true, "tci_host": "192.168.1.60" }),
        )
        .unwrap();
        let cfg = db.notify_config(b).unwrap();
        assert_eq!(cfg.tci_devices.len(), 1);
        assert!(cfg.flex_devices.is_empty(), "Flex must not grow a device");
    }

    /// A station running both radios keeps both lists, and they do not bleed
    /// into each other on the way through the mirror.
    #[test]
    fn both_radios_hold_their_own_lists() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        let cfg = NotifyUserConfig {
            flex_enabled: true,
            tci_enabled: true,
            flex_devices: vec![
                RadioDevice {
                    host: "192.168.1.148".into(),
                    port: 0,
                    enabled: true,
                },
                RadioDevice {
                    host: "192.168.1.149".into(),
                    port: 4993,
                    enabled: true,
                },
            ],
            tci_devices: vec![RadioDevice {
                host: "192.168.1.60".into(),
                port: 0,
                enabled: true,
            }],
            ..Default::default()
        };
        db.set_notify_config(uid, &cfg).unwrap();

        let back = db.notify_config(uid).unwrap();
        assert_eq!(back.flex_devices.len(), 2);
        assert_eq!(back.tci_devices.len(), 1);
        // Each legacy pair follows its OWN first device.
        assert_eq!(back.flex_host, "192.168.1.148");
        assert_eq!(back.tci_host, "192.168.1.60");
        // And each list resolves against its own default port.
        assert_eq!(
            back.flex_targets(4992),
            vec![
                ("192.168.1.148".to_string(), 4992),
                ("192.168.1.149".to_string(), 4993),
            ]
        );
        assert_eq!(
            back.tci_targets(40001),
            vec![("192.168.1.60".to_string(), 40001)]
        );
    }

    /// The Flex twin of the TCI removal test: emptying the list has to stick,
    /// or the adoption reads the old address back on the next load.
    #[test]
    fn removing_every_flex_radio_does_not_resurrect_the_old_one() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        let mut cfg = NotifyUserConfig {
            flex_enabled: true,
            flex_devices: vec![RadioDevice {
                host: "192.168.1.148".into(),
                port: 4992,
                enabled: true,
            }],
            ..Default::default()
        };
        db.set_notify_config(uid, &cfg).unwrap();
        assert_eq!(db.notify_config(uid).unwrap().flex_devices.len(), 1);

        cfg.flex_devices.clear();
        db.set_notify_config(uid, &cfg).unwrap();

        let back = db.notify_config(uid).unwrap();
        assert!(back.flex_devices.is_empty(), "the removal must stick");
        assert_eq!(back.flex_host, "", "the legacy field is cleared with it");
        assert!(back.flex_targets(4992).is_empty());
    }

    #[test]
    fn spotter_kind_narrows_in_both_directions_and_fails_open() {
        let mut n = NotifyUserConfig::default();
        // `all` by default: every spot passes, machine or not.
        assert!(n.passes_spotter(true));
        assert!(n.passes_spotter(false));

        n.notify_spotter_kind = SPOTTER_HUMAN.into();
        assert!(!n.passes_spotter(true), "a skimmer spot is held back");
        assert!(n.passes_spotter(false), "a human's still pings");

        // The direction the old boolean could not express.
        n.notify_spotter_kind = SPOTTER_SKIMMER.into();
        assert!(n.passes_spotter(true), "only the machines now");
        assert!(!n.passes_spotter(false), "a human's is held back");

        // Anything unrecognised must FAIL OPEN. A suppressed Telegram is a
        // spot never learned about, so a bad value may not silence the gate.
        n.notify_spotter_kind = "nonsense".into();
        assert!(n.passes_spotter(true));
        assert!(n.passes_spotter(false));
        n.notify_spotter_kind = String::new();
        assert!(
            n.passes_spotter(true),
            "an unadopted config pings for everything"
        );
    }

    #[test]
    /// The upgrade path: an account configured with the OLD boolean must keep
    /// behaving as it did. Silently resetting it to `all` would start waking
    /// the operator for the skimmer spam they had explicitly turned off.
    fn a_config_predating_the_field_adopts_the_old_manual_only_flag() {
        let (db, _p) = temp_db();
        let uid = db.create_user("VU2CPL", "h", "", "admin").unwrap();

        // Written the way an older build would have: manual_only, no kind.
        db.set_config_json(
            uid,
            "notify_json",
            &serde_json::json!({ "telegram_enabled": true, "notify_manual_only": true }),
        )
        .unwrap();
        let cfg = db.notify_config(uid).unwrap();
        assert_eq!(cfg.notify_spotter_kind, SPOTTER_HUMAN, "adopted, not reset");
        assert!(
            !cfg.passes_spotter(true),
            "and it still holds skimmers back"
        );

        // The same for an account that never set it: `all`, not `human`.
        let other = db.create_user("K1ABC", "h", "", "user").unwrap();
        db.set_config_json(other, "notify_json", &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            db.notify_config(other).unwrap().notify_spotter_kind,
            SPOTTER_ALL
        );

        // And a deliberate choice must survive a round trip — the adoption
        // may not re-fire and drag it back to `human`.
        let mut cfg = db.notify_config(uid).unwrap();
        cfg.notify_spotter_kind = SPOTTER_ALL.into();
        db.set_notify_config(uid, &cfg).unwrap();
        let back = db.notify_config(uid).unwrap();
        assert_eq!(back.notify_spotter_kind, SPOTTER_ALL, "the choice sticks");
        assert!(
            !back.notify_manual_only,
            "and the legacy flag was written in step, so adoption cannot re-fire"
        );
    }

    #[test]
    fn empty_band_mode_lists_mean_all() {
        let n = NotifyUserConfig::default();
        assert!(n.passes_band_mode(Some("20M"), "CW"));
        assert!(n.passes_band_mode(Some("70CM"), "DATA"));
        // A spot whose frequency fell in no band still passes an unset filter
        // — silence there would be a filter nobody asked for.
        assert!(n.passes_band_mode(None, "PHONE"));
    }

    #[test]
    fn band_and_mode_narrowing_are_anded() {
        let n = NotifyUserConfig {
            notify_bands: vec!["20M".into(), "15M".into()],
            notify_modes: vec!["CW".into()],
            ..Default::default()
        };
        assert!(n.passes_band_mode(Some("20M"), "CW"));
        assert!(!n.passes_band_mode(Some("20M"), "DATA"), "mode must gate");
        assert!(!n.passes_band_mode(Some("40M"), "CW"), "band must gate");
        // Band narrowing is on, and this spot has no band at all → excluded.
        assert!(!n.passes_band_mode(None, "CW"));
    }

    #[test]
    fn wants_level_covers_all_eight_and_never_worked() {
        let all_on = NotifyUserConfig {
            notify_unconf_dxcc: true,
            notify_unconf_slot: true,
            notify_unconf_band: true,
            notify_unconf_mode: true,
            ..Default::default()
        };
        for level in AlertLevel::FLAGGABLE {
            assert!(all_on.wants_level(level), "{level:?} should be wanted");
        }
        // Worked / None are outcomes, not alerts — never notifiable.
        assert!(!all_on.wants_level(AlertLevel::Worked));
        assert!(!all_on.wants_level(AlertLevel::None));
        // Default keeps the ? half quiet.
        let d = NotifyUserConfig::default();
        assert!(d.wants_level(AlertLevel::NewDxcc));
        assert!(!d.wants_level(AlertLevel::UnconfDxcc));
    }

    #[test]
    fn unconf_gate_narrows_only_the_question_marks() {
        let both = NotifyUserConfig {
            notify_unconf_skip_worked: true,
            notify_unconf_lotw_only: true,
            ..Default::default()
        };
        // New levels are exempt however hopeless the call.
        assert!(both.passes_unconf_gate(AlertLevel::NewDxcc, true, false));
        // With both ticks, only a new call on LoTW survives.
        assert!(both.passes_unconf_gate(AlertLevel::UnconfBand, false, true));
        assert!(!both.passes_unconf_gate(AlertLevel::UnconfBand, true, true));
        assert!(!both.passes_unconf_gate(AlertLevel::UnconfBand, false, false));
        // Each tick narrows on its own axis only.
        let skip = NotifyUserConfig {
            notify_unconf_skip_worked: true,
            ..Default::default()
        };
        assert!(!skip.passes_unconf_gate(AlertLevel::UnconfDxcc, true, false));
        assert!(skip.passes_unconf_gate(AlertLevel::UnconfDxcc, false, false));
        let lotw = NotifyUserConfig {
            notify_unconf_lotw_only: true,
            ..Default::default()
        };
        assert!(!lotw.passes_unconf_gate(AlertLevel::UnconfSlot, false, false));
        assert!(lotw.passes_unconf_gate(AlertLevel::UnconfSlot, true, true));
        // Default config: wide open — an account that has not opted in
        // behaves exactly as before the gate existed.
        let d = NotifyUserConfig::default();
        assert!(d.passes_unconf_gate(AlertLevel::UnconfDxcc, true, false));
    }

    /// The migration is the risky half of adding a column: production
    /// databases already exist, `CREATE TABLE IF NOT EXISTS` will not touch
    /// them, and the first query naming the new column would fail at
    /// runtime rather than at compile time. This builds a database with the
    /// OLD alerts_sent shape, opens it through `Db::open`, and checks the
    /// column arrives with existing rows intact.
    #[test]
    fn opening_an_old_database_adds_the_new_columns_without_losing_rows() {
        // Unique per run: a previous failure leaves the directory behind
        // (the panic skips the cleanup at the end), and reusing the path
        // would then fail with "table users already exists" — masking the
        // real assertion with a setup error.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dxca-migrate-{}-{nanos}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dxca.db");

        // A pre-migration database, written by hand: no `spotter` column and
        // no `snr_db` either, so this fixture covers both additions.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE users (
                     id INTEGER PRIMARY KEY, callsign TEXT UNIQUE NOT NULL,
                     display_name TEXT NOT NULL DEFAULT '', pass_hash TEXT NOT NULL,
                     role TEXT NOT NULL DEFAULT 'user', created_unix INTEGER NOT NULL);
                 CREATE TABLE alerts_sent (
                     id INTEGER PRIMARY KEY,
                     user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                     time_unix INTEGER NOT NULL, callsign TEXT NOT NULL,
                     frequency_hz INTEGER NOT NULL, mode TEXT NOT NULL,
                     band TEXT NOT NULL, dxcc_name TEXT NOT NULL,
                     level TEXT NOT NULL, source TEXT NOT NULL,
                     delivered INTEGER NOT NULL, error TEXT NOT NULL DEFAULT '');
                 INSERT INTO users (id, callsign, pass_hash, created_unix)
                     VALUES (1, 'VU2CPL', 'h', 0);
                 INSERT INTO alerts_sent
                     (user_id, time_unix, callsign, frequency_hz, mode, band,
                      dxcc_name, level, source, delivered, error)
                     VALUES (1, 100, 'OLDCALL', 14074000, 'FT8', '20M',
                             'Bouvet', 'newDXCC', 'DB0SUE', 1, '');",
            )
            .unwrap();
        }

        // Opening it must migrate, not explode.
        let db = Db::open(&path).expect("an old database must still open");
        let rows = db
            .sent_alerts(1, 10)
            .expect("the new column must be queryable");
        assert_eq!(rows.len(), 1, "the existing row survives");
        assert_eq!(rows[0].callsign, "OLDCALL");
        assert_eq!(rows[0].spotter, "", "back-filled with the default");
        // NULL, not 0: the row was written before anyone recorded an SNR, and
        // 0 dB is a real report. The UI renders None as an em dash.
        assert_eq!(
            rows[0].snr_db, None,
            "a pre-migration row must not claim a 0 dB report"
        );

        // And a new row round-trips the spotter.
        db.record_sent_alert(
            1,
            &SentAlert {
                time_unix: 200,
                callsign: "3Y0J".into(),
                frequency_hz: 14_074_000,
                mode: "FT8".into(),
                band: "20M".into(),
                dxcc_name: "Bouvet".into(),
                level: "newDXCC".into(),
                source: "N2WQ-2".into(),
                spotter: "VU2XYZ".into(),
                snr_db: Some(-11),
                award_ref: String::new(),
                delivered: true,
                error: String::new(),
            },
        )
        .unwrap();
        let rows = db.sent_alerts(1, 10).unwrap();
        assert_eq!(rows[0].spotter, "VU2XYZ", "newest first");
        assert_eq!(rows[0].snr_db, Some(-11), "and the SNR round-trips");

        // Idempotent: opening again must not try to add it twice.
        drop(db);
        let db = Db::open(&path).expect("re-open must be a no-op");
        assert_eq!(db.sent_alerts(1, 10).unwrap().len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sent_alerts_keep_failures_and_stay_bounded_per_user() {
        let (db, _p) = temp_db();
        let a = db.create_user("VU2CPL", "h", "", "admin").unwrap();
        let b = db.create_user("K1ABC", "h", "", "user").unwrap();

        let alert = |call: &str, delivered: bool, error: &str| SentAlert {
            time_unix: 1_787_745_000,
            callsign: call.into(),
            frequency_hz: 14_074_000,
            mode: "FT8".into(),
            band: "20M".into(),
            dxcc_name: "INDIA".into(),
            level: "newDXCC".into(),
            source: "VU2OY".into(),
            spotter: String::new(),
            snr_db: Some(-7),
            award_ref: String::new(),
            delivered,
            error: error.into(),
        };

        db.record_sent_alert(a, &alert("VU2ZZZ", true, "")).unwrap();
        // A refused send is the row most worth keeping — it is why the
        // history exists at all.
        db.record_sent_alert(a, &alert("P5DX", false, "chat not found"))
            .unwrap();
        db.record_sent_alert(b, &alert("W1AW", true, "")).unwrap();

        let rows = db.sent_alerts(a, 100).unwrap();
        assert_eq!(rows.len(), 2, "B's alert is not in A's history");
        let failed = rows.iter().find(|r| !r.delivered).unwrap();
        assert_eq!(failed.callsign, "P5DX");
        assert_eq!(failed.error, "chat not found", "the reason is kept");
        assert_eq!(db.sent_alerts(b, 100).unwrap().len(), 1);

        // The cap is per user, so a busy operator cannot evict another's
        // history. Push A past it and B must be untouched.
        for i in 0..(ALERT_HISTORY_MAX + 20) {
            db.record_sent_alert(a, &alert(&format!("T{i}"), true, ""))
                .unwrap();
        }
        assert_eq!(
            db.sent_alerts(a, 10_000).unwrap().len() as i64,
            ALERT_HISTORY_MAX,
            "A is pruned to the cap"
        );
        assert_eq!(
            db.sent_alerts(b, 100).unwrap().len(),
            1,
            "B's single alert survives A's flood"
        );
    }

    #[test]
    fn legacy_per_user_api_key_is_adopted_once() {
        let (db, _p) = temp_db();
        let id = db.create_user("VU2CPL", "hash", "Manoj", "admin").unwrap();

        // A row as written BEFORE the key moved: api_key is not a field of
        // ClubLogUserConfig any more, so write the raw JSON the old build
        // would have stored.
        db.set_clublog_json_raw(
            id,
            r#"{"callsign":"VU2CPL","email":"a@b.c","app_password":"p","api_key":"LEGACY123"}"#,
        )
        .unwrap();
        assert_eq!(db.clublog_api_key(), "", "server has none to begin with");

        assert_eq!(
            db.adopt_legacy_api_key().unwrap().as_deref(),
            Some("VU2CPL")
        );
        assert_eq!(db.clublog_api_key(), "LEGACY123");

        // Idempotent: a second run finds the server key set and does nothing.
        assert_eq!(db.adopt_legacy_api_key().unwrap(), None);

        // A deliberate clear must survive every later startup, even though
        // the legacy key is still sitting in the user's row. Guarding on
        // "is the server key empty?" instead of the ran-once flag would
        // silently re-adopt here and undo the admin, forever.
        db.set_clublog_api_key("").unwrap();
        assert_eq!(db.adopt_legacy_api_key().unwrap(), None);
        assert_eq!(db.clublog_api_key(), "", "the clear stands");
    }

    fn temp_db() -> (Db, std::path::PathBuf) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "dxca-test-{}-{}.db",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).unwrap(), path)
    }

    #[test]
    fn users_sessions_configs_roundtrip() {
        let (db, path) = temp_db();
        assert_eq!(db.user_count().unwrap(), 0);
        let id = db.create_user("vu2cpl", "Manoj", "hash", "admin").unwrap();
        assert_eq!(db.user_count().unwrap(), 1);
        let (user, hash) = db.user_by_callsign("VU2CPL").unwrap().unwrap();
        assert_eq!(user.id, id);
        assert!(user.is_admin());
        assert_eq!(hash, "hash");
        // Duplicate callsign refused.
        assert!(db.create_user("VU2CPL", "", "h", "user").is_err());

        db.create_session("tok", id, 3600).unwrap();
        assert_eq!(db.session_user("tok").unwrap().unwrap().callsign, "VU2CPL");
        assert!(db.session_user("other").unwrap().is_none());
        db.delete_session("tok").unwrap();
        assert!(db.session_user("tok").unwrap().is_none());

        // Configs default when unset, round-trip when set.
        assert!(!db.notify_config(id).unwrap().telegram_enabled);
        let mut cl = db.clublog_config(id).unwrap();
        assert!(cl.alerts.alert_new_dxcc);
        cl.callsign = "VU2CPL".into();
        cl.email = "op@example.com".into();
        db.set_clublog_config(id, &cl).unwrap();
        let back = db.clublog_config(id).unwrap();
        assert_eq!(back.callsign, "VU2CPL");
        assert_eq!(back.email, "op@example.com");
        assert_eq!(back.refresh_hours, 24, "default survives a round trip");

        let mut m = LogMatrix::default();
        m.record(324, "20M", "DATA", "VU2AAA", true);
        db.set_matrix(id, &m, 1).unwrap();
        let all = db.matrices().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, id);
        assert!(all[0].1.status(324).is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_sessions_do_not_authenticate() {
        let (db, path) = temp_db();
        let id = db.create_user("K1ABC", "", "h", "user").unwrap();
        db.create_session("old", id, -10).unwrap();
        assert!(db.session_user("old").unwrap().is_none());
        let _ = std::fs::remove_file(path);
    }
}

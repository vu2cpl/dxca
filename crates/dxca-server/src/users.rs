//! Per-user state over the shared spot stream (plan §5): the global DXCC
//! resolver (one cty.xml for the server), per-user matrices in memory
//! (backed by SQLite), per-user classification, the ClubLog refresh flow,
//! and Telegram alert fan-out with per-user, per-callsign cooldown.

use crate::db::{Db, NotifyUserConfig};
use dxca_connect::clublog::{self, Endpoints};
use dxca_connect::flex;
use dxca_connect::iota::IotaDirectory;
use dxca_connect::lotw;
use dxca_connect::tci;
use dxca_connect::telegram::{Telegram, escape_html};
use dxca_core::awards::StateTable;
use dxca_core::classify::{AlertClassifier, AlertConfig, AlertLevel, AwardRefs, Classification};
use dxca_core::dxcc::DxccResolver;
use dxca_core::matrix::LogMatrix;
use dxca_core::{Spot, cty};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

/// How long a cached per-user LoTW QSL report stays good before
/// `refresh_user` re-downloads it. The report must be merged on **every**
/// matrix rebuild (the rebuild starts from scratch), but re-downloading on
/// every daily ClubLog refresh would hammer ARRL for data that moves
/// slowly — so the file is cached in `data/` and refreshed on this cadence.
const LOTW_REPORT_MAX_AGE_DAYS: u64 = 7;

pub struct UserService {
    pub db: Arc<Db>,
    resolver: RwLock<Arc<DxccResolver>>,
    matrices: RwLock<HashMap<i64, Arc<LogMatrix>>>,
    /// (user_id, DX call) → last-notified unix.
    cooldowns: Mutex<HashMap<(i64, String), i64>>,
    telegram: Telegram,
    endpoints: Endpoints,
    cty_path: PathBuf,
    /// Known LoTW uploaders (global, M5 display marker).
    lotw_users: RwLock<Arc<HashSet<String>>>,
    lotw_path: PathBuf,
    /// IOTA directory (docs/AWARDS.md phase 3) — shared, like the LoTW
    /// list. `None` until the first download; spot refs pass unvalidated
    /// then, because a missing directory must not switch the award off.
    iota_dir: RwLock<Option<Arc<IotaDirectory>>>,
    iota_path: PathBuf,
    /// FCC call→state table (phase 4). `None` until the first download —
    /// and that first download is always an admin's explicit act.
    states: RwLock<Option<Arc<StateTable>>>,
    fcc_path: PathBuf,
    /// Where per-user LoTW QSL reports are cached (`data/`).
    data_dir: PathBuf,
    /// Live SmartSDR sessions, keyed by address so several accounts aimed at
    /// one radio share a connection. Made on demand and kept for the life of
    /// the process — a radio that goes away is handled inside the client by
    /// reconnecting, not by tearing this down.
    flex: Mutex<HashMap<(String, u16), Arc<flex::FlexClient>>>,
    /// Live TCI sessions, keyed and kept exactly as `flex` above.
    tci: Mutex<HashMap<(String, u16), Arc<tci::TciClient>>>,
}

/// The UTC calendar year of a unix timestamp — the DX Marathon's axis.
/// Civil-date arithmetic rather than a date crate, matching `cty.rs`.
fn year_of(unix: i64) -> i32 {
    let days = unix.div_euclid(86_400);
    // Howard Hinnant's civil_from_days, the inverse of `days_from_civil`.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }) as i32
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs() as i64
}

/// 403-latch scope for the server-wide cty.xml key.
const CTY_SCOPE: &str = "cty";

/// 403-latch scope for one account's ClubLog log credentials.
fn user_scope(user_id: i64) -> String {
    format!("user:{user_id}")
}

/// A stable, non-reversible id for a set of credentials, used to decide
/// whether the thing ClubLog rejected with a 403 is still the thing we are
/// about to send. Hashed rather than stored verbatim: the latch only needs to
/// answer "same as last time?", never to reproduce the secret.
pub fn credential_fingerprint(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for part in parts {
        h.update(part.as_bytes());
        h.update([0]); // length-independent separator: ("ab","c") != ("a","bc")
    }
    format!("{:x}", h.finalize())
}

impl UserService {
    /// Load the cached cty.xml (if present) and every stored matrix.
    pub fn new(
        db: Arc<Db>,
        data_dir: &str,
        telegram: Telegram,
        endpoints: Endpoints,
    ) -> UserService {
        let cty_path = PathBuf::from(data_dir).join("cty.xml");
        let mut resolver = DxccResolver::default();
        if let Ok(xml) = std::fs::read_to_string(&cty_path)
            && let Some(data) = cty::parse(&xml)
        {
            resolver.load(data, now_unix());
        }
        let matrices = db
            .matrices()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, m, _, _)| (id, Arc::new(m)))
            .collect();
        let lotw_path = PathBuf::from(data_dir).join("lotw-users.txt");
        let lotw_users = std::fs::read_to_string(&lotw_path)
            .map(|text| lotw::parse_users(&text))
            .unwrap_or_default();
        let iota_path = PathBuf::from(data_dir).join("iota-groups.json");
        let iota_dir = std::fs::read_to_string(&iota_path)
            .ok()
            .and_then(|text| IotaDirectory::parse(&text).ok())
            .map(Arc::new);
        let fcc_path = PathBuf::from(data_dir).join("fcc-states.txt");
        let states = std::fs::read_to_string(&fcc_path)
            .ok()
            .map(|text| Arc::new(StateTable::parse(text)));
        UserService {
            db,
            resolver: RwLock::new(Arc::new(resolver)),
            matrices: RwLock::new(matrices),
            cooldowns: Mutex::new(HashMap::new()),
            telegram,
            endpoints,
            cty_path,
            lotw_users: RwLock::new(Arc::new(lotw_users)),
            lotw_path,
            iota_dir: RwLock::new(iota_dir),
            iota_path,
            states: RwLock::new(states),
            fcc_path,
            data_dir: PathBuf::from(data_dir),
            flex: Mutex::new(HashMap::new()),
            tci: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_lotw_user(&self, callsign: &str) -> bool {
        lotw::is_user(&self.lotw_users.read().unwrap(), callsign)
    }

    /// Download and swap in the IOTA directory (blocking). Returns the
    /// group count.
    pub fn refresh_iota(&self) -> Result<usize, String> {
        let text = dxca_connect::iota::download(dxca_connect::iota::DEFAULT_URL)?;
        // Parse BEFORE saving, so an error page can never replace a good
        // cached directory on disk — the refresh_lotw arrangement.
        let dir = IotaDirectory::parse(&text)?;
        let count = dir.len();
        if let Some(parent) = self.iota_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.iota_path, &text).map_err(|e| format!("save IOTA: {e}"))?;
        *self.iota_dir.write().unwrap() = Some(Arc::new(dir));
        let _ = self.db.meta_set_now(crate::refresh::IOTA_OK_KEY);
        Ok(count)
    }

    pub fn iota_count(&self) -> usize {
        self.iota_dir
            .read()
            .unwrap()
            .as_ref()
            .map_or(0, |d| d.len())
    }

    /// Download the FCC amateur dump, distill it to the call→state table,
    /// and swap it in (blocking — a ~200 MB download plus a minute of
    /// distillation; always an explicit act or a configured cadence, never
    /// a surprise).
    pub fn refresh_fcc(&self) -> Result<usize, String> {
        let tmp = self.fcc_path.with_extension("zip.part");
        let (table, count) =
            dxca_connect::fcc::download_and_distill(dxca_connect::fcc::DEFAULT_URL, &tmp)?;
        if let Some(parent) = self.fcc_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.fcc_path, &table).map_err(|e| format!("save FCC table: {e}"))?;
        *self.states.write().unwrap() = Some(Arc::new(StateTable::parse(table)));
        let _ = self.db.meta_set_now(crate::refresh::FCC_OK_KEY);
        Ok(count)
    }

    pub fn fcc_count(&self) -> usize {
        self.states.read().unwrap().as_ref().map_or(0, |t| t.len())
    }

    /// The US state a call is licensed in, per the FCC table. `None` with
    /// no table loaded — the WAS axis simply stays quiet until an admin
    /// downloads one.
    pub fn state_of(&self, callsign: &str) -> Option<String> {
        let guard = self.states.read().unwrap();
        guard
            .as_ref()
            .and_then(|t| t.lookup(callsign))
            .map(str::to_string)
    }

    pub fn lotw_count(&self) -> usize {
        self.lotw_users.read().unwrap().len()
    }

    /// Download and reload the LoTW users list (blocking). Returns the
    /// user count.
    pub fn refresh_lotw(&self, url: &str) -> Result<usize, String> {
        let text = lotw::download(url)?;
        let users = lotw::parse_users(&text);
        if users.is_empty() {
            return Err("LoTW list parsed to zero users — not saving".into());
        }
        if let Some(dir) = self.lotw_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&self.lotw_path, &text).map_err(|e| format!("save LoTW list: {e}"))?;
        let count = users.len();
        *self.lotw_users.write().unwrap() = Arc::new(users);
        // Stamped HERE, not in the scheduler, so a manual "Refresh LoTW
        // users list" resets the automatic clock too — otherwise pressing
        // the button would be followed by the scheduler downloading the same
        // 6 MB again on its next tick.
        let _ = self.db.meta_set_now(crate::refresh::LOTW_OK_KEY);
        Ok(count)
    }

    /// Send a test message through the user's configured Telegram
    /// (blocking) — the M5 "Test" button.
    pub fn telegram_test(&self, user_id: i64) -> Result<(), String> {
        let cfg = self.db.notify_config(user_id)?;
        self.telegram.send(
            &cfg.telegram_bot_token,
            &cfg.telegram_chat_id,
            "<b>DXCA test</b>\nTelegram alerts are wired up.",
        )
    }

    /// The shared Telegram sender, for the operational alerts in
    /// [`crate::health`]. Those are not spot alerts and do not belong in the
    /// fan-out, but they go to the same per-account chat.
    pub fn telegram(&self) -> Telegram {
        self.telegram.clone()
    }

    pub fn resolver_loaded(&self) -> bool {
        self.resolver.read().unwrap().is_loaded()
    }

    pub fn entity_count(&self) -> usize {
        self.resolver.read().unwrap().entity_count()
    }

    /// The 1.x refresh flow for one user: cty.xml (when an API key is set),
    /// then the ADIF log, then the matrix build. Blocking — run it on a
    /// blocking task. Returns (qso_count, dxcc_count).
    /// Download and reload **cty.xml** (blocking). Server-wide: one file, one
    /// resolver, every account classified against it — which is why the key
    /// is a server setting and this is admin-only, matching `refresh_lotw`.
    ///
    /// It used to ride along inside `refresh_user`, keyed off whichever
    /// account happened to have an `api_key`. That meant any non-admin could
    /// swap a server-wide resource, and with automatic refresh every keyed
    /// user re-downloaded the same ~10 MB daily.
    /// Whether ClubLog has already 403'd this API key, so the automatic jobs
    /// can skip quietly instead of logging the same refusal every tick.
    pub fn cty_key_rejected(&self, api_key: &str) -> bool {
        self.db
            .credentials_rejected(CTY_SCOPE, &credential_fingerprint(&[api_key]))
    }

    /// The same question for one account's log credentials.
    pub fn user_credentials_rejected(&self, user_id: i64) -> bool {
        let Ok(cfg) = self.db.clublog_config(user_id) else {
            return false;
        };
        self.db.credentials_rejected(
            &user_scope(user_id),
            &credential_fingerprint(&[&cfg.callsign, &cfg.email, &cfg.app_password]),
        )
    }

    pub fn refresh_cty(&self, api_key: &str) -> Result<usize, String> {
        if api_key.is_empty() {
            return Err(
                "no ClubLog API key: this build has no built-in one, so an admin must \
                 set a key in Settings › Reference data"
                    .into(),
            );
        }

        // ClubLog already said 403 for this exact key. Sending it again is
        // both useless and what gets the host firewalled, so refuse here
        // rather than on their server. Changing the key changes the
        // fingerprint, which is what lets this clear itself.
        let fp = credential_fingerprint(&[api_key]);
        if self.db.credentials_rejected(CTY_SCOPE, &fp) {
            return Err(
                "ClubLog rejected this API key (HTTP 403). Set a different key in \
                        Settings › Reference data — it will not be retried until you do."
                    .into(),
            );
        }

        let xml = match clublog::download_cty(&self.endpoints, api_key) {
            Ok(xml) => xml,
            Err(e) => {
                if e.is_forbidden() {
                    let _ = self.db.set_credentials_rejected(CTY_SCOPE, &fp);
                }
                return Err(e.to_string());
            }
        };
        let _ = self.db.clear_credentials_rejected(CTY_SCOPE);
        let data = cty::parse(&xml).ok_or("cty.xml parse failed")?;
        let count = data.entities.len();
        if let Some(dir) = self.cty_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&self.cty_path, &xml).map_err(|e| format!("save cty.xml: {e}"))?;
        let mut resolver = DxccResolver::default();
        resolver.load(data, now_unix());
        *self.resolver.write().unwrap() = Arc::new(resolver);
        // Stamped here so the manual button and the scheduler share one
        // clock, exactly as refresh_lotw does.
        let _ = self.db.meta_set_now(crate::refresh::CTY_OK_KEY);
        Ok(count)
    }

    /// Download one user's ClubLog log and rebuild their matrix (blocking).
    /// Uses their email + app password only — the API key plays no part
    /// here, it was only ever for cty.xml.
    pub fn refresh_user(&self, user_id: i64) -> Result<(usize, usize), String> {
        let cfg = self.db.clublog_config(user_id)?;

        if cfg.callsign.is_empty() || cfg.email.is_empty() || cfg.app_password.is_empty() {
            return Err("need callsign, email and app password".into());
        }
        // Same 403 latch as cty.xml, per account: a wrong app password must
        // not become a request every refresh interval for ever. ClubLog
        // firewall the source IP for repeated bad credentials, and that would
        // punish every other account on this server too.
        let scope = user_scope(user_id);
        let fp = credential_fingerprint(&[&cfg.callsign, &cfg.email, &cfg.app_password]);
        if self.db.credentials_rejected(&scope, &fp) {
            return Err(
                "ClubLog rejected these credentials (HTTP 403). Check the email and \
                        app password under My station › ClubLog account — they will not be \
                        retried until one of them changes."
                    .into(),
            );
        }

        let adif = match clublog::download_adif(
            &self.endpoints,
            &cfg.callsign,
            &cfg.email,
            &cfg.app_password,
        ) {
            Ok(adif) => adif,
            Err(e) => {
                if e.is_forbidden() {
                    let _ = self.db.set_credentials_rejected(&scope, &fp);
                }
                return Err(e.to_string());
            }
        };
        let _ = self.db.clear_credentials_rejected(&scope);
        let content = match String::from_utf8(adif) {
            Ok(s) => s,
            Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(), // Latin-1 fallback
        };

        let resolver = self.resolver.read().unwrap().clone();
        if !resolver.is_loaded() {
            // The key is an admin/server setting now, so a plain user cannot
            // fix this themselves — say who can.
            return Err(
                "no cty.xml loaded — an admin must refresh it in Settings › Reference data".into(),
            );
        }
        let (mut matrix, qso_count, uncredited) =
            LogMatrix::build_from_adif_reporting(&content, &resolver);
        log_uncredited(user_id, &uncredited);
        // The LoTW QSL report — the confirmed side of WAS/VUCC/IOTA
        // (docs/AWARDS.md phase 3). Merged on every rebuild because the
        // rebuild starts from scratch; failures never fail the refresh —
        // the DXCC matrix is still worth having with the award axes a week
        // stale.
        if !cfg.lotw_login.is_empty() && !cfg.lotw_password.is_empty() {
            match self.lotw_report_text(user_id, &cfg.lotw_login, &cfg.lotw_password) {
                Ok(report) => {
                    let n = matrix.merge_lotw_confirmed(&report);
                    println!("dxca: user {user_id}: LoTW report merged, {n} award records");
                }
                Err(e) => {
                    eprintln!("dxca: user {user_id}: LoTW report skipped: {e}");
                    // **Carry the previous award axes forward.** The matrix
                    // is rebuilt from scratch, so without this a single
                    // failed download — a timeout, an ARRL outage — silently
                    // republishes an EMPTY WAS and IOTA state, and every
                    // state on the band starts alerting as new. That is the
                    // erasure that produced the NK3L/CA report; the download
                    // bug was only what triggered it.
                    //
                    // `by_grid` is NOT carried: it is rebuilt from the
                    // ClubLog log we just parsed, so the fresh value is the
                    // right one.
                    if let Some(prev) = self.matrices.read().unwrap().get(&user_id) {
                        matrix.by_state = prev.by_state.clone();
                        matrix.by_iota = prev.by_iota.clone();
                        if !matrix.by_state.is_empty() || !matrix.by_iota.is_empty() {
                            println!(
                                "dxca: user {user_id}: kept {} states and {} islands from the last good report",
                                matrix.by_state.len(),
                                matrix.by_iota.len()
                            );
                        }
                    }
                }
            }
        }
        let dxcc_count = matrix.total_dxcc_count();
        self.db.set_matrix(user_id, &matrix, qso_count)?;
        self.matrices
            .write()
            .unwrap()
            .insert(user_id, Arc::new(matrix));
        Ok((qso_count, dxcc_count))
    }

    /// This user's LoTW QSL report: the cached `data/lotw-report-<id>.adi`
    /// while it is fresh, a new download once it has aged past
    /// [`LOTW_REPORT_MAX_AGE_DAYS`] or does not exist. A failed download
    /// falls back to the stale cache rather than erroring — old
    /// confirmations beat none.
    fn lotw_report_text(
        &self,
        user_id: i64,
        login: &str,
        password: &str,
    ) -> Result<String, String> {
        let path = self.data_dir.join(format!("lotw-report-{user_id}.adi"));
        let fresh = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() < LOTW_REPORT_MAX_AGE_DAYS * 86_400);
        if fresh && let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(text);
        }
        match dxca_connect::lotwreport::download(
            dxca_connect::lotwreport::DEFAULT_BASE,
            login,
            password,
        ) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, &text) {
                    eprintln!("dxca: user {user_id}: LoTW report cache write: {e}");
                }
                Ok(text)
            }
            Err(e) => match std::fs::read_to_string(&path) {
                Ok(stale) => {
                    eprintln!("dxca: user {user_id}: LoTW report: {e} — using stale cache");
                    Ok(stale)
                }
                Err(_) => Err(e),
            },
        }
    }

    /// Award totals for one user's station card. `None` until they have a
    /// matrix — a user who has never refreshed ClubLog has nothing to count,
    /// which the card shows as "no log" rather than as four zeroes.
    pub fn stats(&self, user_id: i64) -> Option<dxca_core::matrix::MatrixStats> {
        Some(self.matrices.read().unwrap().get(&user_id)?.stats())
    }

    /// VUCC / WAS / IOTA totals, same `None`-until-a-matrix rule.
    pub fn award_stats(&self, user_id: i64) -> Option<dxca_core::matrix::AwardStats> {
        Some(self.matrices.read().unwrap().get(&user_id)?.award_stats())
    }

    /// The same totals counting **current entities only** — the ARRL
    /// deleted list left out, so they line up with the published standings.
    ///
    /// `None` also when no cty.xml is loaded: without the resolver there is
    /// no way to know which entities are deleted, and quietly returning the
    /// unfiltered totals under a "current only" label would be a lie.
    pub fn stats_current(&self, user_id: i64) -> Option<dxca_core::matrix::MatrixStats> {
        let resolver = self.resolver.read().unwrap().clone();
        if !resolver.is_loaded() {
            return None;
        }
        Some(
            self.matrices
                .read()
                .unwrap()
                .get(&user_id)?
                .stats_excluding(&resolver.deleted_adifs()),
        )
    }

    /// Per-band / per-mode counts, current entities only. `None` on the same
    /// terms as [`stats_current`](Self::stats_current).
    pub fn band_mode_stats_current(
        &self,
        user_id: i64,
    ) -> Option<dxca_core::matrix::BandModeStats> {
        let resolver = self.resolver.read().unwrap().clone();
        if !resolver.is_loaded() {
            return None;
        }
        Some(
            self.matrices
                .read()
                .unwrap()
                .get(&user_id)?
                .by_band_and_mode_excluding(&resolver.deleted_adifs()),
        )
    }

    /// Per-band and per-mode entity counts for the My ClubLog statistics
    /// card. Same in-memory matrix as `stats`, just sliced.
    pub fn band_mode_stats(&self, user_id: i64) -> Option<dxca_core::matrix::BandModeStats> {
        Some(
            self.matrices
                .read()
                .unwrap()
                .get(&user_id)?
                .by_band_and_mode(),
        )
    }

    /// The sun's elevation at this user's QTH, right now.
    ///
    /// `None` when they have set no locator, or one that will not parse —
    /// which is what keeps the phase-rotation mask opt-in. Computed **once
    /// per request** and handed to `annotate_spot`, never per spot: the sun
    /// does not move across a spot list, and a database read per row would
    /// be absurd.
    pub fn sun_phase(&self, user_id: i64) -> Option<dxca_core::solar::SunPhase> {
        let cfg = self.db.station_config(user_id).ok()?;
        let pos = dxca_core::grid::parse(&cfg.locator)?;
        Some(dxca_core::solar::phase(
            pos,
            now_unix(),
            cfg.greyline_window_min,
        ))
    }

    /// The phase plus the sunrise/sunset it was derived from, for the
    /// screen. The UI shows the two times beside the phase badge so the
    /// operator can see what the mask is reasoning from rather than having
    /// to trust it — the same disclosure the `N dimmed` count provides.
    pub fn sun_state(&self, user_id: i64) -> Option<serde_json::Value> {
        let cfg = self.db.station_config(user_id).ok()?;
        let pos = dxca_core::grid::parse(&cfg.locator)?;
        let now = now_unix();
        let t = dxca_core::solar::sun_times(pos, now);
        Some(serde_json::json!({
            "phase": dxca_core::solar::phase(pos, now, cfg.greyline_window_min).key(),
            "sunrise_unix": t.sunrise_unix,
            "sunset_unix": t.sunset_unix,
            "greyline_window_min": cfg.greyline_window_min,
            "locator": cfg.locator,
        }))
    }

    /// Classify one spot for one user (their matrix + alert toggles).
    /// None when the user has no matrix yet.
    pub fn classify(&self, user_id: i64, spot: &Spot) -> Option<Classification> {
        let matrix = self.matrices.read().unwrap().get(&user_id)?.clone();
        let resolver = self.resolver.read().unwrap().clone();
        let config: AlertConfig = (&self.db.clublog_config(user_id).ok()?.alerts).into();
        let call = spot.dx_callsign()?;
        // The spot-side award facts (docs/AWARDS.md phases 2–4). Each is
        // gathered only when this user's config could rank it — the FCC
        // lookup and the directory check are cheap, but a fleet of users
        // with the awards off should pay literally nothing.
        let state = (config.alert_new_state || config.alert_unconf_state)
            .then(|| self.state_of(&call))
            .flatten();
        let iota = spot.iota.as_deref().filter(|r| {
            // Validated when a directory is loaded; passed through when
            // not — a missing download must not switch the award off.
            self.iota_dir
                .read()
                .unwrap()
                .as_ref()
                .is_none_or(|d| d.is_valid(r))
        });
        // The zone: for a US call the FCC state gives it exactly, because
        // cty.xml has no US call-area records and would answer 5 for the
        // whole country. Everywhere else the resolver's prefix rules are
        // the better source.
        let want_zone = config.alert_new_zone || config.alert_unconf_zone || config.alert_marathon;
        let zone = want_zone
            .then(|| {
                self.state_of(&call)
                    .as_deref()
                    .and_then(dxca_core::awards::us_zone)
                    .or_else(|| resolver.zone(&call))
            })
            .flatten();
        let refs = AwardRefs {
            grid: spot.grid.as_deref(),
            iota,
            state: state.as_deref(),
            zone,
            // The Marathon runs on the calendar year, so it needs one —
            // taken from the spot, not from the clock, so a spot processed
            // either side of midnight on 31 December scores in its own year.
            year: config.alert_marathon.then(|| year_of(spot.time_unix)),
        };
        Some(
            AlertClassifier {
                matrix: &matrix,
                resolver: &resolver,
                config: &config,
            }
            .classify_spot(&call, spot.frequency_mhz(), &spot.mode, &refs),
        )
    }

    /// Is this call already in the user's log? `false` when the user has
    /// no matrix — no log means nothing to skip, and like every other
    /// narrowing in the fan-out the gate fails open.
    fn has_worked_call(&self, user_id: i64, call: &str) -> bool {
        self.matrices
            .read()
            .unwrap()
            .get(&user_id)
            .is_some_and(|m| m.has_worked_call(call))
    }

    /// Alert fan-out for one spot: every user with a matrix classifies it;
    /// matching levels go to their Telegram with per-callsign cooldown
    /// (1.x `maybeNotify`, minus macOS notifications and display filters).
    pub fn fan_out(self: &Arc<Self>, spot: &Spot) {
        let user_ids: Vec<i64> = self.matrices.read().unwrap().keys().copied().collect();
        for user_id in user_ids {
            let Ok(notify) = self.db.notify_config(user_id) else {
                continue;
            };
            // Telegram and the two radio displays are three sinks for the
            // same alerts, and any one alone is a reasonable way to run — so
            // the gate asks whether ANY of them wants this account's alerts,
            // not whether Telegram does.
            let wants_telegram = notify.telegram_enabled;
            let wants_flex =
                notify.flex_enabled && !notify.flex_targets(flex::DEFAULT_PORT).is_empty();
            let wants_tci = notify.tci_enabled && !notify.tci_targets(tci::DEFAULT_PORT).is_empty();
            if !wants_telegram && !wants_flex && !wants_tci {
                continue;
            }
            let Some(c) = self.classify(user_id, spot) else {
                continue;
            };
            let Some(call) = spot.dx_callsign() else {
                continue;
            };
            if !notify.wants_level(c.level) {
                continue;
            }
            // The confirmation-path gate (docs/AWARDS.md, phase 1). Sits
            // right after the level check so its two lookups only run for
            // levels this account actually wants — and before the band/mode
            // narrowing because it is the cheapest place a `?` ping can die.
            if !notify.passes_unconf_gate(
                c.level,
                self.has_worked_call(user_id, &call),
                self.is_lotw_user(&call),
            ) {
                continue;
            }
            // Band / mode-class narrowing is Telegram's alone — the Spots
            // screen keeps its own, so the operator can watch everything and
            // still be pinged for only CW on 20M.
            if !notify.passes_band_mode(c.band, dxca_core::modes::canonical(&spot.mode)) {
                continue;
            }
            // Machines spot relentlessly — on this station roughly three
            // quarters of the feed — so an operator who only wants to be
            // interrupted by a spot a person bothered to send can say so.
            // Independent of the Spots screen's own Manual-only, like the
            // band/mode narrowing above it.
            if !notify.passes_spotter(spot.is_skimmer) {
                continue;
            }
            // The band mask, if this account asked for it on Telegram
            // (milestone 4). Computed per SPOT here rather than per request
            // as the API does, because the fan-out runs continuously and a
            // session can outlive a sunset — a phase cached at startup would
            // narrow the wrong bands for the rest of the evening.
            //
            // New DXCC is exempt, exactly as it is on screen — and the
            // reason is stronger here. A dimmed row is still on the page and
            // one hover from being read; a held Telegram is a spot the
            // operator never learns about at all. If the model is ever
            // wrong, being wrong about the rarest catch of the year is the
            // one failure that would end this feature's welcome.
            if notify.notify_respect_band_mask && c.level != AlertLevel::NewDxcc {
                let open = self
                    .sun_phase(user_id)
                    .zip(c.band)
                    .map(|(p, b)| dxca_core::bands::plausible_in(b, p));
                if !notify.passes_band_mask(open) {
                    continue;
                }
            }
            if !self.cooldown_ok(user_id, &call, &notify) {
                continue;
            }
            // The panadapters first: each is a queue push, never a network
            // round trip, so they cost nothing to do inline and land while
            // the Telegram is still in flight.
            if wants_flex {
                self.push_flex(&notify, &c, &call, spot);
            }
            if wants_tci {
                self.push_tci(&notify, &c, &call, spot);
            }
            if !wants_telegram {
                continue;
            }
            let text = alert_html(&c, &call, spot, self.is_lotw_user(&call));
            let telegram = self.telegram.clone();
            let (token, chat) = (notify.telegram_bot_token, notify.telegram_chat_id);
            // Recorded for the My Alerts history — including failures, which
            // are the rows worth having. Built here where the classification
            // is still to hand; written after the send, with its verdict.
            let mut record = crate::db::SentAlert {
                time_unix: spot.time_unix,
                callsign: call.clone(),
                frequency_hz: spot.frequency_hz() as i64,
                mode: spot.mode.clone(),
                band: c.band.unwrap_or_default().to_string(),
                dxcc_name: c.dxcc_name.clone().unwrap_or_default(),
                level: serde_json::to_value(c.level)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
                source: spot.source_name.clone(),
                spotter: spot.spotter.clone().unwrap_or_default(),
                snr_db: Some(spot.snr_db as i64),
                award_ref: c.award_ref.clone().unwrap_or_default(),
                delivered: true,
                error: String::new(),
            };
            let this = self.clone();
            // Fire-and-forget off the pipeline: a slow Telegram round trip
            // must never stall spot processing.
            tokio::task::spawn_blocking(move || {
                if let Err(e) = telegram.send(&token, &chat, &text) {
                    eprintln!("dxca: telegram: {e}");
                    record.delivered = false;
                    record.error = e;
                }
                if let Err(e) = this.db.record_sent_alert(user_id, &record) {
                    eprintln!("dxca: alert history: {e}");
                }
            });
        }
    }

    /// Panadapter colour for each alert level, taken from the **dashboard's
    /// own dark palette** so a red dot on the radio means what a red row
    /// means on screen. `0xAARRGGBB`, opaque.
    ///
    /// The four `?` levels are the same hues at 58% mixed toward the muted
    /// grey — the `color-mix` the stylesheet performs, precomputed here
    /// because the radio wants a literal.
    ///
    /// One palette, two radios. SmartSDR wants the hex string and TCI wants
    /// the decimal, so the value lives here as a number and each sink
    /// renders it — a second copy of these constants is how the two displays
    /// would quietly drift apart.
    fn alert_argb(level: AlertLevel) -> u32 {
        match level {
            AlertLevel::NewDxcc => 0xFFF5_636B,    // --err
            AlertLevel::NewBand => 0xFF2F_81F7,    // --accent
            AlertLevel::NewMode => 0xFFFA_B219,    // --warn
            AlertLevel::NewSlot => 0xFFF0_883E,    // --alert-slot
            AlertLevel::UnconfDxcc => 0xFFC9_7479, // the four above, dimmed
            AlertLevel::UnconfBand => 0xFF56_86CA,
            AlertLevel::UnconfMode => 0xFFCC_A249,
            AlertLevel::UnconfSlot => 0xFFC6_8A5F,
            // The award axes — three new hues (--alert-iota/-state/-grid in
            // app.css), same dim rule for their ? halves.
            AlertLevel::NewIota => 0xFFA3_71F7,
            AlertLevel::NewState => 0xFFDB_61A2,
            AlertLevel::NewGrid => 0xFF39_C5CF,
            AlertLevel::UnconfIota => 0xFF99_7CCA,
            AlertLevel::UnconfState => 0xFFBA_7399,
            AlertLevel::UnconfGrid => 0xFF5C_ADB3,
            AlertLevel::NewZone => 0xFF3F_B950,
            AlertLevel::UnconfZone => 0xFF6F_A87A,
            AlertLevel::Marathon => 0xFFE3_B341,
            _ => 0xFF8C_8C8C,
        }
    }

    /// The same colour as SmartSDR wants it: `0xAARRGGBB`, uppercase.
    fn flex_color(level: AlertLevel) -> String {
        format!("0x{:08X}", Self::alert_argb(level))
    }

    /// How long each level stays on the panadapter.
    ///
    /// The ladder is the whole point. A **New DXCC** is worth leaving up for
    /// an hour — you may be mid-QSO when it appears and still want to find
    /// it afterwards. A **New Band or Mode** is worth a quarter hour, about
    /// as long as you would stay on a band looking for it. Everything
    /// below — New Slot and the four worked-but-unconfirmed levels — is
    /// worth knowing about only while the station is still calling, so it
    /// gets a minute.
    ///
    /// That floor is what keeps the display usable. Those lower levels are
    /// most of the alert traffic, and at nine nodes a twenty-minute life
    /// would paint the whole band inside an hour, burying the one red mark
    /// this feature exists to show.
    /// Adjustable per account; 0 on any field means the default beside it.
    fn flex_lifetime_secs(cfg: &NotifyUserConfig, level: AlertLevel) -> u64 {
        let or = |set: u64, default: u64| if set == 0 { default } else { set };
        let minutes = match level {
            AlertLevel::NewDxcc => or(cfg.flex_life_dxcc_minutes, 60),
            AlertLevel::NewBand | AlertLevel::NewMode => or(cfg.flex_life_band_mode_minutes, 15),
            _ => or(cfg.flex_life_other_minutes, 1),
        };
        minutes.saturating_mul(60)
    }

    /// Queue one alert onto the operator's panadapter.
    ///
    /// Never blocks: [`FlexClient::spot`] is a channel push, and the TCP
    /// session lives on its own thread. Clients are made on demand and kept,
    /// keyed by address, so several accounts pointing at one radio share a
    /// single connection rather than opening one each.
    fn push_flex(&self, notify: &NotifyUserConfig, c: &Classification, call: &str, spot: &Spot) {
        // One radio or five, the work per radio is the same queue push, so
        // this is a loop and not a special case — the `push_tci` shape, for
        // the same reason. `flex_targets` has already dropped the disabled,
        // the blank and the duplicates.
        for (host, port) in notify.flex_targets(flex::DEFAULT_PORT) {
            let client = {
                let mut map = self.flex.lock().unwrap();
                map.entry((host.clone(), port))
                    .or_insert_with(|| Arc::new(flex::FlexClient::connect(&host, port)))
                    .clone()
            };
            self.push_flex_one(&client, notify, c, call, spot);
        }
    }

    /// The body of [`Self::push_flex`] for one radio.
    ///
    /// Split out for the reason [`Self::push_tci_one`] is: every field but
    /// the address is the same for every radio, so the loop above stays
    /// about *which* radios and this stays about *what* is sent.
    fn push_flex_one(
        &self,
        client: &Arc<flex::FlexClient>,
        notify: &NotifyUserConfig,
        c: &Classification,
        call: &str,
        spot: &Spot,
    ) {
        // Level plus entity when they fit in the radio's 20 characters, the
        // entity alone when they do not — the colour already says which
        // level it is, so the entity is the half worth keeping.
        let comment = flex::comment_for(c.level.label(), c.dxcc_name.as_deref());
        client.spot(&flex::FlexSpot {
            callsign: call.to_string(),
            freq_mhz: spot.frequency_mhz(),
            mode: spot.mode.clone(),
            comment,
            // The station that heard it, falling back to the feed that
            // carried it — the panadapter has one field for this and an
            // empty one reads as a defect.
            spotter: match &spot.spotter {
                Some(s) if !s.is_empty() => s.clone(),
                _ => spot.source_name.clone(),
            },
            timestamp_unix: spot.time_unix,
            color: Self::flex_color(c.level),
            lifetime_secs: Self::flex_lifetime_secs(notify, c.level),
        });
    }

    /// How long each level stays on the ExpertSDR3 panorama.
    ///
    /// The same ladder as [`Self::flex_lifetime_secs`], read from this
    /// account's TCI fields; 0 on any field means the default beside it.
    ///
    /// The difference is who enforces it. SmartSDR is told `lifetime_seconds`
    /// and forgets the spot itself; TCI has no such argument, so the number
    /// here becomes a deadline the client keeps and acts on with
    /// `SPOT_DELETE`.
    fn tci_lifetime_secs(cfg: &NotifyUserConfig, level: AlertLevel) -> u64 {
        let or = |set: u64, default: u64| if set == 0 { default } else { set };
        let minutes = match level {
            AlertLevel::NewDxcc => or(cfg.tci_life_dxcc_minutes, 60),
            AlertLevel::NewBand | AlertLevel::NewMode => or(cfg.tci_life_band_mode_minutes, 15),
            _ => or(cfg.tci_life_other_minutes, 1),
        };
        minutes.saturating_mul(60)
    }

    /// Queue one alert onto the operator's ExpertSDR3 panorama.
    ///
    /// Never blocks, and shares one session per address, exactly as
    /// [`Self::push_flex`] does — see that method for the reasoning, which
    /// is identical.
    fn push_tci(&self, notify: &NotifyUserConfig, c: &Classification, call: &str, spot: &Spot) {
        // One radio or five, the work per radio is the same queue push, so
        // this is a loop and not a special case. `tci_targets` has already
        // dropped the disabled, the blank and the duplicates, so every
        // address here is one this account genuinely wants a mark on.
        for (host, port) in notify.tci_targets(tci::DEFAULT_PORT) {
            let client = {
                let mut map = self.tci.lock().unwrap();
                map.entry((host.clone(), port))
                    .or_insert_with(|| Arc::new(tci::TciClient::connect(&host, port)))
                    .clone()
            };
            self.push_tci_one(&client, notify, c, call, spot);
        }
    }

    /// The body of [`Self::push_tci`] for one radio.
    ///
    /// Split out so the per-radio spot is built once per address rather than
    /// once and then cloned: `TciSpot` owns its strings, and every field but
    /// the callsign is the same for every radio, so building it here keeps
    /// the loop above about *which* radios and this about *what* is sent.
    fn push_tci_one(
        &self,
        client: &Arc<tci::TciClient>,
        notify: &NotifyUserConfig,
        c: &Classification,
        call: &str,
        spot: &Spot,
    ) {
        client.spot(&tci::TciSpot {
            callsign: call.to_string(),
            // TCI wants whole hertz. `frequency_mhz` is the parsed kHz over
            // 1000, so this is a round trip back to what the cluster line
            // said — `round` and not a cast, or 14074.0 kHz arrives as
            // 14073999 Hz and the mark sits a hertz low.
            freq_hz: (spot.frequency_mhz() * 1_000_000.0).round().max(0.0) as u64,
            mode: spot.mode.clone(),
            // Roomier than SmartSDR's twenty characters, so the level and
            // the entity both fit and there is nothing to choose between.
            text: tci::text_for(c.level.label(), c.dxcc_name.as_deref()),
            color_argb: Self::alert_argb(c.level),
            lifetime_secs: Self::tci_lifetime_secs(notify, c.level),
        });
    }

    /// 1.x cooldown: per callsign, clamped 5–60 minutes, with the same
    /// opportunistic 2000-entry sweep.
    fn cooldown_ok(&self, user_id: i64, call: &str, cfg: &NotifyUserConfig) -> bool {
        let key = (user_id, call.to_uppercase());
        let now = now_unix();
        let cooldown_secs = cfg.cooldown_minutes.clamp(5, 60) * 60;
        let mut map = self.cooldowns.lock().unwrap();
        if let Some(&last) = map.get(&key)
            && now - last < cooldown_secs
        {
            return false;
        }
        if map.len() > 2000 {
            map.retain(|_, t| now - *t < 3600);
        }
        map.insert(key, now);
        true
    }
}

/// Most logs have none of these; a big one might have a handful. A cap keeps
/// a pathological log from filling the journal, and the summary line says
/// what was held back so the cap can never read as "that was all of them".
const UNCREDITED_LOG_CAP: usize = 50;

/// Print the contacts ClubLog gives no credit for, after a refresh.
///
/// These are otherwise invisible: the QSO is simply absent from the totals,
/// and the only symptom is a number that disagrees with ClubLog's by one.
/// Tracing VU24DX's 314-against-313 back to a single `ZL8AC` QSO in 65,908
/// records took a whole session — this turns that into one line at refresh
/// time, carrying the date needed to find the QSO in the log and delete it.
fn log_uncredited(user_id: i64, items: &[dxca_core::matrix::UncreditedContact]) {
    if items.is_empty() {
        return;
    }
    println!(
        "dxca: user {user_id}: {} contact(s) in this log earn no DXCC credit:",
        items.len()
    );
    for c in items.iter().take(UNCREDITED_LOG_CAP) {
        println!("dxca: user {user_id}:   {c}");
    }
    if let Some(held) = items
        .len()
        .checked_sub(UNCREDITED_LOG_CAP)
        .filter(|n| *n > 0)
    {
        println!("dxca: user {user_id}:   ... and {held} more not listed");
    }
}

/// The LoTW marker in a Telegram alert: the station uploads to Logbook of
/// the World, so a QSO with it is likely to confirm without a card chase.
///
/// The Spots table marks this with a green `●`, and matching that colour is
/// the constraint. Telegram's HTML supports `<b>`, `<i>` and `<a>` but **no
/// colour attribute**, so a plain `*` or `●` arrives in the body text colour
/// whatever we do. The only green Telegram will render is an emoji that is
/// green in the font itself — and of those, `❇️` is the one shaped like an
/// asterisk rather than a dot, a tick or a heart.
///
/// It is therefore emoji-sized rather than punctuation-sized. That is the
/// trade for the colour; there is no third option.
const LOTW_MARK: &str = "❇️";

/// The 1.x Telegram message: emoji level label, HTML-escaped, source line.
///
/// `is_lotw` appends [`LOTW_MARK`] to the callsign.
fn alert_html(c: &Classification, call: &str, spot: &Spot, is_lotw: bool) -> String {
    // The `?` half reuses its New counterpart's hue as a hollow circle: same
    // axis (DXCC/band/mode/slot), lesser catch — worked already, still not
    // confirmed. Colour says WHICH axis, filled-vs-hollow says how badly you
    // need it.
    let label = match c.level {
        AlertLevel::NewDxcc => "🔴 NEW DXCC",
        AlertLevel::NewSlot => "🟠 New Slot",
        AlertLevel::NewBand => "🔵 New Band",
        AlertLevel::NewMode => "🟡 New Mode",
        AlertLevel::UnconfDxcc => "⭕ ? DXCC (unconfirmed)",
        AlertLevel::UnconfSlot => "🟧 ? Slot (unconfirmed)",
        AlertLevel::UnconfBand => "🔷 ? Band (unconfirmed)",
        AlertLevel::UnconfMode => "🟨 ? Mode (unconfirmed)",
        AlertLevel::NewIota => "🟣 New IOTA",
        AlertLevel::NewState => "🟤 New State",
        AlertLevel::NewGrid => "🟢 New Grid",
        AlertLevel::UnconfIota => "🟪 ? IOTA (unconfirmed)",
        AlertLevel::UnconfState => "🟫 ? State (unconfirmed)",
        AlertLevel::UnconfGrid => "🟩 ? Grid (unconfirmed)",
        AlertLevel::NewZone => "🟩 New Zone",
        AlertLevel::UnconfZone => "🟢 ? Zone (unconfirmed)",
        AlertLevel::Marathon => "🏅 Marathon",
        _ => "Alert",
    };
    let dxcc = c.dxcc_name.clone().unwrap_or_default();
    let freq = format!("{:.3} MHz", spot.frequency_mhz());
    let band = c.band.unwrap_or("");
    // The award key rides in the title, where the level is — "New Grid
    // MK83" answers the whole question before the body is read.
    let key = c
        .award_ref
        .as_deref()
        .map(|r| format!(" {r}"))
        .unwrap_or_default();
    // The mark rides on the callsign, not the label, so it stays put whatever
    // the alert level is — and it goes through `escape_html` with the call
    // rather than being concatenated onto escaped output.
    let title = format!(
        "{label}{key}: {call}{}",
        if is_lotw { LOTW_MARK } else { "" }
    );
    let body = format!(
        "{}{freq}  {band}  {}  {} dB",
        if dxcc.is_empty() {
            String::new()
        } else {
            format!("{dxcc}  ")
        },
        spot.mode,
        spot.snr_db
    );
    // Who actually heard it, and which of our feeds carried it. Labelled
    // rather than joined with "via": on a phone, `Spotter:` and `Node:` are
    // scannable, and a relay chain written as prose is not.
    //
    // A node that spots under its own callsign shows both lines the same,
    // which is the honest answer — it means the node made the spot itself.
    let origin = match &spot.spotter {
        Some(sp) if !sp.is_empty() => {
            format!("Spotter: {sp}   Node: {}", spot.source_name)
        }
        // Decoded here: there is no spotting station to name.
        _ => format!("Node: {}", spot.source_name),
    };
    format!(
        "<b>{}</b>\n{}\n{}\n{}Z",
        escape_html(&title),
        escape_html(&body),
        escape_html(&origin),
        escape_html(&spot.hhmm()),
    )
}

#[cfg(test)]
mod alert_message_tests {
    use super::*;
    use dxca_core::Spot;

    fn spot(source: &str, spotter: Option<&str>) -> Spot {
        Spot {
            // 14:28 UTC on some day — hhmm() is derived from this.
            time_unix: 14 * 3600 + 28 * 60,
            snr_db: -10,
            delta_time_s: 0.0,
            delta_frequency_hz: 0,
            mode: "FT8".into(),
            mode_inferred: false,
            message: "CQ K1JT".into(),
            is_cq: true,
            comment: String::new(),
            low_confidence: false,
            off_air: false,
            dial_frequency_hz: 14_074_000,
            source_name: source.into(),
            spotter: spotter.map(str::to_string),
            is_skimmer: false,
            grid: None,
            iota: None,
        }
    }

    fn classification() -> Classification {
        Classification {
            level: AlertLevel::NewDxcc,
            dxcc_id: Some(24),
            dxcc_name: Some("Bouvet".into()),
            band: Some("20M"),
            is_beacon: false,
            award_ref: None,
        }
    }

    /// A relaying node is not the station that heard the DX. An alert that
    /// may send the operator to the radio should say which is which.
    #[test]
    fn a_relayed_alert_labels_the_spotter_and_the_node() {
        let html = alert_html(
            &classification(),
            "3Y0J",
            &spot("N2WQ-2", Some("VU2XYZ")),
            false,
        );
        assert!(html.contains("Spotter: VU2XYZ"), "got {html}");
        assert!(html.contains("Node: N2WQ-2"), "got {html}");
        assert!(!html.contains(" via "), "labelled, not prose: {html}");
    }

    /// Locally decoded: the source already names the receiver, so "via"
    /// would just repeat it.
    /// Decoded here: there is no spotting station, so no Spotter line —
    /// an empty label would read as missing data rather than as "us".
    #[test]
    fn a_local_alert_names_only_the_node() {
        let html = alert_html(&classification(), "3Y0J", &spot("MSHV", None), false);
        assert!(html.contains("Node: MSHV"), "got {html}");
        assert!(!html.contains("Spotter:"), "no empty label: {html}");
    }

    /// The spot's own time, in UTC, not the delivery time — a queued or
    /// retried alert must still say when the station was heard.
    #[test]
    fn the_alert_carries_the_spot_time_in_utc() {
        let html = alert_html(
            &classification(),
            "3Y0J",
            &spot("N2WQ-2", Some("VU2XYZ")),
            false,
        );
        assert!(html.contains("1428Z"), "got {html}");
    }

    /// A LoTW station is marked right after its callsign — the same fact the
    /// Spots table shows as a green dot, in the one form that survives
    /// Telegram's colourless HTML.
    #[test]
    fn a_lotw_station_is_marked_after_the_callsign() {
        let s = spot("N2WQ-2", Some("VU2XYZ"));
        let plain = alert_html(&classification(), "3Y0J", &s, false);
        let lotw = alert_html(&classification(), "3Y0J", &s, true);

        assert!(lotw.contains("3Y0J❇️"), "marked after the call: {lotw}");
        assert!(!plain.contains("3Y0J❇️"), "unmarked otherwise: {plain}");
        // The mark is the ONLY difference — it must not disturb the level
        // label, the body line, the origin lines or the time.
        assert_eq!(lotw.replace("3Y0J❇️", "3Y0J"), plain);
    }

    /// The mark belongs to the callsign, not to the alert level, so it is
    /// there on every level rather than only on the loudest one.
    #[test]
    fn the_lotw_mark_is_independent_of_the_alert_level() {
        for level in [
            AlertLevel::NewDxcc,
            AlertLevel::NewSlot,
            AlertLevel::UnconfBand,
            AlertLevel::UnconfMode,
        ] {
            let c = Classification {
                level,
                ..classification()
            };
            let html = alert_html(&c, "3Y0J", &spot("MSHV", None), true);
            assert!(html.contains("3Y0J❇️"), "{level:?}: {html}");
        }
    }

    /// A node that spots under its own callsign shows both labels reading
    /// the same. That is the honest answer — it means the node made the
    /// spot itself, rather than relaying somebody else's.
    #[test]
    fn a_node_spotting_under_its_own_name_shows_both_labels() {
        let html = alert_html(
            &classification(),
            "3Y0J",
            &spot("W3LPL", Some("W3LPL")),
            false,
        );
        assert!(html.contains("Spotter: W3LPL"), "got {html}");
        assert!(html.contains("Node: W3LPL"), "got {html}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A ClubLog that always says 403, counting how many times it is asked.
    /// The count is the whole point: the test is not "does a 403 produce an
    /// error" but "does the SECOND attempt reach the network at all".
    fn spawn_403_server(hits: Arc<AtomicUsize>) -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                hits.fetch_add(1, Ordering::SeqCst);
                // Drain the whole request — head AND the declared body —
                // before answering. Replying to a POST while the client is
                // still writing gets the response thrown away as a broken
                // pipe, so the test would see a network error instead of the
                // 403 it is here to check. That is a race, which means it
                // fails only sometimes: worse than not testing at all.
                let mut raw = Vec::new();
                let mut buf = [0u8; 2048];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let Some(head_end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&raw[..head_end]).to_lowercase();
                    let want = head
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")?
                                .trim()
                                .parse::<usize>()
                                .ok()
                        })
                        .unwrap_or(0);
                    if raw.len() >= head_end + 4 + want {
                        break;
                    }
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 403 Forbidden\r\nContent-Length: 9\r\nConnection: close\r\n\r\nforbidden",
                );
            }
        });
        port
    }

    fn service(port: u16) -> (UserService, std::path::PathBuf, std::path::PathBuf) {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("dxca-403-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("dxca.db");
        let _ = std::fs::remove_file(&db_path);
        let db = Arc::new(Db::open(&db_path).unwrap());
        let endpoints = Endpoints::single_base(&format!("http://127.0.0.1:{port}"));
        let svc = UserService::new(db, dir.to_str().unwrap(), Telegram::default(), endpoints);
        (svc, dir, db_path)
    }

    /// ClubLog ask that a 403 stop further requests immediately — they
    /// firewall hosts that keep sending rejected credentials, which would
    /// break every ClubLog feature for every account on the server, not just
    /// the one that was wrong. The automatic jobs run on timers, so a latch
    /// that does not hold means one bad key becomes a request every interval
    /// for ever.
    #[test]
    fn a_403_stops_the_key_being_sent_again_until_it_changes() {
        let hits = Arc::new(AtomicUsize::new(0));
        let port = spawn_403_server(hits.clone());
        let (svc, dir, _) = service(port);

        let key = "0123456789abcdef0123456789abcdef01234567";
        let first = svc.refresh_cty(key).unwrap_err();
        assert!(
            first.contains("403"),
            "the first attempt should report the 403: {first}"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1, "one request so far");

        let second = svc.refresh_cty(key).unwrap_err();
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "the same key must NOT be sent again after a 403"
        );
        assert!(
            second.contains("Settings"),
            "the refusal should say how to fix it, not just repeat the error: {second}"
        );
        assert!(svc.cty_key_rejected(key), "the latch should read as set");

        // A different key is a different credential: the latch is a
        // fingerprint, not a flag, so it lets the new one through with no
        // reset step for the admin to discover.
        let other = "fedcba9876543210fedcba9876543210fedcba98";
        assert!(
            !svc.cty_key_rejected(other),
            "a key that was never tried must not read as rejected"
        );
        let _ = svc.refresh_cty(other);
        assert_eq!(
            hits.load(Ordering::SeqCst),
            2,
            "a changed key must be tried"
        );
        assert!(
            svc.cty_key_rejected(other),
            "and the new key latches on its own 403"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    /// The same latch per account, over the log credentials rather than the
    /// server key. One operator's wrong app password must not get the whole
    /// host firewalled.
    #[test]
    fn a_403_on_a_users_log_stops_that_account_retrying() {
        let hits = Arc::new(AtomicUsize::new(0));
        let port = spawn_403_server(hits.clone());
        let (svc, dir, _) = service(port);

        let user_id = svc
            .db
            .create_user("VU2CPL", "Manoj", "hash", "admin")
            .expect("create user");
        let mut cfg = svc.db.clublog_config(user_id).unwrap();
        cfg.callsign = "VU2CPL".into();
        cfg.email = "someone@example.com".into();
        cfg.app_password = "wrong".into();
        svc.db.set_clublog_config(user_id, &cfg).unwrap();

        let first = svc.refresh_user(user_id).unwrap_err();
        assert!(
            first.contains("403"),
            "first attempt reports the 403: {first}"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        let second = svc.refresh_user(user_id).unwrap_err();
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "rejected credentials must not be sent again"
        );
        assert!(
            second.contains("app password"),
            "the refusal should point at what to change: {second}"
        );
        assert!(svc.user_credentials_rejected(user_id));

        // Fixing the password releases the latch on its own.
        cfg.app_password = "corrected".into();
        svc.db.set_clublog_config(user_id, &cfg).unwrap();
        assert!(!svc.user_credentials_rejected(user_id));
        let _ = svc.refresh_user(user_id);
        assert_eq!(hits.load(Ordering::SeqCst), 2, "the new password is tried");

        let _ = std::fs::remove_dir_all(dir);
    }
}

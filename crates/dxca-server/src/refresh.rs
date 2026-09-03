//! Background refresh of the two things that go stale: each user's ClubLog
//! log, and the shared LoTW users list.
//!
//! Until now both were manual-only — a button each. On a box that runs 24/7
//! that means the log stops moving the moment nobody presses anything, and
//! everything worked since keeps alerting as New DXCC. PLAN §5 always listed
//! a "refresh schedule" among the per-user ClubLog settings; this is it.
//!
//! Design notes, in the order they bit:
//!
//! * **Downloads are blocking.** `refresh_user` and `refresh_lotw` are
//!   synchronous `ureq` calls that can take a minute (ClubLog) or two
//!   (LoTW). They run on `spawn_blocking` so the async runtime — which is
//!   also carrying the spot pipeline — never stalls behind a slow server.
//!
//! * **At most one job per tick.** Several accounts coming due together
//!   would otherwise fire a burst of large downloads at ClubLog from one IP.
//!   One per tick spreads them over the interval instead.
//!
//! * **Attempts are recorded before the outcome is known**, and separately
//!   from successes. `matrices.last_refresh_unix` only advances on success,
//!   so a failing account would be "due" on every single tick and would hammer
//!   a third-party service. `RETRY_AFTER_SECS` is the floor between attempts
//!   whatever happens; the stamp is persisted, so a crash-loop cannot reset it
//!   either.
//!
//! * **No refresh on boot.** The due-check is purely time-based, so a restart
//!   refreshes only what was already overdue. Restarting the service is not a
//!   reason to re-download 56k QSOs.

use crate::users::UserService;
use std::sync::Arc;
use std::time::Duration;

/// How often to look for due work. Fine-grained relative to the intervals
/// themselves (hours, days) — this is a cheap SQLite read, not a download.
const TICK: Duration = Duration::from_secs(15 * 60);

/// Floor between two attempts for the same job, success or failure. Well
/// under the shortest sensible interval, far above a crash-loop's period.
const RETRY_AFTER_SECS: i64 = 60 * 60;

const LOTW_ATTEMPT_KEY: &str = "lotw_last_attempt_unix";
/// Written by `UserService::refresh_lotw` on success — so the manual button
/// and the scheduler share one clock — and read here to decide "due".
pub const LOTW_OK_KEY: &str = "lotw_last_refresh_unix";

const CTY_ATTEMPT_KEY: &str = "cty_last_attempt_unix";
/// Written by `UserService::refresh_cty` on success, same arrangement.
pub const CTY_OK_KEY: &str = "cty_last_refresh_unix";

const IOTA_ATTEMPT_KEY: &str = "iota_last_attempt_unix";
/// Written by `UserService::refresh_iota` on success, same arrangement.
pub const IOTA_OK_KEY: &str = "iota_last_refresh_unix";

const FCC_ATTEMPT_KEY: &str = "fcc_last_attempt_unix";
/// Written by `UserService::refresh_fcc` on success, same arrangement.
pub const FCC_OK_KEY: &str = "fcc_last_refresh_unix";

fn clublog_attempt_key(user_id: i64) -> String {
    format!("clublog_last_attempt_unix:{user_id}")
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs() as i64
}

/// The whole scheduling decision, as a pure function of four numbers — the
/// part worth testing, kept away from SQLite and the network.
///
/// `interval_secs <= 0` means the job is switched off. `last_ok` of 0 means
/// "never succeeded", which is due immediately — but the retry floor still
/// applies, so a never-succeeding job is attempted hourly, not every tick.
fn is_due(now: i64, last_ok: i64, last_attempt: i64, interval_secs: i64) -> bool {
    if interval_secs <= 0 {
        return false;
    }
    now - last_ok >= interval_secs && now - last_attempt >= RETRY_AFTER_SECS
}

/// Spawn the refresh loop. A `*_days` of 0 disables that shared job; each
/// user's `refresh_hours` of 0 disables theirs.
pub fn spawn(
    users: Arc<UserService>,
    cty_refresh_days: u64,
    lotw_refresh_days: u64,
    iota_refresh_days: u64,
    fcc_refresh_days: u64,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        // The first tick fires immediately; skip it so a restart doesn't
        // even *evaluate* work until the process has settled.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let users = users.clone();
            // One job per tick, shared resources before per-user ones: cty
            // and the LoTW list are read by every account, and cty in
            // particular gates classification — a user log rebuilt against a
            // stale resolver is worse than one rebuilt an hour later. The
            // award reference files rank after those two — nothing gates on
            // them — and before the per-user logs, being shared.
            let did_shared = tokio::task::spawn_blocking({
                let users = users.clone();
                move || {
                    run_cty_if_due(&users, cty_refresh_days)
                        || run_lotw_if_due(&users, lotw_refresh_days)
                        || run_iota_if_due(&users, iota_refresh_days)
                        || run_fcc_if_due(&users, fcc_refresh_days)
                }
            })
            .await
            .unwrap_or(false);
            if did_shared {
                continue;
            }
            let _ = tokio::task::spawn_blocking(move || run_one_clublog_if_due(&users)).await;
        }
    });
}

/// True when an IOTA directory download was attempted this tick.
fn run_iota_if_due(users: &UserService, days: u64) -> bool {
    let now = now_unix();
    if !is_due(
        now,
        users.db.meta_unix(IOTA_OK_KEY),
        users.db.meta_unix(IOTA_ATTEMPT_KEY),
        days as i64 * 86_400,
    ) {
        return false;
    }
    let _ = users.db.meta_set_now(IOTA_ATTEMPT_KEY);
    match users.refresh_iota() {
        Ok(count) => println!("dxca: auto-refresh: IOTA directory updated, {count} groups"),
        Err(e) => eprintln!("dxca: auto-refresh: IOTA failed: {e}"),
    }
    true
}

/// True when an FCC download was attempted this tick. Runs only after an
/// admin has pulled the table once by hand: the ~200 MB download must
/// never be a surprise a config default springs on a Pi.
fn run_fcc_if_due(users: &UserService, days: u64) -> bool {
    if users.fcc_count() == 0 {
        return false;
    }
    let now = now_unix();
    if !is_due(
        now,
        users.db.meta_unix(FCC_OK_KEY),
        users.db.meta_unix(FCC_ATTEMPT_KEY),
        days as i64 * 86_400,
    ) {
        return false;
    }
    let _ = users.db.meta_set_now(FCC_ATTEMPT_KEY);
    match users.refresh_fcc() {
        Ok(count) => println!("dxca: auto-refresh: FCC state table updated, {count} calls"),
        Err(e) => eprintln!("dxca: auto-refresh: FCC failed: {e}"),
    }
    true
}

/// True when a cty.xml download was attempted this tick.
fn run_cty_if_due(users: &UserService, days: u64) -> bool {
    let key = crate::builtin::effective_clublog_api_key(&users.db);
    if key.is_empty() {
        // No admin key and no built-in one (a build made without a key).
        // Nothing to do, and nothing to complain about every 15 minutes —
        // a server with no key simply keeps the cty.xml it was given.
        return false;
    }
    let now = now_unix();
    if !is_due(
        now,
        users.db.meta_unix(CTY_OK_KEY),
        users.db.meta_unix(CTY_ATTEMPT_KEY),
        days as i64 * 86_400,
    ) {
        return false;
    }
    let _ = users.db.meta_set_now(CTY_ATTEMPT_KEY);
    match users.refresh_cty(&key) {
        Ok(entities) => println!("dxca: auto-refresh: cty.xml updated, {entities} entities"),
        Err(e) => eprintln!("dxca: auto-refresh: cty.xml failed: {e}"),
    }
    true
}

/// True when a LoTW download was attempted this tick.
fn run_lotw_if_due(users: &UserService, days: u64) -> bool {
    let now = now_unix();
    if !is_due(
        now,
        users.db.meta_unix(LOTW_OK_KEY),
        users.db.meta_unix(LOTW_ATTEMPT_KEY),
        days as i64 * 86_400,
    ) {
        return false;
    }
    let _ = users.db.meta_set_now(LOTW_ATTEMPT_KEY);
    match users.refresh_lotw(dxca_connect::lotw::DEFAULT_URL) {
        // The success stamp is written inside refresh_lotw, shared with the
        // manual button.
        Ok(count) => println!("dxca: auto-refresh: LoTW list updated, {count} users"),
        // Logged, never fatal: a failed refresh leaves the previous list in
        // place and the retry floor decides when to try again.
        Err(e) => eprintln!("dxca: auto-refresh: LoTW failed: {e}"),
    }
    true
}

/// Refresh the single most-overdue account whose interval has elapsed.
fn run_one_clublog_if_due(users: &UserService) {
    let now = now_unix();
    let Ok(all) = users.db.users() else { return };

    let mut candidates: Vec<(i64, i64)> = Vec::new(); // (user_id, last_success)
    for u in all {
        let Ok(cfg) = users.db.clublog_config(u.id) else {
            continue;
        };
        // Not set up far enough to download anything. (`refresh_hours <= 0`
        // is handled by is_due, which reads it as "switched off".)
        if cfg.callsign.is_empty() || cfg.email.is_empty() || cfg.app_password.is_empty() {
            continue;
        }
        // No matrix yet means never refreshed — due immediately.
        let last_ok = users
            .db
            .matrix_meta(u.id)
            .ok()
            .flatten()
            .map(|(_, ts)| ts)
            .unwrap_or(0);
        if !is_due(
            now,
            last_ok,
            users.db.meta_unix(&clublog_attempt_key(u.id)),
            cfg.refresh_hours.saturating_mul(3_600),
        ) {
            continue;
        }
        candidates.push((u.id, last_ok));
    }

    // Longest-stale first, so a queue of due accounts drains fairly rather
    // than by whatever order the users table happens to return.
    candidates.sort_by_key(|(_, last_ok)| *last_ok);
    let Some((user_id, _)) = candidates.first().copied() else {
        return;
    };

    let _ = users.db.meta_set_now(&clublog_attempt_key(user_id));
    match users.refresh_user(user_id) {
        Ok((qsos, dxccs)) => {
            println!("dxca: auto-refresh: user {user_id} log updated, {qsos} QSOs, {dxccs} DXCC")
        }
        Err(e) => eprintln!("dxca: auto-refresh: user {user_id} failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600;
    const DAY: i64 = 86_400;
    const NOW: i64 = 1_800_000_000;

    #[test]
    fn zero_interval_is_off() {
        // Never due, however stale — this is how a user picks "manual only"
        // and how lotw_refresh_days = 0 disables the shared job.
        assert!(!is_due(NOW, 0, 0, 0));
        assert!(!is_due(NOW, NOW - 365 * DAY, 0, 0));
        assert!(!is_due(NOW, 0, 0, -1));
    }

    #[test]
    fn due_only_after_the_interval_elapses() {
        let day_old = NOW - DAY;
        assert!(is_due(NOW, day_old, 0, 24 * HOUR), "24h old, 24h interval");
        assert!(!is_due(NOW, NOW - 23 * HOUR, 0, 24 * HOUR), "23h < 24h");
        // Boundary: exactly the interval counts as due.
        assert!(is_due(NOW, NOW - 24 * HOUR, 0, 24 * HOUR));
    }

    #[test]
    fn never_refreshed_is_due_immediately() {
        // last_ok = 0 (no matrix row at all) — a new account should pull its
        // log on the first tick rather than waiting a full day.
        assert!(is_due(NOW, 0, 0, 24 * HOUR));
    }

    #[test]
    fn retry_floor_stops_a_failing_job_hammering() {
        // Overdue AND failing: last_ok stays old because nothing succeeded,
        // so without the attempt floor this would fire on every 15-min tick.
        let overdue = NOW - 10 * DAY;
        assert!(!is_due(NOW, overdue, NOW - 60, 24 * HOUR), "attempt 1m ago");
        assert!(
            !is_due(NOW, overdue, NOW - 59 * 60, 24 * HOUR),
            "attempt 59m ago — still inside the floor"
        );
        assert!(
            is_due(NOW, overdue, NOW - HOUR, 24 * HOUR),
            "an hour later it may try again"
        );
    }

    #[test]
    fn a_fresh_success_defers_the_next_run() {
        // The success path: last_ok advances, so the job goes quiet for a
        // full interval even though it just attempted.
        assert!(!is_due(NOW, NOW, NOW, 24 * HOUR));
    }

    #[test]
    fn lotw_weekly_default() {
        assert!(is_due(NOW, NOW - 7 * DAY, 0, 7 * DAY));
        assert!(!is_due(NOW, NOW - 6 * DAY, 0, 7 * DAY));
    }
}

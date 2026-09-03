//! The HTTP API (plan §7). Session-cookie auth; admin role gates user
//! management; per-user config and classification for the authenticated
//! account. The embedded web UI is served by the fallback.

use crate::auth;
use crate::config::{BroadcastDestination, ClusterNode, Config, UdpSource};
use crate::db::{ClubLogUserConfig, NotifyUserConfig, StationConfig, User};
use crate::nodes::NodeManager;
use crate::pipeline::{PipelineInput, PipelineState};
use crate::users::UserService;
use dxca_connect::mqtt::MqttDestinationConfig;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub pipeline: Arc<PipelineState>,
    pub nodes: Arc<NodeManager>,
    pub users: Arc<UserService>,
    /// The live global config (M5 web editing) + where it persists.
    pub config: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    /// Pipeline input — hot-applied sources/nodes feed into it.
    pub input_tx: mpsc::Sender<PipelineInput>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/status", get(status))
        .route("/api/spots", get(spots))
        .route("/api/spot-stats", get(spot_stats))
        .route("/api/stream", get(stream))
        .route("/api/setup", post(setup))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/me/station", get(station))
        .route("/api/reference", get(reference))
        .route("/api/config/me/clublog", get(get_clublog).put(put_clublog))
        .route(
            "/api/config/me/notifications",
            get(get_notify).put(put_notify),
        )
        .route("/api/config/me/station", get(get_station).put(put_station))
        .route("/api/me/sun", get(sun))
        .route("/api/clublog/refresh", post(refresh))
        .route("/api/config/global", get(get_global).put(put_global))
        .route("/api/telegram/test", post(telegram_test))
        .route("/api/lotw/refresh", post(lotw_refresh))
        .route("/api/iota/refresh", post(iota_refresh))
        .route("/api/fcc/refresh", post(fcc_refresh))
        .route("/api/cty/refresh", post(cty_refresh))
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/{id}", patch(update_user).delete(delete_user))
        .route("/api/me/alerts", get(my_alerts))
        .route("/api/mqtt", get(get_mqtt).put(put_mqtt))
        .route("/api/blacklist", get(list_blacklist).post(add_blacklist))
        .route(
            "/api/blacklist/{callsign}",
            axum::routing::delete(del_blacklist),
        )
        .with_state(state)
        .fallback(crate::assets::serve)
}

fn err(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, Json(serde_json::json!({ "error": msg.into() }))).into_response()
}

fn unauthorized() -> Response {
    err(StatusCode::UNAUTHORIZED, "not logged in")
}

// A Response-typed Err is the ergonomic axum idiom; the size is irrelevant
// on these cold auth-failure paths.
#[allow(clippy::result_large_err)]
fn require_user(app: &AppState, headers: &HeaderMap) -> Result<User, Response> {
    auth::user_from_headers(&app.users.db, headers).ok_or_else(unauthorized)
}

#[allow(clippy::result_large_err)]
fn require_admin(app: &AppState, headers: &HeaderMap) -> Result<User, Response> {
    let user = require_user(app, headers)?;
    if !user.is_admin() {
        return Err(err(StatusCode::FORBIDDEN, "admin only"));
    }
    Ok(user)
}

// --- status + spots ------------------------------------------------------

fn status_json(app: &AppState) -> serde_json::Value {
    let counters = app.pipeline.broadcaster().counters();
    let user_count = app.users.db.user_count().unwrap_or(0);
    serde_json::json!({
        "name": "dxca",
        "version": env!("CARGO_PKG_VERSION"),
        "setup_required": user_count == 0,
        "users": user_count,
        "cty_loaded": app.users.resolver_loaded(),
        "cty_entities": app.users.entity_count(),
        "lotw_users": app.users.lotw_count(),
        "iota_groups": app.users.iota_count(),
        "fcc_calls": app.users.fcc_count(),
        "telnet_clients": app.pipeline.telnet.client_count(),
        "spots_per_source": *app.pipeline.source_counts.lock().unwrap(),
        "cluster_nodes": app.nodes.statuses(),
        "udp_sent": counters.total_sent(),
        "udp_failed": counters.total_failed(),
    })
}

async fn status(State(app): State<AppState>) -> Json<serde_json::Value> {
    Json(status_json(&app))
}

#[derive(Deserialize)]
struct SpotsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    200
}

/// One spot as the UI sees it: the raw fields plus the extracted DX call,
/// the LoTW marker, and — when a session is present — that user's
/// classification (alert level, DXCC name, band): plan §5's per-user
/// highlighting.
fn annotate_spot(
    app: &AppState,
    user: Option<&User>,
    s: &dxca_core::Spot,
    sun: Option<dxca_core::solar::SunPhase>,
) -> serde_json::Value {
    let mut v = serde_json::to_value(s).expect("spot serializes");
    let dx_call = s.dx_callsign();
    v["dx_call"] = serde_json::to_value(&dx_call).unwrap();
    v["is_lotw"] = serde_json::Value::Bool(dx_call.is_some_and(|c| app.users.is_lotw_user(&c)));

    // The band is a property of the SPOT — a frequency and a band plan —
    // not of anyone's log. It used to be published only as part of a
    // classification, which meant an account with no ClubLog log loaded saw
    // an empty Band column and, worse, no band mask: `classify` returns
    // None without a log matrix, so the mask's own precondition silently
    // became "has a ClubLog log" instead of "has a locator". Derived here
    // from the frequency, exactly as the classifier derives it.
    let band = dxca_core::bands::band_from_mhz(s.frequency_mhz());
    v["band"] = serde_json::to_value(band).unwrap();

    // Phase-rotation mask (docs/PHASE-ROTATION-MASK.md): is this band
    // plausibly workable from the operator's QTH at this moment?
    //
    // Present only when they have set a locator, and it is advice, not an
    // instruction — the server never withholds a spot on this basis.
    // Whether anything is dimmed or hidden is the client's decision, so the
    // mask stays opt-in and off by default.
    if let (Some(phase), Some(b)) = (sun, band) {
        v["band_open"] = serde_json::Value::Bool(dxca_core::bands::plausible_in(b, phase));
    }

    if let Some(u) = user
        && let Some(c) = app.users.classify(u.id, s)
    {
        v["alert"] = serde_json::to_value(c.level).unwrap();
        v["dxcc_name"] = serde_json::to_value(&c.dxcc_name).unwrap();
        v["is_beacon"] = serde_json::Value::Bool(c.is_beacon);
        // The award key an award-level pick fired on ("MK83", "OH",
        // "AS-153") — what the row's tooltip names as the catch.
        if let Some(r) = &c.award_ref {
            v["award_ref"] = serde_json::to_value(r).unwrap();
        }
    }
    v
}

async fn spots(
    State(app): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SpotsQuery>,
) -> Json<serde_json::Value> {
    let user = auth::user_from_headers(&app.users.db, &headers);
    // Once per request, not once per spot.
    let sun = user.as_ref().and_then(|u| app.users.sun_phase(u.id));
    let spots = app.pipeline.recent_spots(q.limit.min(2000));
    let annotated: Vec<serde_json::Value> = spots
        .iter()
        .map(|s| annotate_spot(&app, user.as_ref(), s, sun))
        .collect();
    Json(serde_json::json!({ "spots": annotated }))
}

/// What the spot feed is actually made of: totals by band, by mode class,
/// and by the source that carried it.
///
/// Aggregated **server-side over the whole ring** rather than in the
/// browser. The Spots screen holds the last 500 spots, which on a busy feed
/// is about five minutes — counting those would answer a much smaller
/// question than the one being asked, and would change every time the page
/// reloaded. The ring is 5000, closer to an hour.
///
/// Band comes from the frequency and mode from the spot itself, so none of
/// this depends on which account is asking; it is the same feed for
/// everyone and needs no session.
async fn spot_stats(State(app): State<AppState>) -> Json<serde_json::Value> {
    use std::collections::HashMap;
    let spots = app.pipeline.recent_spots(usize::MAX);

    let mut by_band: HashMap<&'static str, u64> = HashMap::new();
    let mut by_mode: HashMap<String, u64> = HashMap::new();
    let mut by_source: HashMap<String, u64> = HashMap::new();
    let (mut oldest, mut newest) = (i64::MAX, i64::MIN);

    for s in &spots {
        if let Some(b) = dxca_core::bands::band_from_hz(s.frequency_hz()) {
            *by_band.entry(b).or_default() += 1;
        }
        // The mode as reported, not the award bucket: "which bands and modes
        // is my feed actually carrying" is a question about FT8 versus FT4,
        // and collapsing both into DATA would erase the answer.
        let mode = s.mode.trim().to_uppercase();
        if !mode.is_empty() {
            *by_mode.entry(mode).or_default() += 1;
        }
        *by_source.entry(s.source_name.clone()).or_default() += 1;
        oldest = oldest.min(s.time_unix);
        newest = newest.max(s.time_unix);
    }

    // Bands in band order, not alphabetical or by count: an operator reads
    // this as a band plan, and 10M sorting before 160M would be noise.
    let bands: Vec<serde_json::Value> = dxca_core::bands::SELECTABLE_BANDS
        .iter()
        .filter_map(|b| {
            by_band
                .get(b)
                .map(|n| serde_json::json!({"key": b, "count": n}))
        })
        .collect();

    let sorted = |m: HashMap<String, u64>| {
        let mut v: Vec<(String, u64)> = m.into_iter().collect();
        // Commonest first, ties by name so the order never jitters between
        // refreshes when two sources are level.
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.into_iter()
            .map(|(k, n)| serde_json::json!({"key": k, "count": n}))
            .collect::<Vec<_>>()
    };

    Json(serde_json::json!({
        "total": spots.len(),
        "span_secs": if spots.is_empty() { 0 } else { newest - oldest },
        "bands": bands,
        "modes": sorted(by_mode),
        "sources": sorted(by_source),
    }))
}

// --- live stream ---------------------------------------------------------

/// WebSocket: every processed spot as an annotated frame for THIS session's
/// user, plus a status frame every 5 s.
async fn stream(
    State(app): State<AppState>,
    headers: HeaderMap,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Response {
    let user = auth::user_from_headers(&app.users.db, &headers);
    ws.on_upgrade(move |socket| stream_socket(socket, app, user))
}

async fn stream_socket(
    mut socket: axum::extract::ws::WebSocket,
    app: AppState,
    user: Option<User>,
) {
    use axum::extract::ws::Message as WsMessage;
    let mut spot_rx = app.pipeline.spot_events.subscribe();
    let mut status_tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            spot = spot_rx.recv() => match spot {
                Ok(spot) => {
                    let frame = serde_json::json!({
                        "type": "spot",
                        // Recomputed per frame rather than per connection:
                        // a WebSocket can stay open for hours, across a
                        // sunset, and a stale elevation would mask the
                        // wrong bands for the rest of the session.
                        "spot": annotate_spot(
                            &app,
                            user.as_ref(),
                            &spot,
                            user.as_ref().and_then(|u| app.users.sun_phase(u.id)),
                        ),
                    });
                    if socket.send(WsMessage::Text(frame.to_string().into())).await.is_err() {
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },
            _ = status_tick.tick() => {
                let frame = serde_json::json!({ "type": "status", "status": status_json(&app) });
                if socket.send(WsMessage::Text(frame.to_string().into())).await.is_err() {
                    return;
                }
            }
            msg = socket.recv() => match msg {
                None | Some(Err(_)) => return,
                Some(Ok(WsMessage::Close(_))) => return,
                Some(Ok(_)) => continue, // client input ignored
            },
        }
    }
}

// --- auth ----------------------------------------------------------------

#[derive(Deserialize)]
struct Credentials {
    callsign: String,
    password: String,
    #[serde(default)]
    display_name: String,
}

/// First-run bootstrap: creates the admin account. Refused once any user
/// exists (no default credentials, ever).
async fn setup(State(app): State<AppState>, Json(req): Json<Credentials>) -> Response {
    match app.users.db.user_count() {
        Ok(0) => {}
        Ok(_) => return err(StatusCode::FORBIDDEN, "setup already done"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
    create_account(&app, &req, "admin", true).await
}

/// Admin creates further accounts (role "user" unless "admin" requested).
async fn create_user(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUserReq>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let role = if req.role == "admin" { "admin" } else { "user" };
    // No session cookie here — creating an account for someone else must
    // not switch the admin's own session.
    create_account(
        &app,
        &Credentials {
            callsign: req.callsign,
            password: req.password,
            display_name: req.display_name,
        },
        role,
        false,
    )
    .await
}

#[derive(Deserialize)]
struct CreateUserReq {
    callsign: String,
    password: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    role: String,
}

/// Every field optional — an absent one is left alone, so the UI can send
/// just the password or just the role without restating the whole account.
#[derive(Deserialize)]
struct UpdateUserReq {
    #[serde(default)]
    callsign: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

/// Admin edits an account: callsign, display name, role, password — any
/// subset. Deliberately reachable for every account including the caller's
/// own, because an admin renaming or re-passwording themselves is ordinary.
async fn update_user(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserReq>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let target = match app.users.db.user_by_id(id) {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::NOT_FOUND, "no such user"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    // Role must be one of the two the rest of the code understands; a typo'd
    // "Admin" would silently create an account that is neither.
    let role = match req.role.as_deref().map(str::trim) {
        None => None,
        Some("admin") => Some("admin"),
        Some("user") => Some("user"),
        Some(other) => {
            return err(
                StatusCode::BAD_REQUEST,
                format!("role must be 'user' or 'admin', not '{other}'"),
            );
        }
    };

    // Demoting the last admin is unrecoverable in a way deleting is not:
    // /api/setup only re-arms at zero accounts, so a system left with users
    // and no admin has no way back through the UI at all. Refuse it whatever
    // the user count, and say what to do instead.
    if target.is_admin() && role == Some("user") {
        match app.users.db.admin_count() {
            Ok(1) => {
                return err(
                    StatusCode::CONFLICT,
                    "this is the only admin — promote another account first, \
                     otherwise nobody could administer the server",
                );
            }
            Ok(_) => {}
            Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
        }
    }

    // Reject a rename onto a callsign someone else holds before writing
    // anything, so the operator gets "already taken" rather than a raw
    // UNIQUE-constraint string from SQLite.
    let callsign = match req.callsign.as_deref().map(str::trim) {
        Some("") => return err(StatusCode::BAD_REQUEST, "callsign cannot be empty"),
        Some(c) => {
            match app.users.db.user_by_callsign(c) {
                Ok(Some((other, _))) if other.id != id => {
                    return err(
                        StatusCode::CONFLICT,
                        format!("{} already exists", other.callsign),
                    );
                }
                Ok(_) => {}
                Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
            }
            Some(c)
        }
        None => None,
    };

    // Same 6-char floor as account creation — one rule, not two.
    if let Some(pw) = &req.password {
        if pw.len() < 6 {
            return err(StatusCode::BAD_REQUEST, "password ≥ 6 chars");
        }
        let hash = match auth::hash_password(pw) {
            Ok(h) => h,
            Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
        };
        if let Err(e) = app.users.db.set_pass_hash(id, &hash) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, e);
        }
    }

    if let Err(e) = app
        .users
        .db
        .update_user(id, callsign, req.display_name.as_deref(), role)
    {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e);
    }

    match app.users.db.user_by_id(id) {
        Ok(Some(u)) => Json(serde_json::json!({ "user": u })).into_response(),
        Ok(None) => err(StatusCode::NOT_FOUND, "no such user"),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Admin deletes an account, including their own and including the very
/// last one — deleting down to zero is allowed and re-arms the first-run
/// setup card, which is the intended way to start a server over.
///
/// The single refusal is removing the last admin while other accounts
/// remain: that leaves users who cannot be administered and a `/api/setup`
/// that stays closed because the account count is not zero.
async fn delete_user(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let target = match app.users.db.user_by_id(id) {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::NOT_FOUND, "no such user"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    if target.is_admin() {
        let admins = match app.users.db.admin_count() {
            Ok(n) => n,
            Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
        };
        let total = match app.users.db.user_count() {
            Ok(n) => n,
            Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
        };
        if admins == 1 && total > 1 {
            return err(
                StatusCode::CONFLICT,
                "this is the only admin and other accounts remain — promote \
                 another admin first, or delete the other accounts before \
                 this one",
            );
        }
    }

    match app.users.db.delete_user(id) {
        // Sessions cascade with the row, so deleting yourself logs you out
        // on the next request; the UI reloads into the login card.
        Ok(true) => Json(serde_json::json!({
            "deleted": target.callsign,
            "remaining": app.users.db.user_count().unwrap_or(0),
        }))
        .into_response(),
        Ok(false) => err(StatusCode::NOT_FOUND, "no such user"),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn create_account(
    app: &AppState,
    req: &Credentials,
    role: &str,
    with_session: bool,
) -> Response {
    if req.callsign.trim().is_empty() || req.password.len() < 6 {
        return err(
            StatusCode::BAD_REQUEST,
            "callsign required, password ≥ 6 chars",
        );
    }
    let hash = match auth::hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let id = match app
        .users
        .db
        .create_user(req.callsign.trim(), &req.display_name, &hash, role)
    {
        Ok(id) => id,
        Err(e) => return err(StatusCode::CONFLICT, e),
    };
    let body = Json(serde_json::json!({
        "id": id, "callsign": req.callsign.trim().to_uppercase(), "role": role,
    }));
    if !with_session {
        return body.into_response();
    }
    match auth::start_session(&app.users.db, id) {
        Ok(cookie) => ([(header::SET_COOKIE, cookie)], body).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn login(State(app): State<AppState>, Json(req): Json<Credentials>) -> Response {
    let found = match app.users.db.user_by_callsign(&req.callsign) {
        Ok(f) => f,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let Some((user, stored_hash)) = found else {
        return err(StatusCode::UNAUTHORIZED, "bad callsign or password");
    };
    if !auth::verify_password(&req.password, &stored_hash) {
        return err(StatusCode::UNAUTHORIZED, "bad callsign or password");
    }
    match auth::start_session(&app.users.db, user.id) {
        Ok(cookie) => (
            [(header::SET_COOKIE, cookie)],
            Json(serde_json::json!(user)),
        )
            .into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn logout(State(app): State<AppState>, headers: HeaderMap) -> Response {
    auth::end_session(&app.users.db, &headers);
    (
        [(header::SET_COOKIE, auth::clear_cookie())],
        Json(serde_json::json!({"ok": true})),
    )
        .into_response()
}

async fn me(State(app): State<AppState>, headers: HeaderMap) -> Response {
    match require_user(&app, &headers) {
        Ok(user) => Json(serde_json::json!(user)).into_response(),
        Err(resp) => resp,
    }
}

/// The Spots screen's station card: who is logged in, the callsign their log
/// is for, and the award totals. `stats` is null until they refresh ClubLog
/// — the card then says "no log loaded" instead of showing four zeroes,
/// which would read as a station that has worked nothing.
async fn station(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let cl = app.users.db.clublog_config(user.id).ok();
    let meta = app.users.db.matrix_meta(user.id).ok().flatten();
    Json(serde_json::json!({
        "callsign": user.callsign,
        "display_name": user.display_name,
        // The log's own callsign may differ from the login (a /P or club
        // log), so the card names the one the matrix was built from.
        "log_callsign": cl.as_ref().map(|c| c.callsign.clone()).filter(|c| !c.is_empty()),
        "stats": app.users.stats(user.id),
        // The same totals with the ARRL deleted entities left out. Both are
        // sent so the "current only" tickbox is instant — the numbers are a
        // dozen integers, and a round trip per toggle would be worse than
        // the bytes.
        "stats_current": app.users.stats_current(user.id),
        // The per-band / per-mode breakdown behind the My ClubLog statistics
        // card. Sliced from the same in-memory matrix, so it costs a walk of
        // the entity map and no extra storage.
        "by_band_mode": app.users.band_mode_stats(user.id),
        "by_band_mode_current": app.users.band_mode_stats_current(user.id),
        // The non-DXCC award totals (docs/AWARDS.md phases 2–4). No
        // "_current" twin: the deleted-entities list is a DXCC concept.
        "award_stats": app.users.award_stats(user.id),
        "qso_count": meta.map(|m| m.0),
        "last_refresh_unix": meta.map(|m| m.1),
    }))
    .into_response()
}

/// The vocabularies the UI builds its filter controls from — served rather
/// than hardcoded in Svelte so the band list, the mode buckets and the level
/// ladder cannot drift from what the classifier actually emits.
/// The level ladder as the UI receives it. Pulled out of the handler so
/// it can be tested without booting the server — the `notifyField` half is
/// a contract the Alerts tab now binds to directly, and a level that
/// reaches the ladder without one is a control that cannot say no.
fn level_vocabulary() -> Vec<serde_json::Value> {
    dxca_core::classify::AlertLevel::FLAGGABLE
        .iter()
        // `award` is what lets the UI hide an unchased award's levels — the
        // classic eight carry null and are always shown. `notifyField` is
        // the Telegram gate's field name: served, not retyped in Svelte,
        // because the hand-kept copy there missed WAZ and the Marathon
        // entirely and left three rows that could not be switched off.
        .map(|l| {
            serde_json::json!({
                "key": l.key(),
                "label": l.label(),
                "award": l.award(),
                "notifyField": NotifyUserConfig::notify_field(*l),
            })
        })
        .collect()
}

async fn reference() -> Response {
    let levels = level_vocabulary();
    Json(serde_json::json!({
        "bands": dxca_core::bands::SELECTABLE_BANDS,
        "modes": dxca_core::modes::CLASSES,
        "levels": levels,
        // The WAS scopes, served like every other vocabulary so the picker
        // cannot drift from what the classifier understands.
        "was_scopes": [
            { "key": "mixed", "label": "Mixed" },
            { "key": "triple", "label": "Triple Play" },
            { "key": "band", "label": "Per band" },
        ],
    }))
    .into_response()
}

async fn list_users(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    match app.users.db.users() {
        Ok(users) => Json(serde_json::json!({ "users": users })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// This account's Telegram alert history, newest first. Per-user by
/// construction — a session only ever sees its own.
async fn my_alerts(
    State(app): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SpotsQuery>,
) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match app.users.db.sent_alerts(user.id, q.limit.min(500)) {
        Ok(alerts) => Json(serde_json::json!({ "alerts": alerts })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

// --- MQTT destinations ---------------------------------------------------
//
// Admin-gated, stored in the database (0600) rather than config/dxca.toml
// (0644) because of the broker password. `PUT` replaces the whole list —
// the same shape as the global config editor, and simpler than per-row
// verbs for a list this short.

/// Turn stored rows into live publisher configs, skipping disabled ones.
fn mqtt_live_configs(dests: &[crate::db::MqttDestination]) -> Vec<MqttDestinationConfig> {
    dests
        .iter()
        .filter(|d| d.enabled && !d.host.trim().is_empty())
        .map(|d| MqttDestinationConfig {
            name: d.name.clone(),
            host: d.host.clone(),
            port: d.port,
            username: d.username.clone(),
            password: d.password.clone(),
            topic: d.topic.clone(),
            client_id: d.client_id.clone(),
            allowed_sources: d.sources.iter().cloned().collect(),
            unfiltered: d.unfiltered,
        })
        .collect()
}

/// Reconnect the pipeline's publishers from the stored list.
pub fn load_mqtt(app: &AppState) -> Result<usize, String> {
    let dests = app.users.db.mqtt_destinations()?;
    let live = mqtt_live_configs(&dests);
    let n = live.len();
    app.pipeline.apply_mqtt(live);
    Ok(n)
}

async fn get_mqtt(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    match app.users.db.mqtt_destinations() {
        Ok(dests) => {
            let counters = app.pipeline.mqtt().counters();
            Json(serde_json::json!({
                "destinations": dests,
                "sent": counters.total_sent(),
                "failed": counters.total_failed(),
                "connected": counters.configured,
            }))
            .into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[derive(Deserialize)]
struct MqttReq {
    destinations: Vec<crate::db::MqttDestination>,
}

async fn put_mqtt(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<MqttReq>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    // Validate before storing anything: a half-applied list would leave the
    // running publishers disagreeing with the database.
    let mut seen = std::collections::HashSet::new();
    for d in &req.destinations {
        if d.name.trim().is_empty() {
            return err(StatusCode::BAD_REQUEST, "every destination needs a name");
        }
        if !seen.insert(d.name.trim().to_lowercase()) {
            return err(
                StatusCode::BAD_REQUEST,
                format!("duplicate destination name '{}'", d.name),
            );
        }
        if d.enabled && d.host.trim().is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                format!("'{}' is enabled but has no broker host", d.name),
            );
        }
        if d.topic.trim().is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                format!("'{}' needs a base topic", d.name),
            );
        }
    }
    if let Err(e) = app.users.db.set_mqtt_destinations(&req.destinations) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e);
    }
    match load_mqtt(&app) {
        Ok(n) => Json(serde_json::json!({
            "destinations": req.destinations, "connected": n,
        }))
        .into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

// --- blacklist -----------------------------------------------------------
//
// Server-wide and admin-gated. Every write refreshes the pipeline's live
// set as well as the database, so an edit takes effect on the next spot
// without a restart — the same hot-apply contract as sources and nodes.

/// Push the stored list into the running pipeline. Called on every write and
/// once at startup; the database stays the source of truth.
fn refresh_blacklist(app: &AppState) -> Result<Vec<String>, String> {
    let calls = app.users.db.blacklist()?;
    app.pipeline.apply_blacklist(calls.clone());
    Ok(calls)
}

async fn list_blacklist(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    match app.users.db.blacklist() {
        Ok(calls) => Json(serde_json::json!({ "calls": calls })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[derive(Deserialize)]
struct BlacklistReq {
    callsign: String,
}

async fn add_blacklist(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BlacklistReq>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let call = req.callsign.trim().to_uppercase();
    if call.is_empty() {
        return err(StatusCode::BAD_REQUEST, "callsign required");
    }
    let added = match app.users.db.blacklist_add(&call) {
        Ok(a) => a,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    match refresh_blacklist(&app) {
        Ok(calls) => Json(serde_json::json!({
            "callsign": call, "added": added, "calls": calls,
        }))
        .into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn del_blacklist(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(callsign): Path<String>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    match app.users.db.blacklist_remove(&callsign) {
        Ok(false) => err(StatusCode::NOT_FOUND, "not listed"),
        Ok(true) => match refresh_blacklist(&app) {
            Ok(calls) => Json(serde_json::json!({
                "removed": callsign.trim().to_uppercase(), "calls": calls,
            }))
            .into_response(),
            Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        },
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Load the stored blacklist into the pipeline at startup.
pub fn load_blacklist(app: &AppState) -> Result<usize, String> {
    refresh_blacklist(app).map(|c| c.len())
}

// --- per-user config -----------------------------------------------------

async fn get_clublog(State(app): State<AppState>, headers: HeaderMap) -> Response {
    with_user_config(&app, &headers, |db, uid| db.clublog_config(uid))
}

async fn put_clublog(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(cfg): Json<ClubLogUserConfig>,
) -> Response {
    match require_user(&app, &headers) {
        Ok(user) => match app.users.db.set_clublog_config(user.id, &cfg) {
            Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
            Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        },
        Err(resp) => resp,
    }
}

async fn get_notify(State(app): State<AppState>, headers: HeaderMap) -> Response {
    with_user_config(&app, &headers, |db, uid| db.notify_config(uid))
}

async fn put_notify(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(cfg): Json<NotifyUserConfig>,
) -> Response {
    // Empty is allowed and normalised to `all` on write — that is what an
    // older client, which knows nothing of this field, will send. Anything
    // else unrecognised is refused rather than quietly treated as `all`: a
    // typo that silently widens who wakes you is worse than an error.
    use crate::db::{SPOTTER_ALL, SPOTTER_HUMAN, SPOTTER_SKIMMER};
    let kind = cfg.notify_spotter_kind.as_str();
    if !matches!(kind, "" | SPOTTER_ALL | SPOTTER_HUMAN | SPOTTER_SKIMMER) {
        return err(
            StatusCode::BAD_REQUEST,
            "spotter kind must be all, human or skimmer",
        );
    }
    match require_user(&app, &headers) {
        Ok(user) => match app.users.db.set_notify_config(user.id, &cfg) {
            Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
            Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        },
        Err(resp) => resp,
    }
}

async fn get_station(State(app): State<AppState>, headers: HeaderMap) -> Response {
    with_user_config(&app, &headers, |db, uid| db.station_config(uid))
}

/// Where the sun is for this account right now: the phase, and the sunrise
/// and sunset it was derived from.
///
/// 204 rather than an error when there is no locator — "nothing to say" is
/// the normal state for most accounts, not a failure. The UI reads that as
/// "no band mask available" and shows no control at all.
async fn sun(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match app.users.sun_state(user.id) {
        Some(v) => Json(v).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn put_station(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(mut cfg): Json<StationConfig>,
) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    // Validate here rather than letting a typo silently disable the mask:
    // an operator who sets a locator and sees nothing happen has no way to
    // tell a rejected value from a feature that is not working.
    cfg.locator = cfg.locator.trim().to_uppercase();
    if !cfg.locator.is_empty() && dxca_core::grid::parse(&cfg.locator).is_none() {
        return err(
            StatusCode::BAD_REQUEST,
            "not a Maidenhead locator — expected 4 or 6 characters like MK82 or JN58TD",
        );
    }
    // Bounded rather than free. Below 5 minutes the grey line is too narrow
    // to be a phase at all; at the top end it stops being a grey line and
    // starts being "most of the day", at which point the mask says nothing.
    //
    // The ceiling was 180 and is now 360 (2026-08-29, VU2CPL). Three hours is
    // a fair description of the enhancement at mid latitudes, but it is not the whole
    // story: on the low bands a high-latitude path can stay open for hours
    // either side of the terminator, and near the solstices the sun crosses
    // the horizon so obliquely that the transition genuinely lasts that long.
    // An operator chasing 160m to Scandinavia in December is not misusing the
    // control by asking for six hours.
    //
    // Refused rather than clamped: silently changing a number the operator
    // typed is how they end up not trusting the screen.
    if !(5..=360).contains(&cfg.greyline_window_min) {
        return err(
            StatusCode::BAD_REQUEST,
            "greyline window must be between 5 and 360 minutes",
        );
    }
    match app.users.db.set_station_config(user.id, &cfg) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

fn with_user_config<T: serde::Serialize>(
    app: &AppState,
    headers: &HeaderMap,
    read: impl Fn(&crate::db::Db, i64) -> Result<T, String>,
) -> Response {
    match require_user(app, headers) {
        Ok(user) => match read(&app.users.db, user.id) {
            Ok(cfg) => Json(serde_json::json!(cfg)).into_response(),
            Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        },
        Err(resp) => resp,
    }
}

// --- global config (M5 web editing, admin) -------------------------------

/// The editable arrays plus the file-only scalars (shown read-only).
async fn get_global(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let cfg = app.config.lock().unwrap().clone();
    Json(serde_json::json!({
        "udp_sources": cfg.udp_sources,
        "cluster_nodes": cfg.cluster_nodes,
        "broadcast_destinations": cfg.broadcast_destinations,
        "read_only": {
            "web_bind": cfg.web_bind,
            "telnet_port": cfg.telnet_port,
            // Whether port 7575 accepts LOGIN. It changes what the telnet
            // server will do, so an admin must be able to see it without
            // reading the TOML on the box — it was invisible until 2.17.4.
            "telnet_interactive": cfg.telnet_interactive,
            "dedupe_window_secs": cfg.dedupe_window_secs,
            "spot_ring_capacity": cfg.spot_ring_capacity,
            "data_dir": cfg.data_dir,
            "cty_refresh_days": cfg.cty_refresh_days,
            "lotw_refresh_days": cfg.lotw_refresh_days,
            "iota_refresh_days": cfg.iota_refresh_days,
            "fcc_refresh_days": cfg.fcc_refresh_days,
        },
        // Server-wide, admin-only, stored in the 0600 database rather than
        // the 0644 config file. This is the ADMIN-SET key only: the built-in
        // one is deliberately never sent to a client, or the UI would become a
        // way to read a key out of any server you happen to administer.
        "clublog_api_key": app.users.db.clublog_api_key(),
        // Whether this binary ships a key of its own, so the UI can say the
        // field is optional without ever being told what the key is.
        "clublog_key_built_in": crate::builtin::has_builtin_clublog_key(),
        "cty_last_refresh_unix": app.users.db.meta_unix(crate::refresh::CTY_OK_KEY),
        // When the shared LoTW list was last actually downloaded — 0 = never
        // recorded, which is what a list seeded from a file cache looks like.
        "lotw_last_refresh_unix": app.users.db.meta_unix(crate::refresh::LOTW_OK_KEY),
        "iota_last_refresh_unix": app.users.db.meta_unix(crate::refresh::IOTA_OK_KEY),
        "fcc_last_refresh_unix": app.users.db.meta_unix(crate::refresh::FCC_OK_KEY),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct GlobalConfigReq {
    udp_sources: Vec<UdpSource>,
    cluster_nodes: Vec<ClusterNode>,
    broadcast_destinations: Vec<BroadcastDestination>,
    /// Server-wide ClubLog API key for cty.xml. Absent = leave as-is, so a
    /// client that never learned about the field cannot blank it; an empty
    /// string IS a deliberate clear.
    #[serde(default)]
    clublog_api_key: Option<String>,
}

/// Hot-apply + persist the three arrays. Bind failures (port clash)
/// reject the whole request before anything is torn down; persistence
/// failure is reported but the running state is already applied.
async fn put_global(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<GlobalConfigReq>,
) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }

    // Names must be unique — they key counters, status maps, and the
    // spotter field on cluster lines.
    for (label, names) in [
        (
            "source",
            req.udp_sources
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>(),
        ),
        (
            "node",
            req.cluster_nodes.iter().map(|n| n.name.clone()).collect(),
        ),
        (
            "destination",
            req.broadcast_destinations
                .iter()
                .map(|d| d.name.clone())
                .collect(),
        ),
    ] {
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            if name.trim().is_empty() {
                return err(
                    StatusCode::BAD_REQUEST,
                    format!("a {label} has an empty name"),
                );
            }
            if !seen.insert(name.to_uppercase()) {
                return err(
                    StatusCode::BAD_REQUEST,
                    format!("duplicate {label} name: {name}"),
                );
            }
        }
    }

    // The API key is server state, not pipeline state — persisted to the
    // database, not to config/dxca.toml (0644). `None` means the client
    // didn't send the field at all, which must not blank a stored key;
    // `Some("")` is a deliberate clear.
    if let Some(key) = &req.clublog_api_key
        && let Err(e) = app.users.db.set_clublog_api_key(key.trim())
    {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e);
    }

    // Apply: sources first (binds can fail → reject), then destinations
    // and nodes (infallible diffs).
    if let Err(e) = app
        .pipeline
        .apply_sources(&req.udp_sources, &app.input_tx)
        .await
    {
        return err(StatusCode::BAD_REQUEST, format!("source listener: {e}"));
    }
    let new_cfg = {
        let mut cfg = app.config.lock().unwrap();
        cfg.udp_sources = req.udp_sources;
        cfg.cluster_nodes = req.cluster_nodes;
        cfg.broadcast_destinations = req.broadcast_destinations;
        cfg.clone()
    };
    if let Err(e) = app
        .pipeline
        .apply_destinations(new_cfg.broadcast_destinations())
    {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("destinations: {e}"),
        );
    }
    app.nodes.apply(&new_cfg.cluster_nodes, &app.input_tx);

    match new_cfg.save(&app.config_path) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("applied, but saving the config file failed: {e}"),
        ),
    }
}

/// Send a test message through the caller's Telegram config (M5 button).
async fn telegram_test(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let service = app.users.clone();
    match tokio::task::spawn_blocking(move || service.telegram_test(user.id)).await {
        Ok(Ok(())) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

/// Refresh the global LoTW users list (admin; the list is server-wide).
/// Admin-only, like the LoTW one: cty.xml is a server-wide resource backing
/// one shared resolver, so refreshing it is not a per-user action.
async fn cty_refresh(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let service = app.users.clone();
    let key = crate::builtin::effective_clublog_api_key(&app.users.db);
    let result = tokio::task::spawn_blocking(move || service.refresh_cty(&key)).await;
    match result {
        Ok(Ok(entities)) => Json(serde_json::json!({ "cty_entities": entities })).into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

async fn lotw_refresh(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let service = app.users.clone();
    let result =
        tokio::task::spawn_blocking(move || service.refresh_lotw(dxca_connect::lotw::DEFAULT_URL))
            .await;
    match result {
        Ok(Ok(count)) => Json(serde_json::json!({"lotw_users": count})).into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

async fn iota_refresh(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let service = app.users.clone();
    let result = tokio::task::spawn_blocking(move || service.refresh_iota()).await;
    match result {
        Ok(Ok(count)) => Json(serde_json::json!({"iota_groups": count})).into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

/// The FCC pull is ~200 MB and minutes of work — still one admin POST, on a
/// blocking task like the others, because "an admin decides when" is the
/// entire safety story (`fcc.rs` module docs).
async fn fcc_refresh(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&app, &headers) {
        return resp;
    }
    let service = app.users.clone();
    let result = tokio::task::spawn_blocking(move || service.refresh_fcc()).await;
    match result {
        Ok(Ok(count)) => Json(serde_json::json!({"fcc_calls": count})).into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

// --- ClubLog refresh -----------------------------------------------------

/// Synchronous refresh (download + parse + matrix build) on a blocking
/// task; the response reports the resulting counts, 1.x-status style.
async fn refresh(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_user(&app, &headers) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let service = app.users.clone();
    let result = tokio::task::spawn_blocking(move || service.refresh_user(user.id)).await;
    match result {
        Ok(Ok((qso_count, dxcc_count))) => Json(serde_json::json!({
            "qso_count": qso_count,
            "dxcc_count": dxcc_count,
        }))
        .into_response(),
        Ok(Err(e)) => err(StatusCode::BAD_GATEWAY, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Alerts tab binds each ladder row to the `notifyField` served
    /// with it. Every level must carry one, or that row is a checkbox
    /// wired to nothing — which is what WAZ's two levels and the Marathon
    /// were between 2.19.0 and 2.20.4, sharing a single phantom slot and
    /// pinging whatever the operator ticked.
    #[test]
    fn every_served_level_carries_the_field_that_gates_it() {
        let levels = level_vocabulary();
        assert_eq!(
            levels.len(),
            dxca_core::classify::AlertLevel::FLAGGABLE.len()
        );
        for l in &levels {
            let key = l["key"].as_str().expect("a key");
            let field = l["notifyField"]
                .as_str()
                .unwrap_or_else(|| panic!("{key} is served without a notifyField"));
            assert!(field.starts_with("notify_"), "{key} names {field}");
        }
        // The three the hand-kept table missed, named so a future trim
        // cannot quietly drop them again.
        for key in ["newZone", "unconfZone", "marathon"] {
            assert!(
                levels.iter().any(|l| l["key"] == key),
                "{key} is missing from the served ladder"
            );
        }
    }
}

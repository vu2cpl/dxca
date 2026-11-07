//! DXCA server — composition root.
//!
//! M4: config → UDP source listeners + DX-cluster node clients → spot
//! pipeline → telnet server + UDP broadcast; SQLite-backed users with
//! session auth; per-user ClubLog matrices classified over the shared
//! stream; Telegram alert fan-out. The embedded web UI is still the stub
//! shell until M5 (docs/PLAN.md).

use dxca_connect::clublog::Endpoints;
use dxca_connect::telegram::Telegram;
use dxca_connect::telnet::InteractiveConfig;
use dxca_server::api::{self, AppState};
use dxca_server::auth;
use dxca_server::db::Db;
use dxca_server::nodes::NodeManager;
use dxca_server::telnetcmd::TelnetCommands;
use dxca_server::users::UserService;
use dxca_server::{config, pipeline};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

/// Subcommands, handled before anything binds a port or dials a node.
///
/// Exists for exactly one reason: an admin who forgets their password has no
/// way back in. The only password change is `PATCH /api/users/{id}`, which
/// needs an admin session — the one thing they cannot get — and `/api/setup`
/// re-arms only at **zero** accounts, so the documented "start over" route
/// means deleting every account, and with them every ClubLog setting, alert
/// preference and worked matrix on the install.
///
/// A `cargo run --example` would have been a smaller change, but it is
/// useless to the people who need it: it wants a source checkout and a Rust
/// toolchain, and the locked-out operator is typically running a release
/// binary from the installer, a Pi image, or the Windows `.exe`.
///
/// Returns `true` when a subcommand ran and `main` should stop.
fn run_subcommand() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        return false; // no arguments: start the server, exactly as before
    };
    match cmd.as_str() {
        "reset-password" => {
            reset_password(args.next());
            true
        }
        "-h" | "--help" | "help" => {
            println!(
                "dxca {}\n\n\
                 Usage:\n  \
                 dxca                         run the server (config/dxca.toml)\n  \
                 dxca reset-password <CALL>   set an account's password, for an\n                               \
                 admin who is locked out\n  \
                 dxca --help                  this message",
                env!("CARGO_PKG_VERSION")
            );
            true
        }
        other => {
            eprintln!("dxca: unknown command '{other}' — try 'dxca --help'");
            std::process::exit(2);
        }
    }
}

/// Set one account's password from the console.
///
/// The password is read from **stdin, never from argv**: an argument lands
/// in the shell history and is visible in `ps` to every user on the box, and
/// this is the one command whose whole job is a secret.
fn reset_password(callsign: Option<String>) {
    let Some(callsign) = callsign else {
        eprintln!("dxca: usage: dxca reset-password <callsign>");
        std::process::exit(2);
    };
    let callsign = callsign.to_uppercase();

    // The same config the server reads, so the operator never has to know
    // where `data_dir` points — and a typo'd path cannot silently create an
    // empty database beside the real one.
    let cfg = match config::Config::load(Path::new(config::DEFAULT_PATH)) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("dxca: config error: {e}");
            std::process::exit(1);
        }
    };
    let db = match Db::open(&Path::new(&cfg.data_dir).join("dxca.db")) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("dxca: {e}");
            std::process::exit(1);
        }
    };

    // Looked up before prompting: being told the account does not exist
    // after typing a password twice is a small cruelty.
    let found = match db.user_by_callsign(&callsign) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("dxca: {e}");
            std::process::exit(1);
        }
    };
    let Some((user, _)) = found else {
        eprintln!("dxca: no account with callsign {callsign}");
        std::process::exit(1);
    };

    print!("New password for {} ({}): ", user.callsign, user.role);
    let _ = std::io::stdout().flush();
    let mut pw = String::new();
    if std::io::stdin().lock().read_line(&mut pw).is_err() {
        eprintln!("dxca: could not read the password — nothing changed");
        std::process::exit(1);
    }
    let pw = pw.trim_end_matches(['\n', '\r']);

    // The same floor `PUT /api/users/{id}` enforces. A back door that can
    // set a password the front door would refuse is a back door.
    if pw.len() < 6 {
        eprintln!("dxca: password must be at least 6 characters — nothing changed");
        std::process::exit(1);
    }

    // The server's own hasher, so the argon2 parameters match every other
    // stored hash. Re-implementing it here is how the two drift apart and
    // the new password fails to verify against the running binary.
    let hash = match auth::hash_password(pw) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("dxca: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = db.set_pass_hash(user.id, &hash) {
        eprintln!("dxca: {e}");
        std::process::exit(1);
    }
    println!(
        "dxca: {} can now log in with the new password.",
        user.callsign
    );
}

#[tokio::main]
async fn main() {
    if run_subcommand() {
        return;
    }
    let cfg = match config::Config::load(Path::new(config::DEFAULT_PATH)) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("dxca: config error: {e}");
            std::process::exit(1);
        }
    };

    // Opened before the pipeline: the telnet server's optional login gate
    // authenticates against these accounts, so the database has to exist
    // before the listener binds.
    let db = match Db::open(&Path::new(&cfg.data_dir).join("dxca.db")) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            eprintln!("dxca: {e}");
            std::process::exit(1);
        }
    };

    // DX-cluster node clients: honest-status supervised connections. Built
    // before the pipeline because the telnet server's command passthrough
    // writes to these nodes, and the listener binds inside pipeline::start.
    let manager = Arc::new(NodeManager::new());

    let interactive = cfg.telnet_interactive.then(|| {
        let commands = TelnetCommands::start(manager.clone());
        // Closes the cycle: the sink writes to nodes, and the nodes hand
        // every event to the sink first. Must happen before any node
        // starts, or a `SHOW/DX` reply could reach the spot pipeline.
        manager.set_event_filter(commands.clone());
        InteractiveConfig {
            auth: Arc::new(dxca_server::auth::DbAuthenticator::new(db.clone())),
            commands: Some(commands),
        }
    });
    let (pipeline_state, input_tx) = match pipeline::start(&cfg, interactive).await {
        Ok(started) => started,
        Err(e) => {
            eprintln!("dxca: pipeline start failed (port clash?): {e}");
            std::process::exit(1);
        }
    };

    manager.apply(&cfg.cluster_nodes, &input_tx);
    let telegram = match &cfg.telegram_base_override {
        Some(base) => Telegram::with_base(base),
        None => Telegram::default(),
    };
    let endpoints = match &cfg.clublog_base_override {
        Some(base) => Endpoints::single_base(base),
        None => Endpoints::default(),
    };
    let users = Arc::new(UserService::new(db, &cfg.data_dir, telegram, endpoints));

    // Alert fan-out: every processed spot classifies per user.
    let mut spot_rx = pipeline_state.spot_events.subscribe();
    let fan_out_users = users.clone();
    tokio::spawn(async move {
        loop {
            match spot_rx.recv().await {
                Ok(spot) => fan_out_users.fan_out(&spot),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        }
    });

    // The ClubLog API key used to live in each user's ClubLog config even
    // though it only ever fetched the SHARED cty.xml. Lift a pre-existing one
    // to the server setting, once, so an upgrade needs no manual step.
    match users.db.adopt_legacy_api_key() {
        Ok(Some(from)) => println!("dxca: adopted ClubLog API key from {from}'s settings"),
        Ok(None) => {}
        Err(e) => eprintln!("dxca: could not check for a legacy ClubLog API key: {e}"),
    }

    // Automatic cty / ClubLog / LoTW re-download (PLAN §5's "refresh schedule").
    dxca_server::refresh::spawn(
        users.clone(),
        cfg.cty_refresh_days,
        cfg.lotw_refresh_days,
        cfg.iota_refresh_days,
        cfg.fcc_refresh_days,
    );
    // Feed-health alerts. Silent unless an account sets a threshold, and
    // powerless if this host is what failed — see the module docs.
    dxca_server::health::spawn(users.clone(), pipeline_state.clone(), manager.clone());
    // Retained status for dashboards (Home Assistant). Idle until an admin
    // ticks Status on an MQTT destination — see the module docs.
    dxca_server::statuspub::spawn(users.clone(), pipeline_state.clone(), manager.clone());
    // Once a day, a look at GitHub's latest release; a banner in the web UI
    // and one log line when there is a newer one. Never downloads anything,
    // and never asks by itself from a development build.
    let update = Arc::new(dxca_server::update::Checker::new(users.db.clone()));
    dxca_server::update::spawn(update.clone(), cfg.check_for_updates);

    let app_state = AppState {
        pipeline: pipeline_state,
        nodes: manager.clone(),
        users,
        config: Arc::new(std::sync::Mutex::new(cfg.clone())),
        config_path: Path::new(config::DEFAULT_PATH).to_path_buf(),
        input_tx: input_tx.clone(),
        update,
    };
    // MQTT destinations live in the database (they carry a broker password),
    // so they connect here rather than from the TOML config.
    match api::load_mqtt(&app_state) {
        Ok(0) => {}
        Ok(n) => println!("dxca: MQTT destinations connected ({n})"),
        Err(e) => eprintln!("dxca: MQTT load failed, nothing is published: {e}"),
    }

    // The pipeline drops blacklisted calls from its own live set, so the
    // stored list has to reach it before the first spot does.
    match api::load_blacklist(&app_state) {
        Ok(0) => {}
        Ok(n) => println!("dxca: blacklist loaded ({n} calls)"),
        Err(e) => eprintln!("dxca: blacklist load failed, nothing is blocked: {e}"),
    }

    let app = api::build_router(app_state.clone());

    let listener = match tokio::net::TcpListener::bind(&cfg.web_bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("dxca: cannot bind web {}: {e}", cfg.web_bind);
            std::process::exit(1);
        }
    };
    let sources: Vec<String> = cfg
        .udp_sources
        .iter()
        .filter(|s| s.enabled)
        .map(|s| format!("{} {}", s.name, s.port))
        .collect();
    let node_names: Vec<String> = cfg
        .cluster_nodes
        .iter()
        .filter(|n| n.enabled)
        .map(|n| n.name.clone())
        .collect();
    println!(
        "dxca {} — web http://{}/ · telnet {} · sources [{}] · nodes [{}] · users {} · cty {}",
        env!("CARGO_PKG_VERSION"),
        cfg.web_bind,
        cfg.telnet_port,
        sources.join(", "),
        node_names.join(", "),
        app_state.users.db.user_count().unwrap_or(0),
        if app_state.users.resolver_loaded() {
            format!("{} entities", app_state.users.entity_count())
        } else {
            "not loaded".to_string()
        }
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
    println!("dxca: shut down cleanly");
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    ctrl_c.await.expect("install ctrl-c handler");
}

mod bt;
mod config;
mod csv;
mod handlers;
mod models;
mod persistence;

use actix_files::Files;
use actix_web::{App, HttpServer, middleware, web};
use std::collections::HashMap;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::net::UnixListener;
use tokio::sync::broadcast;

use config::SiteConfig;
use persistence::{AppState, load_db};
use versus_shared::{Film, VoteEvent};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let cfg = SiteConfig::load();
    persistence::init_storage(&cfg);

    let csv_content = std::fs::read_to_string(&cfg.data_csv)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", cfg.data_csv));
    let films: HashMap<usize, Film> = csv::parse_csv(&csv_content, &cfg)
        .into_iter()
        .map(|f| (f.id, f))
        .collect();
    let (ratings, users, votes_on_disk) = load_db(&films);
    let (vote_tx, _) = broadcast::channel::<VoteEvent>(64);

    log::info!(
        "Loaded {} items from {} for site '{}'",
        films.len(),
        cfg.data_csv,
        cfg.display_name
    );

    let state = web::Data::new(AppState::new(
        films,
        ratings,
        users,
        vote_tx,
        votes_on_disk,
        &cfg,
    ));
    // Config the browser needs, precomputed once and served at /api/config.
    let client_config = web::Data::new(cfg.client_json());
    let port = cfg.port;
    let client_dist = cfg.client_dist.clone();

    // Poll banned.txt every 5 seconds for changes
    let ban_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            ban_state.reload_banned();
        }
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(client_config.clone())
            .wrap(middleware::Logger::new("%{r}a \"%r\" %s %b %Dms"))
            .route("/health", web::get().to(handlers::health))
            .route("/api/config", web::get().to(handlers::config))
            .route("/api/films", web::get().to(handlers::get_films))
            .route("/api/selection", web::post().to(handlers::set_selection))
            .route("/api/pair", web::get().to(handlers::get_pair))
            .route("/api/vote", web::post().to(handlers::vote))
            .route("/api/unvote", web::post().to(handlers::unvote))
            .route("/api/reset-votes", web::post().to(handlers::reset_votes))
            .route("/api/vote/stream", web::get().to(handlers::vote_stream))
            .route("/api/leaderboard", web::get().to(handlers::leaderboard))
            .route(
                "/api/leaderboard.csv",
                web::get().to(handlers::leaderboard_csv),
            )
            .route("/api/stats", web::get().to(handlers::stats))
            .route(
                "/api/user-contributions",
                web::get().to(handlers::user_contributions),
            )
            .route("/api/user-matrix", web::get().to(handlers::user_matrix))
            .route("/api/global-matrix", web::get().to(handlers::global_matrix))
            .service(Files::new("/", &client_dist).index_file("index.html"))
    })
    .shutdown_timeout(1);

    let server = match inherited_listener() {
        Some(listener) => {
            log::info!("Server running on the socket systemd passed");
            server.listen_uds(listener)?
        }
        None => {
            log::info!("Server running at http://127.0.0.1:{port}");
            server.bind(("127.0.0.1", port))?
        }
    };

    server.run().await
}

/// The one listening socket systemd passes this process, by the protocol of
/// `sd_listen_fds`: `LISTEN_PID` names it, `LISTEN_FDS` counts the sockets,
/// which begin at descriptor 3. It comes back as a duplicate that closes on
/// exec, and descriptor 3 is closed, so no child could inherit the listener.
fn inherited_listener() -> Option<UnixListener> {
    let ours = std::env::var("LISTEN_PID").ok()?.parse::<u32>().ok()? == std::process::id();
    let one = std::env::var("LISTEN_FDS").ok()? == "1";
    if !(ours && one) {
        return None;
    }
    // SAFETY: by the protocol, descriptor 3 is a listening socket this process owns.
    let passed = unsafe { OwnedFd::from_raw_fd(3) };
    Some(UnixListener::from(
        passed.try_clone().expect("the socket can be duplicated"),
    ))
}

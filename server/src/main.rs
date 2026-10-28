mod bt;
mod config;
mod csv;
mod handlers;
mod models;
mod persistence;

use actix_files::Files;
use actix_web::{App, HttpServer, middleware, web};
use std::collections::HashMap;
use std::net::TcpListener;
use std::os::unix::io::FromRawFd;
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
        cfg.require_selection,
        cfg.min_votes,
        cfg.min_voters,
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

    log::info!("Server running at http://localhost:{port}");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(client_config.clone())
            .wrap(middleware::Logger::new("%a \"%r\" %s %b %Dms"))
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

    // Use systemd socket activation if LISTEN_FDS is set, otherwise bind directly
    let server = if std::env::var("LISTEN_FDS")
        .map(|v| v.parse::<u32>().unwrap_or(0))
        .unwrap_or(0)
        >= 1
    {
        let listener = unsafe { TcpListener::from_raw_fd(3) };
        server.listen(listener)?
    } else {
        server.bind(("127.0.0.1", port))?
    };

    server.run().await
}

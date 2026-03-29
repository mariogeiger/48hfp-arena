mod bt;
mod csv;
mod handlers;
mod models;
mod persistence;

use actix_files::Files;
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use tokio::sync::broadcast;

use models::VoteEvent;
use persistence::{load_db, AppState};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let csv_content = std::fs::read_to_string("data.csv").expect("Cannot read data.csv");
    let films: HashMap<usize, models::Film> = csv::parse_csv(&csv_content)
        .into_iter()
        .map(|f| (f.id, f))
        .collect();
    let (ratings, users, votes_on_disk) = load_db(&films);
    let (vote_tx, _) = broadcast::channel::<VoteEvent>(64);

    let state = web::Data::new(AppState::new(films, ratings, users, vote_tx, votes_on_disk));

    println!("Server running at http://localhost:4848");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/api/films", web::get().to(handlers::get_films))
            .route("/api/selection", web::post().to(handlers::set_selection))
            .route("/api/pair", web::get().to(handlers::get_pair))
            .route("/api/vote", web::post().to(handlers::vote))
            .route("/api/unvote", web::post().to(handlers::unvote))
            .route("/api/vote/stream", web::get().to(handlers::vote_stream))
            .route("/api/leaderboard", web::get().to(handlers::leaderboard))
            .route("/api/stats", web::get().to(handlers::stats))
            .route("/api/user-matrix", web::get().to(handlers::user_matrix))
            .route("/api/global-matrix", web::get().to(handlers::global_matrix))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:4848")?
    .shutdown_timeout(5)
    .run()
    .await
}

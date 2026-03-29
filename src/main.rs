use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Film {
    id: usize,
    title: String,
    team: String,
    city: String,
    poster_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EloRating {
    film_id: usize,
    rating: f64,
    wins: u32,
    losses: u32,
    comparisons: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserState {
    seen_films: Vec<usize>,
    compared_pairs: Vec<(usize, usize)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    elo_ratings: HashMap<usize, EloRating>,
    users: HashMap<String, UserState>,
}

const DB_PATH: &str = "db.json";

struct AppState {
    films: Vec<Film>,
    elo_ratings: Mutex<HashMap<usize, EloRating>>,
    users: Mutex<HashMap<String, UserState>>,
}

impl AppState {
    fn save(&self) {
        let data = PersistentData {
            elo_ratings: self.elo_ratings.lock().unwrap().clone(),
            users: self.users.lock().unwrap().clone(),
        };
        if let Ok(json) = serde_json::to_string(&data) {
            let _ = std::fs::write(DB_PATH, json);
        }
    }
}

fn load_db(films: &[Film]) -> (HashMap<usize, EloRating>, HashMap<String, UserState>) {
    if let Ok(content) = std::fs::read_to_string(DB_PATH) {
        if let Ok(data) = serde_json::from_str::<PersistentData>(&content) {
            // Merge: keep saved ratings but ensure all films have an entry
            let mut ratings = data.elo_ratings;
            for film in films {
                ratings.entry(film.id).or_insert(EloRating {
                    film_id: film.id,
                    rating: 1500.0,
                    wins: 0,
                    losses: 0,
                    comparisons: 0,
                });
            }
            println!("Loaded {} ratings, {} users from {}", ratings.len(), data.users.len(), DB_PATH);
            return (ratings, data.users);
        }
    }

    let mut ratings = HashMap::new();
    for film in films {
        ratings.insert(
            film.id,
            EloRating {
                film_id: film.id,
                rating: 1500.0,
                wins: 0,
                losses: 0,
                comparisons: 0,
            },
        );
    }
    println!("No existing db found, starting fresh");
    (ratings, HashMap::new())
}

#[derive(Deserialize)]
struct SelectionPayload {
    user_id: String,
    film_ids: Vec<usize>,
}

#[derive(Deserialize)]
struct VotePayload {
    user_id: String,
    winner_id: usize,
    loser_id: usize,
}

#[derive(Deserialize)]
struct PairRequest {
    user_id: String,
}

fn parse_csv(content: &str) -> Vec<Film> {
    let mut films = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if i == 0 {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() < 2 {
            continue;
        }

        let title = parts[0].trim().trim_matches('"').to_string();
        let team = parts[1].trim().to_string();
        let city = if parts.len() > 2 {
            parts[2].trim().to_string()
        } else {
            String::new()
        };

        let poster_url = if !city.is_empty() {
            format!(
                "https://www.48hourfilm.com/storage/posters/48HFP {} 2025 - {} - Poster - file 1.jpg",
                city, team
            )
        } else {
            String::new()
        };

        films.push(Film {
            id: i,
            title,
            team,
            city,
            poster_url,
        });
    }
    films
}

fn calculate_elo(winner_rating: f64, loser_rating: f64, k: f64) -> (f64, f64) {
    let expected_winner = 1.0 / (1.0 + 10f64.powf((loser_rating - winner_rating) / 400.0));
    let expected_loser = 1.0 / (1.0 + 10f64.powf((winner_rating - loser_rating) / 400.0));
    let new_winner = winner_rating + k * (1.0 - expected_winner);
    let new_loser = loser_rating + k * (0.0 - expected_loser);
    (new_winner, new_loser)
}

async fn get_films(data: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(&data.films)
}

async fn set_selection(
    data: web::Data<AppState>,
    payload: web::Json<SelectionPayload>,
) -> HttpResponse {
    {
        let mut users = data.users.lock().unwrap();
        users.insert(
            payload.user_id.clone(),
            UserState {
                seen_films: payload.film_ids.clone(),
                compared_pairs: Vec::new(),
            },
        );
    }
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn get_pair(data: web::Data<AppState>, query: web::Query<PairRequest>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    let user = match users.get(&query.user_id) {
        Some(u) => u,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "unknown user"}));
        }
    };

    if user.seen_films.len() < 2 {
        return HttpResponse::Ok().json(serde_json::json!({"done": true}));
    }

    let mut rng = rand::thread_rng();
    let mut attempts = 0;
    loop {
        attempts += 1;
        if attempts > 500 {
            return HttpResponse::Ok().json(serde_json::json!({"done": true}));
        }
        let a = *user.seen_films.choose(&mut rng).unwrap();
        let b = *user.seen_films.choose(&mut rng).unwrap();
        if a == b {
            continue;
        }

        let pair = if a < b { (a, b) } else { (b, a) };
        if user.compared_pairs.contains(&pair) {
            continue;
        }

        let film_a = data.films.iter().find(|f| f.id == a).unwrap();
        let film_b = data.films.iter().find(|f| f.id == b).unwrap();

        return HttpResponse::Ok().json(serde_json::json!({
            "done": false,
            "a": film_a,
            "b": film_b,
        }));
    }
}

async fn vote(data: web::Data<AppState>, payload: web::Json<VotePayload>) -> HttpResponse {
    {
        let mut ratings = data.elo_ratings.lock().unwrap();
        let winner_rating = ratings
            .entry(payload.winner_id)
            .or_insert(EloRating {
                film_id: payload.winner_id,
                rating: 1500.0,
                wins: 0,
                losses: 0,
                comparisons: 0,
            })
            .rating;
        let loser_rating = ratings
            .entry(payload.loser_id)
            .or_insert(EloRating {
                film_id: payload.loser_id,
                rating: 1500.0,
                wins: 0,
                losses: 0,
                comparisons: 0,
            })
            .rating;

        let (new_winner, new_loser) = calculate_elo(winner_rating, loser_rating, 32.0);

        let w = ratings.get_mut(&payload.winner_id).unwrap();
        w.rating = new_winner;
        w.wins += 1;
        w.comparisons += 1;

        let l = ratings.get_mut(&payload.loser_id).unwrap();
        l.rating = new_loser;
        l.losses += 1;
        l.comparisons += 1;
    }

    {
        let mut users = data.users.lock().unwrap();
        if let Some(user) = users.get_mut(&payload.user_id) {
            let pair = if payload.winner_id < payload.loser_id {
                (payload.winner_id, payload.loser_id)
            } else {
                (payload.loser_id, payload.winner_id)
            };
            user.compared_pairs.push(pair);
        }
    }

    data.save();
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn leaderboard(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.elo_ratings.lock().unwrap();
    let mut board: Vec<serde_json::Value> = ratings
        .values()
        .filter(|r| r.comparisons > 0)
        .map(|r| {
            let film = data.films.iter().find(|f| f.id == r.film_id);
            serde_json::json!({
                "film_id": r.film_id,
                "title": film.map(|f| f.title.as_str()).unwrap_or("?"),
                "team": film.map(|f| f.team.as_str()).unwrap_or("?"),
                "city": film.map(|f| f.city.as_str()).unwrap_or("?"),
                "poster_url": film.map(|f| f.poster_url.as_str()).unwrap_or(""),
                "rating": (r.rating * 10.0).round() / 10.0,
                "wins": r.wins,
                "losses": r.losses,
                "comparisons": r.comparisons,
            })
        })
        .collect();

    board.sort_by(|a, b| {
        b["rating"]
            .as_f64()
            .unwrap()
            .partial_cmp(&a["rating"].as_f64().unwrap())
            .unwrap()
    });

    HttpResponse::Ok().json(board)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let csv_content = std::fs::read_to_string("data.csv").expect("Cannot read data.csv");
    let films = parse_csv(&csv_content);

    let (ratings, users) = load_db(&films);

    let state = web::Data::new(AppState {
        films,
        elo_ratings: Mutex::new(ratings),
        users: Mutex::new(users),
    });

    println!("Server running at http://localhost:4848");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/api/films", web::get().to(get_films))
            .route("/api/selection", web::post().to(set_selection))
            .route("/api/pair", web::get().to(get_pair))
            .route("/api/vote", web::post().to(vote))
            .route("/api/leaderboard", web::get().to(leaderboard))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:4848")?
    .run()
    .await
}

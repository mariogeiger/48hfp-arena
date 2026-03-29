use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tokio::sync::broadcast;

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

impl EloRating {
    fn new(film_id: usize) -> Self {
        Self {
            film_id,
            rating: 1500.0,
            wins: 0,
            losses: 0,
            comparisons: 0,
        }
    }
}

fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserState {
    seen_films: Vec<usize>,
    compared_pairs: HashSet<(usize, usize)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    elo_ratings: HashMap<usize, EloRating>,
    users: HashMap<String, UserState>,
}

const DB_PATH: &str = "db.json";
const DB_TMP_PATH: &str = "db.json.tmp";

#[derive(Debug, Clone, Serialize)]
struct VoteEvent {
    user_id: String,
    winner_title: String,
    loser_title: String,
}

struct AppState {
    films: HashMap<usize, Film>,
    film_ids: HashSet<usize>,
    elo_ratings: Mutex<HashMap<usize, EloRating>>,
    users: Mutex<HashMap<String, UserState>>,
    vote_tx: broadcast::Sender<VoteEvent>,
}

impl AppState {
    fn save(&self) {
        let data = PersistentData {
            elo_ratings: self.elo_ratings.lock().unwrap().clone(),
            users: self.users.lock().unwrap().clone(),
        };
        if let Ok(json) = serde_json::to_string(&data) {
            // Atomic write: write to tmp then rename
            if std::fs::write(DB_TMP_PATH, &json).is_ok() {
                let _ = std::fs::rename(DB_TMP_PATH, DB_PATH);
            }
        }
    }
}

fn load_db(films: &HashMap<usize, Film>) -> (HashMap<usize, EloRating>, HashMap<String, UserState>) {
    let (mut ratings, users) = std::fs::read_to_string(DB_PATH)
        .ok()
        .and_then(|c| serde_json::from_str::<PersistentData>(&c).ok())
        .map(|data| {
            println!(
                "Loaded {} ratings, {} users from {}",
                data.elo_ratings.len(),
                data.users.len(),
                DB_PATH
            );
            (data.elo_ratings, data.users)
        })
        .unwrap_or_else(|| {
            println!("No existing db found, starting fresh");
            (HashMap::new(), HashMap::new())
        });

    for &id in films.keys() {
        ratings.entry(id).or_insert_with(|| EloRating::new(id));
    }
    (ratings, users)
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
    let films: Vec<&Film> = data.films.values().collect();
    HttpResponse::Ok().json(films)
}

async fn set_selection(
    data: web::Data<AppState>,
    payload: web::Json<SelectionPayload>,
) -> HttpResponse {
    {
        let mut users = data.users.lock().unwrap();
        let user = users
            .entry(payload.user_id.clone())
            .or_insert_with(|| UserState {
                seen_films: Vec::new(),
                compared_pairs: HashSet::new(),
            });
        user.seen_films = payload.film_ids.clone();
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

    let seen = &user.seen_films;
    let mut remaining = Vec::new();
    for i in 0..seen.len() {
        for j in (i + 1)..seen.len() {
            let pair = canonical_pair(seen[i], seen[j]);
            if !user.compared_pairs.contains(&pair) {
                remaining.push(pair);
            }
        }
    }

    if remaining.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({"done": true}));
    }

    let mut rng = rand::thread_rng();
    let &(a, b) = remaining.choose(&mut rng).unwrap();

    let film_a = &data.films[&a];
    let film_b = &data.films[&b];

    HttpResponse::Ok().json(serde_json::json!({
        "done": false,
        "a": film_a,
        "b": film_b,
        "remaining": remaining.len(),
    }))
}

async fn vote(data: web::Data<AppState>, payload: web::Json<VotePayload>) -> HttpResponse {
    // Validate film IDs exist
    if !data.film_ids.contains(&payload.winner_id)
        || !data.film_ids.contains(&payload.loser_id)
        || payload.winner_id == payload.loser_id
    {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "invalid film ids"}));
    }

    let pair = canonical_pair(payload.winner_id, payload.loser_id);

    {
        let users = data.users.lock().unwrap();
        if let Some(user) = users.get(&payload.user_id) {
            if user.compared_pairs.contains(&pair) {
                return HttpResponse::Ok()
                    .json(serde_json::json!({"status": "already_voted"}));
            }
        }
    }

    {
        let mut ratings = data.elo_ratings.lock().unwrap();
        let winner_rating = ratings
            .entry(payload.winner_id)
            .or_insert_with(|| EloRating::new(payload.winner_id))
            .rating;
        let loser_rating = ratings
            .entry(payload.loser_id)
            .or_insert_with(|| EloRating::new(payload.loser_id))
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
            user.compared_pairs.insert(pair);
        }
    }

    data.save();

    let winner_title = data.films.get(&payload.winner_id)
        .map(|f| f.title.clone()).unwrap_or_default();
    let loser_title = data.films.get(&payload.loser_id)
        .map(|f| f.title.clone()).unwrap_or_default();
    let _ = data.vote_tx.send(VoteEvent {
        user_id: payload.user_id.clone(),
        winner_title,
        loser_title,
    });

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn vote_stream(data: web::Data<AppState>) -> HttpResponse {
    let mut rx = data.vote_tx.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap();
                    yield Ok::<_, actix_web::Error>(
                        actix_web::web::Bytes::from(format!("data: {}\n\n", json))
                    );
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

async fn leaderboard(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.elo_ratings.lock().unwrap();
    let mut board: Vec<serde_json::Value> = ratings
        .values()
        .filter(|r| r.comparisons > 0)
        .map(|r| {
            let film = data.films.get(&r.film_id);
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
    let films: HashMap<usize, Film> = parse_csv(&csv_content)
        .into_iter()
        .map(|f| (f.id, f))
        .collect();
    let film_ids: HashSet<usize> = films.keys().copied().collect();

    let (ratings, users) = load_db(&films);
    let (vote_tx, _) = broadcast::channel::<VoteEvent>(64);

    let state = web::Data::new(AppState {
        films,
        film_ids,
        elo_ratings: Mutex::new(ratings),
        users: Mutex::new(users),
        vote_tx,
    });

    println!("Server running at http://localhost:4848");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/api/films", web::get().to(get_films))
            .route("/api/selection", web::post().to(set_selection))
            .route("/api/pair", web::get().to(get_pair))
            .route("/api/vote", web::post().to(vote))
            .route("/api/vote/stream", web::get().to(vote_stream))
            .route("/api/leaderboard", web::get().to(leaderboard))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:4848")?
    .run()
    .await
}

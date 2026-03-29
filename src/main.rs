use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
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
    comparisons: u32,
}

impl EloRating {
    fn new(film_id: usize) -> Self {
        Self {
            film_id,
            rating: 1500.0,
            wins: 0,
            comparisons: 0,
        }
    }
    fn losses(&self) -> u32 {
        self.comparisons - self.wins
    }
}

fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastVote {
    winner_id: usize,
    loser_id: usize,
    winner_delta: f64,
    loser_delta: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UserState {
    seen_films: Vec<usize>,
    compared_pairs: HashSet<(usize, usize)>,
    #[serde(skip)]
    vote_history: Vec<LastVote>,
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
            // Atomic write: write to tmp, fsync, then rename
            if let Ok(file) = std::fs::File::create(DB_TMP_PATH) {
                use std::io::Write;
                let mut writer = std::io::BufWriter::new(file);
                if writer.write_all(json.as_bytes()).is_ok()
                    && writer.flush().is_ok()
                    && writer.get_ref().sync_all().is_ok()
                {
                    let _ = std::fs::rename(DB_TMP_PATH, DB_PATH);
                }
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

#[derive(Deserialize)]
struct UndoPayload {
    user_id: String,
}

fn parse_csv(content: &str) -> Vec<Film> {
    content.lines()
        .skip(1)
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            let parts: Vec<&str> = line.splitn(3, ',').collect();
            let title = parts.first()?.trim().trim_matches('"').to_string();
            let team = parts.get(1)?.trim().to_string();
            let city = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let poster_url = format!(
                "https://www.48hourfilm.com/storage/posters/48HFP {} 2025 - {} - Poster - file 1.jpg",
                city, team
            );
            Some(Film { id: i + 1, title, team, city, poster_url })
        })
        .collect()
}

fn calculate_elo(winner_rating: f64, loser_rating: f64, k: f64) -> (f64, f64) {
    let expected = 1.0 / (1.0 + 10f64.powf((loser_rating - winner_rating) / 400.0));
    let delta = k * (1.0 - expected);
    (winner_rating + delta, loser_rating - delta)
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
        let user = users.entry(payload.user_id.clone()).or_default();
        user.seen_films = payload.film_ids.clone();
    }
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn get_pair(data: web::Data<AppState>, query: web::Query<PairRequest>) -> HttpResponse {
    let mut users = data.users.lock().unwrap();
    let user = users.entry(query.user_id.clone()).or_default();

    let votes = user.compared_pairs.len();

    if user.seen_films.len() < 2 {
        return HttpResponse::Ok().json(serde_json::json!({"done": true, "votes": votes}));
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
        return HttpResponse::Ok().json(serde_json::json!({"done": true, "votes": votes}));
    }

    let ratings = data.elo_ratings.lock().unwrap();
    let weights: Vec<f64> = remaining.iter().map(|&(a, b)| {
        let ra = ratings.get(&a).map(|r| (r.rating, r.comparisons)).unwrap_or((1500.0, 0));
        let rb = ratings.get(&b).map(|r| (r.rating, r.comparisons)).unwrap_or((1500.0, 0));
        let closeness = 1.0 / (1.0 + (ra.0 - rb.0).abs() / 200.0);
        let uncertainty = 2.0 / (2.0 + ra.1 as f64 + rb.1 as f64);
        let quality = ((ra.0 + rb.0) / 2.0 - 1400.0).max(0.0) / 200.0;
        closeness + uncertainty + quality
    }).collect();
    drop(ratings);

    let mut rng = rand::thread_rng();
    let dist = WeightedIndex::new(&weights).unwrap();
    let (a, b) = remaining[rng.sample(&dist)];

    let film_a = &data.films[&a];
    let film_b = &data.films[&b];

    HttpResponse::Ok().json(serde_json::json!({
        "done": false,
        "a": film_a,
        "b": film_b,
        "remaining": remaining.len(),
        "votes": votes,
    }))
}

async fn vote(data: web::Data<AppState>, payload: web::Json<VotePayload>) -> HttpResponse {
    // Validate film IDs exist
    if !data.films.contains_key(&payload.winner_id)
        || !data.films.contains_key(&payload.loser_id)
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

    let last_vote;
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

        last_vote = LastVote {
            winner_id: payload.winner_id,
            loser_id: payload.loser_id,
            winner_delta: new_winner - winner_rating,
            loser_delta: new_loser - loser_rating,
        };

        let w = ratings.get_mut(&payload.winner_id).unwrap();
        w.rating = new_winner;
        w.wins += 1;
        w.comparisons += 1;

        let l = ratings.get_mut(&payload.loser_id).unwrap();
        l.rating = new_loser;
        l.comparisons += 1;
    }

    {
        let mut users = data.users.lock().unwrap();
        if let Some(user) = users.get_mut(&payload.user_id) {
            user.compared_pairs.insert(pair);
            user.vote_history.push(last_vote);
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

async fn undo(data: web::Data<AppState>, payload: web::Json<UndoPayload>) -> HttpResponse {
    let last_vote = {
        let mut users = data.users.lock().unwrap();
        let user = match users.get_mut(&payload.user_id) {
            Some(u) => u,
            None => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "unknown user"}));
            }
        };
        let lv = match user.vote_history.pop() {
            Some(lv) => lv,
            None => {
                return HttpResponse::Ok()
                    .json(serde_json::json!({"status": "nothing_to_undo"}));
            }
        };
        let pair = canonical_pair(lv.winner_id, lv.loser_id);
        user.compared_pairs.remove(&pair);
        lv
    };

    {
        let mut ratings = data.elo_ratings.lock().unwrap();
        if let Some(w) = ratings.get_mut(&last_vote.winner_id) {
            w.rating -= last_vote.winner_delta;
            w.wins = w.wins.saturating_sub(1);
            w.comparisons = w.comparisons.saturating_sub(1);
        }
        if let Some(l) = ratings.get_mut(&last_vote.loser_id) {
            l.rating -= last_vote.loser_delta;
            l.comparisons = l.comparisons.saturating_sub(1);
        }
    }

    data.save();

    let film_a = &data.films[&last_vote.winner_id];
    let film_b = &data.films[&last_vote.loser_id];
    HttpResponse::Ok().json(serde_json::json!({
        "status": "undone",
        "a": film_a,
        "b": film_b,
    }))
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

async fn stats(data: web::Data<AppState>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    let ratings = data.elo_ratings.lock().unwrap();

    let total_users = users.len();
    let active_users = users.values().filter(|u| !u.compared_pairs.is_empty()).count();
    let total_votes: usize = users.values().map(|u| u.compared_pairs.len()).sum();
    let films_with_votes = ratings.values().filter(|r| r.comparisons > 0).count();
    let total_films = data.films.len();

    let avg_votes_per_user = if active_users > 0 {
        total_votes as f64 / active_users as f64
    } else {
        0.0
    };

    let mut films_selected_count: HashMap<usize, u32> = HashMap::new();
    for user in users.values() {
        for &fid in &user.seen_films {
            *films_selected_count.entry(fid).or_insert(0) += 1;
        }
    }
    let most_selected: Vec<serde_json::Value> = {
        let mut entries: Vec<_> = films_selected_count.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        entries.iter().take(10).map(|&(&fid, &count)| {
            let title = data.films.get(&fid).map(|f| f.title.as_str()).unwrap_or("?");
            serde_json::json!({"film_id": fid, "title": title, "count": count})
        }).collect()
    };

    let mut vote_dist: Vec<(usize, u32)> = {
        let mut counts = HashMap::<usize, u32>::new();
        for u in users.values() {
            if !u.compared_pairs.is_empty() {
                *counts.entry(u.compared_pairs.len()).or_default() += 1;
            }
        }
        counts.into_iter().collect()
    };
    vote_dist.sort_by_key(|&(votes, _)| votes);
    let vote_distribution: Vec<serde_json::Value> = vote_dist
        .iter()
        .map(|&(votes, users)| serde_json::json!({"votes": votes, "users": users}))
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "total_users": total_users,
        "active_users": active_users,
        "total_votes": total_votes,
        "total_films": total_films,
        "films_with_votes": films_with_votes,
        "avg_votes_per_user": (avg_votes_per_user * 10.0).round() / 10.0,
        "most_selected_films": most_selected,
        "vote_distribution": vote_distribution,
    }))
}

async fn leaderboard(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.elo_ratings.lock().unwrap();
    let mut ranked: Vec<&EloRating> = ratings.values().filter(|r| r.comparisons > 0).collect();
    ranked.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());

    let board: Vec<serde_json::Value> = ranked
        .iter()
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
                "losses": r.losses(),
                "comparisons": r.comparisons,
            })
        })
        .collect();

    HttpResponse::Ok().json(board)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let csv_content = std::fs::read_to_string("data.csv").expect("Cannot read data.csv");
    let films: HashMap<usize, Film> = parse_csv(&csv_content)
        .into_iter()
        .map(|f| (f.id, f))
        .collect();
    let (ratings, users) = load_db(&films);
    let (vote_tx, _) = broadcast::channel::<VoteEvent>(64);

    let state = web::Data::new(AppState {
        films,
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
            .route("/api/undo", web::post().to(undo))
            .route("/api/vote/stream", web::get().to(vote_stream))
            .route("/api/leaderboard", web::get().to(leaderboard))
            .route("/api/stats", web::get().to(stats))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:4848")?
    .run()
    .await
}

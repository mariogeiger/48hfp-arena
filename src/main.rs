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
struct BtRating {
    film_id: usize,
    /// Bradley-Terry strength parameter β (normalized so geometric mean of active films = 1).
    score: f64,
    comparisons: u32,
    /// Pairwise win counts: film_id → number of times this film beat that film.
    wins_against: HashMap<usize, u32>,
}

impl BtRating {
    fn new(film_id: usize) -> Self {
        Self {
            film_id,
            score: 1.0,
            comparisons: 0,
            wins_against: HashMap::new(),
        }
    }
    fn wins(&self) -> u32 {
        self.wins_against.values().sum()
    }
    fn losses(&self) -> u32 {
        self.comparisons - self.wins()
    }
}

fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastVote {
    winner_id: usize,
    loser_id: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UserState {
    seen_films: Vec<usize>,
    compared_pairs: HashSet<(usize, usize)>,
    vote_history: Vec<LastVote>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    bt_ratings: HashMap<usize, BtRating>,
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
    bt_ratings: Mutex<HashMap<usize, BtRating>>,
    users: Mutex<HashMap<String, UserState>>,
    vote_tx: broadcast::Sender<VoteEvent>,
}

impl AppState {
    fn save(&self) {
        let data = PersistentData {
            bt_ratings: self.bt_ratings.lock().unwrap().clone(),
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

fn load_db(films: &HashMap<usize, Film>) -> (HashMap<usize, BtRating>, HashMap<String, UserState>) {
    let (mut ratings, users) = std::fs::read_to_string(DB_PATH)
        .ok()
        .and_then(|c| serde_json::from_str::<PersistentData>(&c).ok())
        .map(|data| {
            println!(
                "Loaded {} ratings, {} users from {}",
                data.bt_ratings.len(),
                data.users.len(),
                DB_PATH
            );
            (data.bt_ratings, data.users)
        })
        .unwrap_or_else(|| {
            println!("No existing db found, starting fresh");
            (HashMap::new(), HashMap::new())
        });

    for &id in films.keys() {
        ratings.entry(id).or_insert_with(|| BtRating::new(id));
    }
    (ratings, users)
}

/// Run the Bradley-Terry MM algorithm to convergence.
///
/// The model assigns each film a strength β_i > 0 such that
///   P(i beats j) = β_i / (β_i + β_j).
///
/// The MM update is:
///   β_i ← W_i / Σ_{j≠i} n_ij / (β_i + β_j)
/// where W_i = total wins and n_ij = total comparisons between i and j.
///
/// After each iteration the strengths are normalized so the geometric mean
/// of active films equals 1, which anchors the scale.
///
/// Films with zero wins keep a near-zero score (they sit below all ranked films).
fn run_bradley_terry(ratings: &mut HashMap<usize, BtRating>) {
    let active_ids: Vec<usize> = ratings.values()
        .filter(|r| r.comparisons > 0)
        .map(|r| r.film_id)
        .collect();

    if active_ids.is_empty() {
        return;
    }

    // Films with 0 wins cannot be estimated by MLE (β → 0); pin them to a small floor.
    for &id in &active_ids {
        if ratings[&id].wins() == 0 {
            ratings.get_mut(&id).unwrap().score = 1e-6;
        }
    }

    let ranked_ids: Vec<usize> = active_ids.iter()
        .copied()
        .filter(|&id| ratings[&id].wins() > 0)
        .collect();

    if ranked_ids.is_empty() {
        return;
    }

    // Map film_id → index into old_scores; allocated once, reused each iteration.
    let idx: HashMap<usize, usize> = active_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let mut old_scores: Vec<f64> = active_ids.iter().map(|&id| ratings[&id].score).collect();

    for _ in 0..500 {
        // Snapshot scores at the start of this iteration.
        for (pos, &id) in active_ids.iter().enumerate() {
            old_scores[pos] = ratings[&id].score;
        }

        let mut max_rel_change = 0.0_f64;

        for &i in &ranked_ids {
            let w_i = ratings[&i].wins() as f64;
            let score_i = old_scores[idx[&i]];

            let denom: f64 = active_ids.iter()
                .filter(|&&j| j != i)
                .filter_map(|&j| {
                    let n_ij =
                        ratings[&i].wins_against.get(&j).copied().unwrap_or(0) as f64
                        + ratings[&j].wins_against.get(&i).copied().unwrap_or(0) as f64;
                    if n_ij > 0.0 {
                        Some(n_ij / (score_i + old_scores[idx[&j]]))
                    } else {
                        None
                    }
                })
                .sum();

            if denom > 0.0 {
                let new_score = w_i / denom;
                let rel_change = (new_score - score_i).abs() / score_i;
                max_rel_change = max_rel_change.max(rel_change);
                ratings.get_mut(&i).unwrap().score = new_score;
            }
        }

        let log_mean: f64 = ranked_ids.iter()
            .map(|&id| ratings[&id].score.ln())
            .sum::<f64>()
            / ranked_ids.len() as f64;
        let scale = (-log_mean).exp();
        for &id in &ranked_ids {
            ratings.get_mut(&id).unwrap().score *= scale;
        }

        if max_rel_change < 1e-8 {
            break;
        }
    }
}

/// Convert a Bradley-Terry strength β to an ELO-equivalent display rating.
/// With the normalization β̄ = 1 (geometric mean), the average film scores 1500.
fn bt_score_to_display(score: f64) -> f64 {
    400.0 * score.max(1e-10).log10() + 1500.0
}

async fn get_films(data: web::Data<AppState>) -> HttpResponse {
    let films: Vec<&Film> = data.films.values().collect();
    HttpResponse::Ok().json(films)
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

    let ratings = data.bt_ratings.lock().unwrap();
    let weights: Vec<f64> = remaining.iter().map(|&(a, b)| {
        let ra = ratings.get(&a).map(|r| (r.score, r.comparisons)).unwrap_or((1.0, 0));
        let rb = ratings.get(&b).map(|r| (r.score, r.comparisons)).unwrap_or((1.0, 0));

        // Convert to display-rating space for intuitive thresholds.
        let ra_disp = bt_score_to_display(ra.0);
        let rb_disp = bt_score_to_display(rb.0);

        // Prefer pairs where ratings are close (high information gain).
        let closeness = 1.0 / (1.0 + (ra_disp - rb_disp).abs() / 200.0);
        // Prefer films with fewer comparisons (reduce uncertainty).
        let uncertainty = 2.0 / (2.0 + ra.1 as f64 + rb.1 as f64);
        // Prefer higher-rated pairs (surface quality content).
        let quality = ((ra_disp + rb_disp) / 2.0 - 1400.0).max(0.0) / 200.0;

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

    {
        let mut ratings = data.bt_ratings.lock().unwrap();

        let w = ratings.entry(payload.winner_id).or_insert_with(|| BtRating::new(payload.winner_id));
        w.comparisons += 1;
        *w.wins_against.entry(payload.loser_id).or_insert(0) += 1;

        let l = ratings.entry(payload.loser_id).or_insert_with(|| BtRating::new(payload.loser_id));
        l.comparisons += 1;

        run_bradley_terry(&mut ratings);
    }

    {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(payload.user_id.clone()).or_default();
        user.compared_pairs.insert(pair);
        user.vote_history.push(LastVote {
            winner_id: payload.winner_id,
            loser_id: payload.loser_id,
        });
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
        let mut ratings = data.bt_ratings.lock().unwrap();

        if let Some(w) = ratings.get_mut(&last_vote.winner_id) {
            w.comparisons = w.comparisons.saturating_sub(1);
            let entry = w.wins_against.entry(last_vote.loser_id).or_insert(0);
            *entry = entry.saturating_sub(1);
        }
        if let Some(l) = ratings.get_mut(&last_vote.loser_id) {
            l.comparisons = l.comparisons.saturating_sub(1);
        }

        run_bradley_terry(&mut ratings);
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
    let ratings = data.bt_ratings.lock().unwrap();

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
    let ratings = data.bt_ratings.lock().unwrap();
    let mut ranked: Vec<&BtRating> = ratings.values().filter(|r| r.comparisons > 0).collect();
    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

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
                "rating": (bt_score_to_display(r.score) * 10.0).round() / 10.0,
                "wins": r.wins(),
                "losses": r.losses(),
                "comparisons": r.comparisons,
            })
        })
        .collect();

    HttpResponse::Ok().json(board)
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
        bt_ratings: Mutex::new(ratings),
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

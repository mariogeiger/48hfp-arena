mod bt;
use bt::{bt_score_to_display, run_bradley_terry, BtRating};

use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
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

fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UserState {
    seen_films: Vec<usize>,
    compared_pairs: HashSet<(usize, usize)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    bt_ratings: HashMap<usize, BtRating>,
    users: HashMap<String, UserState>,
}

const DB_PATH: &str = "db.json";
const DB_TMP_PATH: &str = "db.json.tmp";
const BACKUP_DIR: &str = "backups";
const MAX_BACKUPS: usize = 100;

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
    /// Vote count loaded from disk — save() refuses to write fewer votes than this.
    votes_on_disk: AtomicUsize,
}

fn count_votes(users: &HashMap<String, UserState>) -> usize {
    users.values().map(|u| u.compared_pairs.len()).sum()
}

impl AppState {
    fn save(&self) {
        let data = PersistentData {
            bt_ratings: self.bt_ratings.lock().unwrap().clone(),
            users: self.users.lock().unwrap().clone(),
        };

        // Safety check: never overwrite db with fewer votes than what was on disk
        let new_votes = count_votes(&data.users);
        let min_votes = self.votes_on_disk.load(Ordering::Relaxed);
        if new_votes < min_votes {
            eprintln!(
                "REFUSING TO SAVE: new data has {} votes but db on disk had {}. \
                 This looks like data loss. Backup exists in {}/",
                new_votes, min_votes, BACKUP_DIR
            );
            return;
        }

        if let Ok(json) = serde_json::to_string(&data) {
            // Backup current db.json before overwriting
            backup_db();

            // Atomic write: write to tmp, fsync, then rename
            if let Ok(file) = std::fs::File::create(DB_TMP_PATH) {
                use std::io::Write;
                let mut writer = std::io::BufWriter::new(file);
                if writer.write_all(json.as_bytes()).is_ok()
                    && writer.flush().is_ok()
                    && writer.get_ref().sync_all().is_ok()
                {
                    let _ = std::fs::rename(DB_TMP_PATH, DB_PATH);
                    // Update the floor to the new count
                    self.votes_on_disk.store(new_votes, Ordering::Relaxed);
                }
            }
        }
    }
}

fn backup_db() {
    use std::time::SystemTime;

    // Only backup if db.json exists and is non-empty
    let _metadata = match std::fs::metadata(DB_PATH) {
        Ok(m) if m.len() > 0 => m,
        _ => return,
    };

    let _ = std::fs::create_dir_all(BACKUP_DIR);

    // Hourly backup: db_20260329_14.json (one per hour)
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let secs_of_day = timestamp % 86400;
    let days = timestamp / 86400;
    let (year, month, day) = unix_days_to_date(days as i64);
    let hour = secs_of_day / 3600;
    let backup_name = format!(
        "{}/db_{:04}{:02}{:02}_{:02}.json",
        BACKUP_DIR, year, month, day, hour
    );

    // Skip if this hour's backup already exists
    if std::path::Path::new(&backup_name).exists() {
        return;
    }

    if let Err(e) = std::fs::copy(DB_PATH, &backup_name) {
        eprintln!("Warning: failed to backup db: {}", e);
        return;
    }

    // Prune old backups, keep most recent MAX_BACKUPS
    if let Ok(entries) = std::fs::read_dir(BACKUP_DIR) {
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with("db_") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();
        if backups.len() > MAX_BACKUPS {
            backups.sort_by_key(|e| e.file_name());
            for entry in &backups[..backups.len() - MAX_BACKUPS] {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

fn unix_days_to_date(days: i64) -> (i64, u32, u32) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Count "compared_pairs" in a raw JSON value (works even if schema changed).
fn count_votes_in_raw_json(content: &str) -> usize {
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(content) else {
        return 0;
    };
    let Some(users) = raw.get("users").and_then(|u| u.as_object()) else {
        return 0;
    };
    users.values()
        .filter_map(|u| u.get("compared_pairs").and_then(|p| p.as_array()))
        .map(|pairs| pairs.len())
        .sum()
}

/// Returns (ratings, users, votes_on_disk).
/// `votes_on_disk` is the vote count from the file on disk — even if the schema
/// didn't parse, we extract it from raw JSON so the safety floor is set.
fn load_db(films: &HashMap<usize, Film>) -> (HashMap<usize, BtRating>, HashMap<String, UserState>, usize) {
    let (mut ratings, users, votes_on_disk) = match std::fs::read_to_string(DB_PATH) {
        Err(_) => {
            println!("No existing db found, starting fresh");
            (HashMap::new(), HashMap::new(), 0)
        }
        Ok(content) => {
            // Always count votes from raw JSON first — this works regardless of schema
            let raw_votes = count_votes_in_raw_json(&content);

            match serde_json::from_str::<PersistentData>(&content) {
                Ok(data) => {
                    let votes = count_votes(&data.users);
                    println!(
                        "Loaded {} ratings, {} users, {} votes from {}",
                        data.bt_ratings.len(),
                        data.users.len(),
                        votes,
                        DB_PATH
                    );
                    (data.bt_ratings, data.users, votes)
                }
                Err(e) => {
                    // Schema mismatch or corruption — save the old file so data isn't lost
                    let rescue_path = format!("{}/db_rescue_{}.json", BACKUP_DIR, std::process::id());
                    let _ = std::fs::create_dir_all(BACKUP_DIR);
                    eprintln!(
                        "ERROR: Failed to parse {}: {}",
                        DB_PATH, e
                    );
                    eprintln!(
                        "Saving unreadable db to {} — starting fresh",
                        rescue_path
                    );
                    eprintln!(
                        "Old db had {} votes — save() will refuse to overwrite until that count is exceeded",
                        raw_votes
                    );
                    let _ = std::fs::copy(DB_PATH, &rescue_path);
                    (HashMap::new(), HashMap::new(), raw_votes)
                }
            }
        }
    };

    for &id in films.keys() {
        ratings.entry(id).or_insert_with(|| BtRating::new(id));
    }
    (ratings, users, votes_on_disk)
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
    winner_id: usize,
    loser_id: usize,
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
    let pair = canonical_pair(payload.winner_id, payload.loser_id);

    {
        let mut users = data.users.lock().unwrap();
        let user = match users.get_mut(&payload.user_id) {
            Some(u) => u,
            None => return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "unknown user"})),
        };
        if !user.compared_pairs.remove(&pair) {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "pair not found"}));
        }
    }

    {
        let mut ratings = data.bt_ratings.lock().unwrap();

        if let Some(w) = ratings.get_mut(&payload.winner_id) {
            w.comparisons = w.comparisons.saturating_sub(1);
            let entry = w.wins_against.entry(payload.loser_id).or_insert(0);
            *entry = entry.saturating_sub(1);
        }
        if let Some(l) = ratings.get_mut(&payload.loser_id) {
            l.comparisons = l.comparisons.saturating_sub(1);
        }

        run_bradley_terry(&mut ratings);
    }

    data.save();

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
    let (ratings, users, votes_on_disk) = load_db(&films);
    let (vote_tx, _) = broadcast::channel::<VoteEvent>(64);

    let state = web::Data::new(AppState {
        films,
        bt_ratings: Mutex::new(ratings),
        users: Mutex::new(users),
        vote_tx,
        votes_on_disk: AtomicUsize::new(votes_on_disk),
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

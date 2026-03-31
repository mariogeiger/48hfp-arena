use crate::bt::{BtRating, run_bradley_terry};
use crate::models::{Film, PersistentData, UserState, VoteEvent, parse_pair_key};

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

pub const DB_PATH: &str = "db.json";
const DB_TMP_PATH: &str = "db.json.tmp";
const BACKUP_DIR: &str = "backups";
const MAX_BACKUPS: usize = 100;
const BANNED_PATH: &str = "banned.txt";

pub struct AppState {
    pub films: HashMap<usize, Film>,
    pub bt_ratings: Mutex<HashMap<usize, BtRating>>,
    pub users: Mutex<HashMap<String, UserState>>,
    pub vote_tx: broadcast::Sender<VoteEvent>,
    /// Vote count loaded from disk — save() refuses to write fewer votes than this.
    votes_on_disk: AtomicUsize,
    pub banned: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new(
        films: HashMap<usize, Film>,
        ratings: HashMap<usize, BtRating>,
        users: HashMap<String, UserState>,
        vote_tx: broadcast::Sender<VoteEvent>,
        votes_on_disk: usize,
    ) -> Self {
        let banned = load_banned();
        Self {
            films,
            bt_ratings: Mutex::new(ratings),
            users: Mutex::new(users),
            vote_tx,
            votes_on_disk: AtomicUsize::new(votes_on_disk),
            banned: Mutex::new(banned),
        }
    }

    pub fn is_banned(&self, user_id: &str) -> bool {
        self.banned.lock().unwrap().contains(user_id)
    }

    /// Reload banned.txt; if it changed, recompute BT ratings from scratch
    /// excluding banned users' votes.
    pub fn reload_banned(&self) {
        let new_banned = load_banned();
        {
            let mut current = self.banned.lock().unwrap();
            if *current == new_banned {
                return;
            }
            *current = new_banned.clone();
        }
        log::info!("Ban list changed, recomputing ratings...");

        let mut ratings = self.bt_ratings.lock().unwrap();
        let users = self.users.lock().unwrap();

        // Reset all ratings
        for r in ratings.values_mut() {
            r.score = 1.0;
            r.comparisons = 0;
            r.wins_against.clear();
        }

        // Replay all non-banned votes
        for (uid, state) in users.iter() {
            if new_banned.contains(uid) {
                continue;
            }
            for (key, &winner) in &state.vote_outcomes {
                let Some((a, b)) = parse_pair_key(key) else {
                    continue;
                };
                let loser = if winner == a { b } else { a };

                let w = ratings
                    .entry(winner)
                    .or_insert_with(|| BtRating::new(winner));
                w.comparisons += 1;
                *w.wins_against.entry(loser).or_insert(0) += 1;

                let l = ratings.entry(loser).or_insert_with(|| BtRating::new(loser));
                l.comparisons += 1;
            }
        }

        run_bradley_terry(&mut ratings);
        log::info!("Ratings recomputed after ban list update");
    }

    pub fn save(&self) {
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

fn count_votes(users: &HashMap<String, UserState>) -> usize {
    users.values().map(|u| u.compared_pairs.len()).sum()
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
    users
        .values()
        .filter_map(|u| u.get("compared_pairs").and_then(|p| p.as_array()))
        .map(|pairs| pairs.len())
        .sum()
}

/// Returns (ratings, users, votes_on_disk).
/// `votes_on_disk` is the vote count from the file on disk — even if the schema
/// didn't parse, we extract it from raw JSON so the safety floor is set.
pub fn load_db(
    films: &HashMap<usize, Film>,
) -> (HashMap<usize, BtRating>, HashMap<String, UserState>, usize) {
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
                    let rescue_path =
                        format!("{}/db_rescue_{}.json", BACKUP_DIR, std::process::id());
                    let _ = std::fs::create_dir_all(BACKUP_DIR);
                    eprintln!("ERROR: Failed to parse {}: {}", DB_PATH, e);
                    eprintln!("Saving unreadable db to {} — starting fresh", rescue_path);
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

/// Read banned user IDs from banned.txt (one UUID per line, # comments allowed).
pub fn load_banned() -> HashSet<String> {
    let content = match std::fs::read_to_string(BANNED_PATH) {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

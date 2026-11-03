use crate::bt::{BtRating, bt_score_to_display, fisher_pair_scores};
use crate::models::UserState;
use crate::persistence::AppState;
use versus_shared::*;

use actix_web::{HttpResponse, web};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast;

pub async fn health(data: web::Data<AppState>) -> HttpResponse {
    let ratings_ok = data.bt_ratings.try_lock().is_ok();
    let users_ok = data.users.try_lock().is_ok();
    if ratings_ok && users_ok {
        HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
    } else {
        log::error!(
            "Health check FAILED: ratings_lock={}, users_lock={}",
            ratings_ok,
            users_ok
        );
        HttpResponse::InternalServerError()
            .json(serde_json::json!({"status": "deadlocked", "ratings_lock": ratings_ok, "users_lock": users_ok}))
    }
}

/// Serves the site's client-facing configuration (branding, copy, column
/// flags). Precomputed in `main` and stored as shared app data.
pub async fn config(cfg: web::Data<serde_json::Value>) -> HttpResponse {
    HttpResponse::Ok().json(cfg.get_ref())
}

pub async fn get_films(data: web::Data<AppState>) -> HttpResponse {
    let films = data.films();
    let films: Vec<&Film> = films.values().collect();
    HttpResponse::Ok().json(films)
}

pub async fn set_selection(
    data: web::Data<AppState>,
    payload: web::Json<SelectionPayload>,
) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(payload.user_id.clone()).or_default();
        user.seen_films = payload.film_ids.clone();
    }
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

/// The items a user compares: every item when the site has no selection
/// step, else the user's own picks, as far as they are published.
fn in_play(data: &AppState, user: &UserState, films: &HashMap<usize, Film>) -> Vec<usize> {
    if data.require_selection {
        user.seen_films
            .iter()
            .copied()
            .filter(|id| films.contains_key(id))
            .collect()
    } else {
        let mut all: Vec<usize> = films.keys().copied().collect();
        all.sort_unstable();
        all
    }
}

pub async fn get_pair(data: web::Data<AppState>, query: web::Query<PairRequest>) -> HttpResponse {
    if data.is_banned(&query.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let films = data.films();
    // Collect user data then drop the lock before acquiring bt_ratings
    // (lock order: bt_ratings -> users; never hold users while taking bt_ratings)
    let (votes, seen, remaining) = {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(query.user_id.clone()).or_default();
        let votes = user.compared_pairs.len();

        let seen = in_play(&data, user, &films);

        if seen.len() < 2 {
            return HttpResponse::Ok().json(serde_json::json!({"done": true, "votes": votes}));
        }

        let focus = query.focus_film.filter(|f| seen.contains(f));
        let mut remaining = Vec::new();
        for i in 0..seen.len() {
            for j in (i + 1)..seen.len() {
                let pair = canonical_pair(seen[i], seen[j]);
                if !user.compared_pairs.contains(&pair)
                    && (focus.is_none() || pair.0 == focus.unwrap() || pair.1 == focus.unwrap())
                {
                    remaining.push(pair);
                }
            }
        }

        (votes, seen, remaining)
    };

    if remaining.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({"done": true, "votes": votes}));
    }

    let ratings = data.bt_ratings.lock().unwrap();
    let scores = fisher_pair_scores(&ratings, &seen, &remaining);
    drop(ratings);

    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_score = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let spread = (max_score - min_score).max(1e-10);
    let beta = 5.0;
    let weights: Vec<f64> = scores
        .iter()
        .map(|&s| (((s - min_score) / spread - 1.0) * beta).exp())
        .collect();

    let mut rng = rand::thread_rng();
    let dist = WeightedIndex::new(&weights).unwrap();
    let (a, b) = remaining[rng.sample(&dist)];

    HttpResponse::Ok().json(serde_json::json!({
        "done": false,
        "a": films[&a],
        "b": films[&b],
        "remaining": remaining.len(),
        "votes": votes,
    }))
}

pub async fn vote(data: web::Data<AppState>, payload: web::Json<VotePayload>) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let films = data.films();
    if !films.contains_key(&payload.winner_id)
        || !films.contains_key(&payload.loser_id)
        || payload.winner_id == payload.loser_id
    {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "invalid film ids"}));
    }

    let pair = canonical_pair(payload.winner_id, payload.loser_id);
    let key = pair_key(payload.winner_id, payload.loser_id);
    let outcome = payload.outcome();

    // The ratings follow from every outcome kept (lock order: bt_ratings ->
    // users -> banned).
    {
        let mut ratings = data.bt_ratings.lock().unwrap();
        let mut users = data.users.lock().unwrap();
        let user = users.entry(payload.user_id.clone()).or_default();
        if user.vote_outcomes.get(&key) == Some(&outcome) {
            return HttpResponse::Ok().json(serde_json::json!({"status": "already_voted"}));
        }
        user.compared_pairs.insert(pair);
        user.vote_outcomes.insert(key, outcome);
        let banned = data.banned.lock().unwrap();
        AppState::recompute_ratings(&mut ratings, &users, &banned);
    }

    data.save();
    let change = match outcome {
        Outcome::Won(winner) => LiveChange::Vote {
            winner,
            loser: if winner == pair.0 { pair.1 } else { pair.0 },
        },
        Outcome::Tie => LiveChange::Tie {
            a: pair.0,
            b: pair.1,
        },
    };
    data.broadcast(&payload.user_id, change);

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn unvote(data: web::Data<AppState>, payload: web::Json<UnvotePayload>) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let pair = canonical_pair(payload.winner_id, payload.loser_id);

    // Lock bt_ratings first to maintain consistent lock order (bt_ratings -> users)
    {
        let mut ratings = data.bt_ratings.lock().unwrap();
        let mut users = data.users.lock().unwrap();

        let user = match users.get_mut(&payload.user_id) {
            Some(u) => u,
            None => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "unknown user"}));
            }
        };
        if !user.compared_pairs.remove(&pair) {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "pair not found"}));
        }
        user.vote_outcomes
            .remove(&pair_key(payload.winner_id, payload.loser_id));
        let banned = data.banned.lock().unwrap();
        AppState::recompute_ratings(&mut ratings, &users, &banned);
    }

    // One vote fewer is no data loss: the floor follows.
    data.sync_vote_floor();
    data.save();
    data.broadcast(
        &payload.user_id,
        LiveChange::Unvote {
            winner: payload.winner_id,
            loser: payload.loser_id,
        },
    );

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn reset_votes(
    data: web::Data<AppState>,
    payload: web::Json<PairRequest>,
) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }

    {
        let mut ratings = data.bt_ratings.lock().unwrap();
        let mut users = data.users.lock().unwrap();
        let banned = data.banned.lock().unwrap();

        if let Some(user) = users.get_mut(&payload.user_id) {
            user.compared_pairs.clear();
            user.vote_outcomes.clear();
        }

        AppState::recompute_ratings(&mut ratings, &users, &banned);
    }

    data.sync_vote_floor();
    data.save();
    data.broadcast(&payload.user_id, LiveChange::Reset);

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn vote_stream(data: web::Data<AppState>) -> HttpResponse {
    let mut rx = data.live_tx.subscribe();
    let stream = async_stream::stream! {
        // Periodic comment forces a write so Actix can detect dead clients
        // and drop their fd; without it, idle SSE connections leak forever.
        let mut keepalive = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            tokio::select! {
                _ = keepalive.tick() => {
                    yield Ok::<_, actix_web::Error>(
                        actix_web::web::Bytes::from_static(b": keepalive\n\n")
                    );
                }
                res = rx.recv() => {
                    match res {
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
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

pub async fn stats(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();
    let banned = data.banned.lock().unwrap();

    let total_users = users.keys().filter(|uid| !banned.contains(*uid)).count();
    let active_users = users
        .iter()
        .filter(|(uid, u)| !banned.contains(*uid) && !u.compared_pairs.is_empty())
        .count();
    let total_votes: usize = users
        .iter()
        .filter(|(uid, _)| !banned.contains(*uid))
        .map(|(_, u)| u.compared_pairs.len())
        .sum();
    let films_with_votes = ratings.values().filter(|r| r.comparisons > 0).count();

    HttpResponse::Ok().json(serde_json::json!({
        "total_users": total_users,
        "active_users": active_users,
        "total_votes": total_votes,
        "total_films": data.films().len(),
        "films_with_votes": films_with_votes,
    }))
}

pub async fn user_matrix(
    data: web::Data<AppState>,
    query: web::Query<PairRequest>,
) -> HttpResponse {
    let published = data.films();
    let users = data.users.lock().unwrap();
    let newcomer = UserState::default();
    let user = users.get(&query.user_id).unwrap_or(&newcomer);

    // Every pair the user may compare, and every pair they did: a vote taken
    // back leaves its row in place, an empty cell to vote in again.
    let mut film_ids: HashSet<usize> = in_play(&data, user, &published).into_iter().collect();
    for &(a, b) in &user.compared_pairs {
        film_ids.insert(a);
        film_ids.insert(b);
    }

    let mut films: Vec<serde_json::Value> = film_ids
        .iter()
        .filter_map(|&id| published.get(&id))
        .map(|f| serde_json::json!({"id": f.id, "title": f.title}))
        .collect();
    films.sort_by_key(|f| f["id"].as_u64().unwrap_or(0));

    let votes: Vec<serde_json::Value> = user
        .vote_outcomes
        .iter()
        .filter_map(|(key, outcome)| {
            let (a, b) = parse_pair_key(key)?;
            Some(serde_json::json!({"film_a": a, "film_b": b, "outcome": outcome}))
        })
        .collect();

    let legacy_votes: Vec<serde_json::Value> = user
        .compared_pairs
        .iter()
        .filter(|&&(a, b)| !user.vote_outcomes.contains_key(&pair_key(a, b)))
        .map(|&(a, b)| serde_json::json!({"film_a": a, "film_b": b}))
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "films": films,
        "votes": votes,
        "legacy_votes": legacy_votes,
    }))
}

pub async fn global_matrix(data: web::Data<AppState>) -> HttpResponse {
    let published = data.films();
    let ratings = data.bt_ratings.lock().unwrap();

    let mut film_ids: Vec<usize> = ratings
        .values()
        .filter(|r| r.comparisons > 0)
        .map(|r| r.film_id)
        .collect();
    film_ids.sort();

    let films: Vec<serde_json::Value> = film_ids
        .iter()
        .filter_map(|&id| {
            let film = published.get(&id)?;
            let score = ratings.get(&id).map(|r| r.score).unwrap_or(1.0);
            Some(serde_json::json!({"id": film.id, "title": film.title, "score": score}))
        })
        .collect();

    let mut wins: Vec<serde_json::Value> = Vec::new();
    let mut ties: Vec<serde_json::Value> = Vec::new();
    for &id in &film_ids {
        if let Some(r) = ratings.get(&id) {
            for (&loser, &count) in &r.wins_against {
                if count > 0 {
                    wins.push(serde_json::json!({"winner": id, "loser": loser, "count": count}));
                }
            }
            // Each tie once, from the item of the smaller id.
            for (&other, &count) in &r.ties_with {
                if count > 0 && id < other {
                    ties.push(serde_json::json!({"a": id, "b": other, "count": count}));
                }
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({ "films": films, "wins": wins, "ties": ties }))
}

fn ranked_films<'a>(
    ratings: &'a HashMap<usize, BtRating>,
    users: &HashMap<String, UserState>,
    banned: &HashSet<String>,
    min_votes: u32,
    min_voters: usize,
) -> Vec<&'a BtRating> {
    let mut voters_per_film: HashMap<usize, HashSet<&String>> = HashMap::new();
    for (user_id, state) in users.iter() {
        if banned.contains(user_id) {
            continue;
        }
        for key in state.vote_outcomes.keys() {
            if let Some((a, b)) = parse_pair_key(key) {
                voters_per_film.entry(a).or_default().insert(user_id);
                voters_per_film.entry(b).or_default().insert(user_id);
            }
        }
    }

    let mut ranked: Vec<&BtRating> = ratings
        .values()
        .filter(|r| {
            r.comparisons >= min_votes
                && voters_per_film.get(&r.film_id).map_or(0, |s| s.len()) >= min_voters
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap()
            .then(a.film_id.cmp(&b.film_id))
    });
    ranked
}

pub async fn leaderboard(data: web::Data<AppState>) -> HttpResponse {
    let films = data.films();
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();
    let banned = data.banned.lock().unwrap();
    let ranked = ranked_films(&ratings, &users, &banned, data.min_votes, data.min_voters);

    let board: Vec<serde_json::Value> = ranked
        .iter()
        .filter(|r| films.contains_key(&r.film_id))
        .map(|r| {
            let film = films.get(&r.film_id);
            serde_json::json!({
                "film_id": r.film_id,
                "title": film.map(|f| f.title.as_str()).unwrap_or("?"),
                "team": film.map(|f| f.team.as_str()).unwrap_or("?"),
                "city": film.map(|f| f.city.as_str()).unwrap_or("?"),
                "poster_url": film.map(|f| f.poster_url.as_str()).unwrap_or(""),
                "video_url": film.map(|f| f.video_url.as_str()).unwrap_or(""),
                "rating": bt_score_to_display(r.score),
                "wins": r.wins(),
                "ties": r.ties(),
                "losses": r.losses(),
                "comparisons": r.comparisons,
            })
        })
        .collect();

    HttpResponse::Ok().json(board)
}

pub async fn user_contributions(
    data: web::Data<AppState>,
    query: web::Query<PairRequest>,
) -> HttpResponse {
    let users = data.users.lock().unwrap();
    let banned = data.banned.lock().unwrap();

    let mut entries: Vec<serde_json::Value> = users
        .iter()
        .filter(|(uid, state)| !banned.contains(*uid) && !state.compared_pairs.is_empty())
        .map(|(uid, state)| {
            let mut films_voted: HashSet<usize> = HashSet::new();
            for &(a, b) in &state.compared_pairs {
                films_voted.insert(a);
                films_voted.insert(b);
            }
            serde_json::json!({
                "user_id": uid,
                "is_you": *uid == query.user_id,
                "votes": state.compared_pairs.len(),
                "films_voted": films_voted.len(),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        let va = a["votes"].as_u64().unwrap_or(0);
        let vb = b["votes"].as_u64().unwrap_or(0);
        vb.cmp(&va)
    });

    for (i, entry) in entries.iter_mut().enumerate() {
        entry["rank"] = serde_json::json!(i + 1);
        if let Some(obj) = entry.as_object_mut() {
            obj.remove("user_id");
        }
    }

    HttpResponse::Ok().json(entries)
}

pub async fn leaderboard_csv(data: web::Data<AppState>) -> HttpResponse {
    let films = data.films();
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();
    let banned = data.banned.lock().unwrap();
    let ranked = ranked_films(&ratings, &users, &banned, data.min_votes, data.min_voters);

    let mut csv = String::from("Rank,Title,Team,City,Rating,Wins,Ties,Losses,Comparisons\n");
    for (i, r) in ranked
        .iter()
        .filter(|r| films.contains_key(&r.film_id))
        .enumerate()
    {
        let film = films.get(&r.film_id);
        let title = film.map(|f| f.title.as_str()).unwrap_or("?");
        let team = film.map(|f| f.team.as_str()).unwrap_or("?");
        let city = film.map(|f| f.city.as_str()).unwrap_or("?");
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",{},{},{},{},{}\n",
            i + 1,
            title.replace('"', "\"\""),
            team.replace('"', "\"\""),
            city.replace('"', "\"\""),
            bt_score_to_display(r.score),
            r.wins(),
            r.ties(),
            r.losses(),
            r.comparisons,
        ));
    }

    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(csv)
}

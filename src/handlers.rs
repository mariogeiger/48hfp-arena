use crate::bt::{BtRating, bt_score_to_display, fisher_pair_scores, run_bradley_terry};
use crate::models::*;
use crate::persistence::AppState;

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

pub async fn get_films(data: web::Data<AppState>) -> HttpResponse {
    let films: Vec<&Film> = data.films.values().collect();
    HttpResponse::Ok().json(films)
}

pub async fn set_selection(
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

pub async fn get_pair(data: web::Data<AppState>, query: web::Query<PairRequest>) -> HttpResponse {
    // Collect user data then drop the lock before acquiring bt_ratings
    // (lock order: bt_ratings -> users; never hold users while taking bt_ratings)
    let (votes, seen, remaining) = {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(query.user_id.clone()).or_default();
        let votes = user.compared_pairs.len();

        if user.seen_films.len() < 2 {
            return HttpResponse::Ok().json(serde_json::json!({"done": true, "votes": votes}));
        }

        let seen = user.seen_films.clone();
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
        "a": data.films[&a],
        "b": data.films[&b],
        "remaining": remaining.len(),
        "votes": votes,
    }))
}

pub async fn vote(data: web::Data<AppState>, payload: web::Json<VotePayload>) -> HttpResponse {
    if !data.films.contains_key(&payload.winner_id)
        || !data.films.contains_key(&payload.loser_id)
        || payload.winner_id == payload.loser_id
    {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "invalid film ids"}));
    }

    let pair = canonical_pair(payload.winner_id, payload.loser_id);
    let key = pair_key(payload.winner_id, payload.loser_id);

    let prev_winner: Option<usize> = {
        let users = data.users.lock().unwrap();
        if let Some(user) = users.get(&payload.user_id) {
            if user.compared_pairs.contains(&pair) {
                if user.vote_outcomes.get(&key) == Some(&payload.winner_id) {
                    return HttpResponse::Ok().json(serde_json::json!({"status": "already_voted"}));
                }
                user.vote_outcomes.get(&key).copied()
            } else {
                None
            }
        } else {
            None
        }
    };

    {
        let mut ratings = data.bt_ratings.lock().unwrap();

        if let Some(prev) = prev_winner {
            let prev_loser = if prev == pair.0 { pair.1 } else { pair.0 };
            if let Some(w) = ratings.get_mut(&prev) {
                w.comparisons = w.comparisons.saturating_sub(1);
                let entry = w.wins_against.entry(prev_loser).or_insert(0);
                *entry = entry.saturating_sub(1);
            }
            if let Some(l) = ratings.get_mut(&prev_loser) {
                l.comparisons = l.comparisons.saturating_sub(1);
            }
        }

        let w = ratings
            .entry(payload.winner_id)
            .or_insert_with(|| BtRating::new(payload.winner_id));
        w.comparisons += 1;
        *w.wins_against.entry(payload.loser_id).or_insert(0) += 1;

        let l = ratings
            .entry(payload.loser_id)
            .or_insert_with(|| BtRating::new(payload.loser_id));
        l.comparisons += 1;

        run_bradley_terry(&mut ratings);
    }

    {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(payload.user_id.clone()).or_default();
        user.compared_pairs.insert(pair);
        user.vote_outcomes.insert(key, payload.winner_id);
    }

    data.save();

    let winner_title = data
        .films
        .get(&payload.winner_id)
        .map(|f| f.title.clone())
        .unwrap_or_default();
    let loser_title = data
        .films
        .get(&payload.loser_id)
        .map(|f| f.title.clone())
        .unwrap_or_default();
    let _ = data.vote_tx.send(VoteEvent {
        user_id: payload.user_id.clone(),
        winner_title,
        loser_title,
    });

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn unvote(data: web::Data<AppState>, payload: web::Json<UnvotePayload>) -> HttpResponse {
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

pub async fn vote_stream(data: web::Data<AppState>) -> HttpResponse {
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

pub async fn stats(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();

    let total_users = users.len();
    let active_users = users
        .values()
        .filter(|u| !u.compared_pairs.is_empty())
        .count();
    let total_votes: usize = users.values().map(|u| u.compared_pairs.len()).sum();
    let films_with_votes = ratings.values().filter(|r| r.comparisons > 0).count();

    HttpResponse::Ok().json(serde_json::json!({
        "total_users": total_users,
        "active_users": active_users,
        "total_votes": total_votes,
        "total_films": data.films.len(),
        "films_with_votes": films_with_votes,
    }))
}

pub async fn user_matrix(
    data: web::Data<AppState>,
    query: web::Query<PairRequest>,
) -> HttpResponse {
    let users = data.users.lock().unwrap();
    let user = match users.get(&query.user_id) {
        Some(u) => u,
        None => {
            return HttpResponse::Ok()
                .json(serde_json::json!({"films": [], "votes": [], "legacy_votes": []}));
        }
    };

    let mut film_ids: HashSet<usize> = HashSet::new();
    for key in user.vote_outcomes.keys() {
        if let Some((a, b)) = parse_pair_key(key) {
            film_ids.insert(a);
            film_ids.insert(b);
        }
    }
    for &(a, b) in &user.compared_pairs {
        if !user.vote_outcomes.contains_key(&pair_key(a, b)) {
            film_ids.insert(a);
            film_ids.insert(b);
        }
    }

    let mut films: Vec<serde_json::Value> = film_ids
        .iter()
        .filter_map(|&id| data.films.get(&id))
        .map(|f| serde_json::json!({"id": f.id, "title": f.title}))
        .collect();
    films.sort_by_key(|f| f["id"].as_u64().unwrap_or(0));

    let votes: Vec<serde_json::Value> = user
        .vote_outcomes
        .iter()
        .filter_map(|(key, &winner)| {
            let (a, b) = parse_pair_key(key)?;
            Some(serde_json::json!({"film_a": a, "film_b": b, "winner": winner}))
        })
        .collect();

    let legacy_votes: Vec<serde_json::Value> = user
        .compared_pairs
        .iter()
        .filter(|&&(a, b)| !user.vote_outcomes.contains_key(&pair_key(a, b)))
        .map(|&(a, b)| serde_json::json!({"film_a": a, "film_b": b, "winner": null}))
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "films": films,
        "votes": votes,
        "legacy_votes": legacy_votes,
    }))
}

pub async fn global_matrix(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.bt_ratings.lock().unwrap();

    let mut film_ids: Vec<usize> = ratings
        .values()
        .filter(|r| r.comparisons > 0)
        .map(|r| r.film_id)
        .collect();
    film_ids.sort();

    let films: Vec<serde_json::Value> = film_ids
        .iter()
        .filter_map(|&id| data.films.get(&id))
        .map(|f| serde_json::json!({"id": f.id, "title": f.title}))
        .collect();

    let mut wins: Vec<serde_json::Value> = Vec::new();
    for &id in &film_ids {
        if let Some(r) = ratings.get(&id) {
            for (&loser, &count) in &r.wins_against {
                if count > 0 {
                    wins.push(serde_json::json!({"winner": id, "loser": loser, "count": count}));
                }
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({ "films": films, "wins": wins }))
}

const MIN_VOTES: u32 = 10;
const MIN_VOTERS: usize = 2;

fn ranked_films<'a>(
    ratings: &'a HashMap<usize, BtRating>,
    users: &HashMap<String, UserState>,
) -> Vec<&'a BtRating> {
    let mut voters_per_film: HashMap<usize, HashSet<&String>> = HashMap::new();
    for (user_id, state) in users.iter() {
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
            r.comparisons >= MIN_VOTES
                && voters_per_film.get(&r.film_id).map_or(0, |s| s.len()) >= MIN_VOTERS
        })
        .collect();
    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    ranked
}

pub async fn leaderboard(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();
    let ranked = ranked_films(&ratings, &users);

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
                "video_url": film.map(|f| f.video_url.as_str()).unwrap_or(""),
                "rating": bt_score_to_display(r.score),
                "wins": r.wins(),
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

    let mut entries: Vec<serde_json::Value> = users
        .iter()
        .filter(|(_, state)| !state.compared_pairs.is_empty())
        .map(|(uid, state)| {
            serde_json::json!({
                "user_id": uid,
                "is_you": *uid == query.user_id,
                "films_selected": state.seen_films.len(),
                "votes": state.compared_pairs.len(),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        let va = a["votes"].as_u64().unwrap_or(0);
        let vb = b["votes"].as_u64().unwrap_or(0);
        vb.cmp(&va).then_with(|| {
            let fa = a["films_selected"].as_u64().unwrap_or(0);
            let fb = b["films_selected"].as_u64().unwrap_or(0);
            fb.cmp(&fa)
        })
    });

    for (i, entry) in entries.iter_mut().enumerate() {
        let is_you = entry["is_you"].as_bool().unwrap_or(false);
        entry["label"] = serde_json::json!(if is_you {
            "You".to_string()
        } else {
            format!("User {}", i + 1)
        });
        if let Some(obj) = entry.as_object_mut() {
            obj.remove("user_id");
        }
    }

    HttpResponse::Ok().json(entries)
}

pub async fn leaderboard_csv(data: web::Data<AppState>) -> HttpResponse {
    let ratings = data.bt_ratings.lock().unwrap();
    let users = data.users.lock().unwrap();
    let ranked = ranked_films(&ratings, &users);

    let mut csv = String::from("Rank,Title,Team,City,Rating,Wins,Losses,Comparisons\n");
    for (i, r) in ranked.iter().enumerate() {
        let film = data.films.get(&r.film_id);
        let title = film.map(|f| f.title.as_str()).unwrap_or("?");
        let team = film.map(|f| f.team.as_str()).unwrap_or("?");
        let city = film.map(|f| f.city.as_str()).unwrap_or("?");
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",{},{},{},{}\n",
            i + 1,
            title.replace('"', "\"\""),
            team.replace('"', "\"\""),
            city.replace('"', "\"\""),
            bt_score_to_display(r.score),
            r.wins(),
            r.losses(),
            r.comparisons,
        ));
    }

    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(csv)
}

fn parse_pair_key(key: &str) -> Option<(usize, usize)> {
    let (a, b) = key.split_once(',')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

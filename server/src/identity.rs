//! An identity is the UUID a browser keeps; whoever holds it votes as it.
//! Here an identity gets a private name, so that a link carrying it can ask
//! "are you Tom?", and one identity folds into another when a device takes
//! up the session of another.

use crate::models::UserState;
use crate::persistence::AppState;
use versus_shared::{MergePayload, NamePayload, UserQuery, pair_key};

use actix_web::{HttpResponse, web};

const MAX_NAME_CHARS: usize = 40;

/// The identity's name and vote count, for whoever holds it.
pub async fn identity(data: web::Data<AppState>, query: web::Query<UserQuery>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    match users.get(&query.user_id) {
        Some(user) => HttpResponse::Ok().json(serde_json::json!({
            "name": user.name,
            "votes": user.compared_pairs.len(),
        })),
        None => HttpResponse::NotFound().json(serde_json::json!({"error": "unknown_user"})),
    }
}

pub async fn set_name(data: web::Data<AppState>, payload: web::Json<NamePayload>) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let name = payload.name.trim();
    if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "bad_text"}));
    }
    {
        let mut users = data.users.lock().unwrap();
        let user = users.entry(payload.user_id.clone()).or_default();
        user.name = (!name.is_empty()).then(|| name.to_string());
    }
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

/// Fold the identity `user_id` into `into` and forget it. `into` keeps its
/// votes and gains the pairs only `user_id` compared; their selections join.
/// Answers with the selection of the merged identity.
pub async fn merge(data: web::Data<AppState>, payload: web::Json<MergePayload>) -> HttpResponse {
    if data.is_banned(&payload.user_id) || data.is_banned(&payload.into) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let selection = {
        // Lock order: bt_ratings -> users.
        let mut ratings = data.bt_ratings.lock().unwrap();
        let mut users = data.users.lock().unwrap();
        if !users.contains_key(&payload.into) {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "unknown_user"}));
        }
        let from = if payload.user_id == payload.into {
            UserState::default()
        } else {
            users.remove(&payload.user_id).unwrap_or_default()
        };
        let into = users.get_mut(&payload.into).unwrap();
        fold(into, from);
        let selection = into.seen_films.clone();
        let banned = data.banned.lock().unwrap();
        AppState::recompute_ratings(&mut ratings, &users, &banned);
        selection
    };
    // A pair both identities compared now counts once.
    data.sync_vote_floor();
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"selection": selection}))
}

/// `into` with what only `from` has: the pairs it compared, with their
/// outcomes, the items it selected, and its name if `into` has none.
fn fold(into: &mut UserState, from: UserState) {
    for pair in from.compared_pairs {
        if into.compared_pairs.insert(pair) {
            let key = pair_key(pair.0, pair.1);
            if let Some(&winner) = from.vote_outcomes.get(&key) {
                into.vote_outcomes.insert(key, winner);
            }
        }
    }
    for id in from.seen_films {
        if !into.seen_films.contains(&id) {
            into.seen_films.push(id);
        }
    }
    if into.name.is_none() {
        into.name = from.name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter(votes: &[(usize, usize, usize)], seen: &[usize], name: Option<&str>) -> UserState {
        let mut user = UserState {
            seen_films: seen.to_vec(),
            name: name.map(str::to_string),
            ..UserState::default()
        };
        for &(a, b, winner) in votes {
            user.compared_pairs
                .insert(versus_shared::canonical_pair(a, b));
            user.vote_outcomes.insert(pair_key(a, b), winner);
        }
        user
    }

    #[test]
    fn fold_keeps_the_votes_of_into_and_adds_the_others() {
        let mut into = voter(&[(1, 2, 1), (2, 3, 3)], &[1, 2, 3], Some("Tom"));
        let from = voter(&[(2, 1, 2), (3, 4, 4)], &[3, 4], Some("Laptop"));
        fold(&mut into, from);
        assert_eq!(into.compared_pairs.len(), 3);
        assert_eq!(into.vote_outcomes[&pair_key(1, 2)], 1);
        assert_eq!(into.vote_outcomes[&pair_key(2, 3)], 3);
        assert_eq!(into.vote_outcomes[&pair_key(3, 4)], 4);
        assert_eq!(into.seen_films, vec![1, 2, 3, 4]);
        assert_eq!(into.name.as_deref(), Some("Tom"));
    }

    #[test]
    fn fold_takes_the_name_into_lacks() {
        let mut into = voter(&[], &[], None);
        fold(&mut into, voter(&[], &[], Some("Tom")));
        assert_eq!(into.name.as_deref(), Some("Tom"));
    }
}

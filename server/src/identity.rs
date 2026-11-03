//! An identity is the UUID a browser keeps; whoever holds it votes as it.
//! Here an identity gets a private name, so that a link carrying it can ask
//! "are you Tom?", and a Google account, so that signing in finds it again.
//! One identity folds into another when a device takes up another's session.

use crate::google::Google;
use crate::models::UserState;
use crate::persistence::AppState;
use versus_shared::{GooglePayload, MergePayload, NamePayload, UserQuery, pair_key};

use actix_web::{HttpResponse, web};
use std::collections::HashMap;

const MAX_NAME_CHARS: usize = 40;

/// The identity's name and vote count, for whoever holds it.
pub async fn identity(data: web::Data<AppState>, query: web::Query<UserQuery>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    match users.get(&query.user_id) {
        Some(user) => HttpResponse::Ok().json(serde_json::json!({
            "name": user.name,
            "votes": user.compared_pairs.len(),
            "google": user.google.as_ref().map(|g| serde_json::json!({
                "email": g.email,
                "name": g.name,
                "picture": g.picture,
            })),
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

/// Fold the identity `user_id` into `into`, which a link offered. Answers
/// with the selection of the merged identity.
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
        absorb(&mut users, &payload.user_id, &payload.into);
        let banned = data.banned.lock().unwrap();
        AppState::recompute_ratings(&mut ratings, &users, &banned);
        users[&payload.into].seen_films.clone()
    };
    // A pair both identities compared now counts once.
    data.sync_vote_floor();
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"selection": selection}))
}

/// Sign in with the Google account an ID token names. The identity that
/// account already signs in as is the one to continue, and this device's
/// identity folds into it unless another account holds it; an account seen
/// for the first time takes this device's identity, or a new one if another
/// account holds it. Answers with the identity to continue and its selection.
pub async fn sign_in_with_google(
    data: web::Data<AppState>,
    google: web::Data<Google>,
    payload: web::Json<GooglePayload>,
) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let account = match google.verify(&payload.credential) {
        Ok(account) => account,
        Err(code) => {
            log::warn!("Google sign-in refused: {code}");
            return HttpResponse::Unauthorized().json(serde_json::json!({"error": code}));
        }
    };
    let (user_id, selection) = {
        // Lock order: bt_ratings -> users.
        let mut ratings = data.bt_ratings.lock().unwrap();
        let mut users = data.users.lock().unwrap();
        let owner = users
            .iter()
            .find(|(_, u)| u.google.as_ref().is_some_and(|g| g.sub == account.sub))
            .map(|(id, _)| id.clone());
        let device_free = users
            .get(&payload.user_id)
            .is_none_or(|u| u.google.is_none());
        let user_id = match owner {
            Some(owner) => {
                if data.is_banned(&owner) {
                    return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
                }
                if device_free {
                    absorb(&mut users, &payload.user_id, &owner);
                }
                owner
            }
            None if device_free => payload.user_id.clone(),
            None => uuid::Uuid::new_v4().to_string(),
        };
        let user = users.entry(user_id.clone()).or_default();
        // A link to this identity asks "are you <name>?": Google names it.
        if user.name.is_none() {
            let name = [&account.given_name, &account.name]
                .into_iter()
                .find(|n| !n.trim().is_empty())
                .map(|n| n.trim().chars().take(MAX_NAME_CHARS).collect());
            user.name = name;
        }
        // The e-mail, name and picture may have changed since.
        user.google = Some(account);
        let selection = user.seen_films.clone();
        let banned = data.banned.lock().unwrap();
        AppState::recompute_ratings(&mut ratings, &users, &banned);
        (user_id, selection)
    };
    data.sync_vote_floor();
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"user_id": user_id, "selection": selection}))
}

/// Fold the identity `from` into `into`, which it becomes, and forget it.
fn absorb(users: &mut HashMap<String, UserState>, from: &str, into: &str) {
    if from == into {
        return;
    }
    let from = users.remove(from).unwrap_or_default();
    fold(users.entry(into.to_string()).or_default(), from);
}

/// `into` with what only `from` has: the pairs it compared, with their
/// outcomes, the items it selected, and its name and Google account if
/// `into` has none.
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
    if into.google.is_none() {
        into.google = from.google;
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

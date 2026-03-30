use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Film {
    pub id: usize,
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub video_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserState {
    pub seen_films: Vec<usize>,
    pub compared_pairs: HashSet<(usize, usize)>,
    #[serde(default)]
    pub vote_outcomes: HashMap<String, usize>, // "min,max" -> winner_id
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentData {
    pub bt_ratings: HashMap<usize, crate::bt::BtRating>,
    pub users: HashMap<String, UserState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoteEvent {
    pub user_id: String,
    pub winner_title: String,
    pub loser_title: String,
}

#[derive(Deserialize)]
pub struct SelectionPayload {
    pub user_id: String,
    pub film_ids: Vec<usize>,
}

#[derive(Deserialize)]
pub struct VotePayload {
    pub user_id: String,
    pub winner_id: usize,
    pub loser_id: usize,
}

#[derive(Deserialize)]
pub struct PairRequest {
    pub user_id: String,
    pub focus_film: Option<usize>,
}

#[derive(Deserialize)]
pub struct UnvotePayload {
    pub user_id: String,
    pub winner_id: usize,
    pub loser_id: usize,
}

pub fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

pub fn pair_key(a: usize, b: usize) -> String {
    let (lo, hi) = canonical_pair(a, b);
    format!("{},{}", lo, hi)
}

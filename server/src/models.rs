use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::bt::BtRating;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserState {
    pub seen_films: Vec<usize>,
    pub compared_pairs: HashSet<(usize, usize)>,
    #[serde(default)]
    pub vote_outcomes: HashMap<String, usize>, // "min,max" -> winner_id
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentData {
    pub bt_ratings: HashMap<usize, BtRating>,
    pub users: HashMap<String, UserState>,
}

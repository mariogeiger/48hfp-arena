use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::bt::BtRating;
use crate::google::GoogleAccount;
use versus_shared::Outcome;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserState {
    pub seen_films: Vec<usize>,
    pub compared_pairs: HashSet<(usize, usize)>,
    #[serde(default)]
    /// "min,max" → what the duel came to. A compared pair without one was
    /// compared before outcomes were recorded.
    pub vote_outcomes: HashMap<String, Outcome>,
    /// The name the user gave to recognize their identity on another device;
    /// private, shown only to whoever holds the identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The Google account that signs in as this identity, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleAccount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentData {
    pub bt_ratings: HashMap<usize, BtRating>,
    pub users: HashMap<String, UserState>,
}

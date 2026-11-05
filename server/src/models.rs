use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::bt::BtRating;
use crate::google::GoogleAccount;
use versus_shared::{Film, Outcome};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserState {
    /// The items the user picked, on a site where no item is picked by
    /// default.
    pub seen_films: Vec<usize>,
    /// On a site where every item is picked by default, the items the user
    /// took out; none until they first choose. Items added later are not in
    /// it, so they join the selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_films: Option<Vec<usize>>,
    pub compared_pairs: HashSet<(usize, usize)>,
    /// "min,max" → the vote on that duel. A compared pair without one was
    /// compared before outcomes were recorded.
    #[serde(default, alias = "vote_outcomes")]
    pub votes: HashMap<String, Vote>,
    /// The name the user gave to recognize their identity on another device;
    /// private, shown only to whoever holds the identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The Google account that signs in as this identity, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleAccount>,
    /// The name the list of who voted most shows for this identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<Nickname>,
}

impl UserState {
    /// The items among `published` that the user compares, by increasing id.
    pub fn selection(&self, published: &HashMap<usize, Film>, by_default: bool) -> Vec<usize> {
        let mut ids: Vec<usize> = if by_default {
            let skipped = self.skipped_films.as_deref().unwrap_or_default();
            published
                .keys()
                .copied()
                .filter(|id| !skipped.contains(id))
                .collect()
        } else {
            self.seen_films
                .iter()
                .copied()
                .filter(|id| published.contains_key(id))
                .collect()
        };
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Make `ids` the items the user compares.
    pub fn select(&mut self, ids: Vec<usize>, published: &HashMap<usize, Film>, by_default: bool) {
        if by_default {
            let mut skipped: Vec<usize> = published
                .keys()
                .copied()
                .filter(|id| !ids.contains(id))
                .collect();
            skipped.sort_unstable();
            self.skipped_films = Some(skipped);
            self.seen_films.clear();
        } else {
            self.seen_films = ids;
        }
    }

    /// Fold in the selection another identity kept, which the user takes
    /// up: the items either compares.
    pub fn join_selection(&mut self, seen: Vec<usize>, skipped: Option<Vec<usize>>) {
        for id in seen {
            if !self.seen_films.contains(&id) {
                self.seen_films.push(id);
            }
        }
        // Not having chosen is having taken nothing out.
        self.skipped_films = match (self.skipped_films.take(), skipped) {
            (Some(mine), Some(theirs)) => {
                Some(mine.into_iter().filter(|id| theirs.contains(id)).collect())
            }
            _ => None,
        };
    }
}

/// A vote kept: what the duel came to and, for a vote cast since votes are
/// dated, when, in seconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "KeptVote")]
pub struct Vote {
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<u64>,
}

impl Vote {
    /// The vote `outcome`, cast now.
    pub fn now(outcome: Outcome) -> Self {
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .ok();
        Self { outcome, at }
    }
}

/// A vote as stored: dated, or a bare outcome from before votes were dated.
#[derive(Deserialize)]
#[serde(untagged)]
enum KeptVote {
    Dated {
        outcome: Outcome,
        #[serde(default)]
        at: Option<u64>,
    },
    Bare(Outcome),
}

impl From<KeptVote> for Vote {
    fn from(kept: KeptVote) -> Self {
        match kept {
            KeptVote::Dated { outcome, at } => Self { outcome, at },
            KeptVote::Bare(outcome) => Self { outcome, at: None },
        }
    }
}

/// A public nickname: drawn at random, or written by its owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nickname {
    pub text: String,
    #[serde(default)]
    pub written: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentData {
    pub bt_ratings: HashMap<usize, BtRating>,
    pub users: HashMap<String, UserState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use versus_shared::FilmFields;

    #[test]
    fn votes_read_bare_outcomes_from_before_they_were_dated() {
        let user: UserState = serde_json::from_str(
            r#"{"seen_films": [], "compared_pairs": [[1, 2], [2, 3]],
                "vote_outcomes": {"1,2": 2, "2,3": "tie"}}"#,
        )
        .unwrap();
        assert_eq!(
            user.votes["1,2"],
            Vote {
                outcome: Outcome::Won(2),
                at: None
            }
        );
        assert_eq!(user.votes["2,3"].outcome, Outcome::Tie);
        let dated = Vote {
            outcome: Outcome::Tie,
            at: Some(7),
        };
        let json = serde_json::to_string(&dated).unwrap();
        assert_eq!(serde_json::from_str::<Vote>(&json).unwrap(), dated);
        let undated = Vote {
            outcome: Outcome::Won(3),
            at: None,
        };
        let json = serde_json::to_string(&undated).unwrap();
        assert_eq!(serde_json::from_str::<Vote>(&json).unwrap(), undated);
    }

    fn published(ids: &[usize]) -> HashMap<usize, Film> {
        ids.iter()
            .map(|&id| (id, FilmFields::default().into_film(id)))
            .collect()
    }

    #[test]
    fn a_site_that_picks_by_default_keeps_what_is_taken_out() {
        let mut user = UserState::default();
        assert_eq!(user.selection(&published(&[1, 2, 3]), true), vec![1, 2, 3]);
        user.select(vec![1, 3], &published(&[1, 2, 3]), true);
        assert_eq!(user.selection(&published(&[1, 2, 3]), true), vec![1, 3]);
        // An item added later joins the selection.
        assert_eq!(
            user.selection(&published(&[1, 2, 3, 4]), true),
            vec![1, 3, 4]
        );
    }

    #[test]
    fn a_site_that_picks_nothing_by_default_keeps_what_is_picked() {
        let mut user = UserState::default();
        assert!(user.selection(&published(&[1, 2, 3]), false).is_empty());
        user.select(vec![3, 1], &published(&[1, 2, 3]), false);
        assert_eq!(user.selection(&published(&[1, 2, 3, 4]), false), vec![1, 3]);
        // An item withdrawn from the catalog is no longer in it.
        assert_eq!(user.selection(&published(&[1, 2]), false), vec![1]);
    }

    #[test]
    fn joined_selections_hold_what_either_holds() {
        let all = published(&[1, 2, 3, 4]);
        let mut mine = UserState::default();
        mine.select(vec![1, 2], &all, true);
        mine.join_selection(Vec::new(), Some(vec![1, 4]));
        assert_eq!(mine.selection(&all, true), vec![1, 2, 3]);
        // An identity that never chose has everything.
        mine.join_selection(Vec::new(), None);
        assert_eq!(mine.selection(&all, true), vec![1, 2, 3, 4]);
    }
}

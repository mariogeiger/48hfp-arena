use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::bt::BtRating;
use crate::google::GoogleAccount;
use versus_shared::Outcome;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserState {
    pub seen_films: Vec<usize>,
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
}

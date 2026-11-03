use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Film {
    pub id: usize,
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub video_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub synopsis: String,
}

impl Film {
    pub fn fields(&self) -> FilmFields {
        FilmFields {
            title: self.title.clone(),
            team: self.team.clone(),
            city: self.city.clone(),
            poster_url: self.poster_url.clone(),
            video_url: self.video_url.clone(),
            synopsis: self.synopsis.clone(),
        }
    }
}

/// Everything about a film that people may edit: a film without its id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilmFields {
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    pub video_url: String,
    /// Several lines of free text; revisions written before it existed read
    /// it as empty.
    #[serde(default)]
    pub synopsis: String,
}

impl FilmFields {
    pub fn into_film(self, id: usize) -> Film {
        Film {
            id,
            title: self.title,
            team: self.team,
            city: self.city,
            poster_url: self.poster_url,
            video_url: self.video_url,
            synopsis: self.synopsis,
        }
    }
}

/// What the live stream tells every client, which words it in its language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEvent {
    pub user_id: String,
    #[serde(flatten)]
    pub change: LiveChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveChange {
    Vote {
        winner: usize,
        loser: usize,
    },
    /// Neither of the two won.
    Tie {
        a: usize,
        b: usize,
    },
    Unvote {
        winner: usize,
        loser: usize,
    },
    Reset,
    /// A film was edited, proposed or reviewed.
    Film {
        film: usize,
        change: FilmChange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilmChange {
    Edited,
    Proposed,
    Approved,
    Rejected,
}

impl Change {
    pub fn film_change(&self) -> FilmChange {
        match self {
            Change::Edit { .. } => FilmChange::Edited,
            Change::Propose { .. } => FilmChange::Proposed,
            Change::Review { approve: true } => FilmChange::Approved,
            Change::Review { approve: false } => FilmChange::Rejected,
        }
    }
}

#[derive(Deserialize)]
pub struct SelectionPayload {
    pub user_id: String,
    pub film_ids: Vec<usize>,
}

/// A vote: `winner_id` beat `loser_id`, or with `tie` neither won.
#[derive(Deserialize)]
pub struct VotePayload {
    pub user_id: String,
    pub winner_id: usize,
    pub loser_id: usize,
    #[serde(default)]
    pub tie: bool,
}

impl VotePayload {
    pub fn outcome(&self) -> Outcome {
        if self.tie {
            Outcome::Tie
        } else {
            Outcome::Won(self.winner_id)
        }
    }
}

/// What a duel came to: the item that won it, or a tie, which counts as half
/// a win to each. Kept as the winner's id, or the word `"tie"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "KeptOutcome", into = "KeptOutcome")]
pub enum Outcome {
    Won(usize),
    Tie,
}

impl Outcome {
    /// What the item `id` of the pair scored: 1 for a win, ½ for a tie, 0
    /// for a loss.
    pub fn score_of(self, id: usize) -> f64 {
        match self {
            Outcome::Won(winner) if winner == id => 1.0,
            Outcome::Won(_) => 0.0,
            Outcome::Tie => 0.5,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum KeptOutcome {
    Won(usize),
    Word(String),
}

impl From<Outcome> for KeptOutcome {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Won(id) => KeptOutcome::Won(id),
            Outcome::Tie => KeptOutcome::Word("tie".into()),
        }
    }
}

impl TryFrom<KeptOutcome> for Outcome {
    type Error = String;
    fn try_from(kept: KeptOutcome) -> Result<Self, String> {
        match kept {
            KeptOutcome::Won(id) => Ok(Outcome::Won(id)),
            KeptOutcome::Word(word) if word == "tie" => Ok(Outcome::Tie),
            KeptOutcome::Word(word) => Err(format!("unknown outcome {word:?}")),
        }
    }
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

#[derive(Deserialize)]
pub struct UserQuery {
    pub user_id: String,
}

/// The name a user gives their identity; empty forgets it.
#[derive(Deserialize)]
pub struct NamePayload {
    pub user_id: String,
    pub name: String,
}

/// Fold the identity `user_id` into the identity `into`, which goes on.
#[derive(Deserialize)]
pub struct MergePayload {
    pub user_id: String,
    pub into: String,
}

/// An ID token Google gave the browser of `user_id`.
#[derive(Deserialize)]
pub struct GooglePayload {
    pub user_id: String,
    pub credential: String,
}

#[derive(Deserialize)]
pub struct EditPayload {
    pub user_id: String,
    pub fields: FilmFields,
}

/// Restore a film's fields as revision `rev` left them; 0 is the original.
#[derive(Deserialize)]
pub struct RevertPayload {
    pub user_id: String,
    pub rev: u64,
}

#[derive(Deserialize)]
pub struct ProposePayload {
    pub user_id: String,
    pub fields: FilmFields,
}

#[derive(Deserialize)]
pub struct ReviewPayload {
    pub user_id: String,
    pub approve: bool,
}

/// One revision of a film as the history shows it: its author is a short
/// anonymous tag, never the user id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub rev: u64,
    pub time: u64,
    pub author: String,
    pub is_you: bool,
    #[serde(flatten)]
    pub change: Change,
}

/// A change to the catalog. `Edit` and `Propose` carry the film's complete
/// fields, so any revision can be restored as it stands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Edit {
        fields: FilmFields,
        /// The revision this edit restored, 0 being the original fields.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revert_of: Option<u64>,
    },
    Propose {
        fields: FilmFields,
    },
    Review {
        approve: bool,
    },
}

/// A proposed film awaiting its review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub film: Film,
    pub author: String,
    pub time: u64,
    pub is_yours: bool,
}

pub fn canonical_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

pub fn pair_key(a: usize, b: usize) -> String {
    let (lo, hi) = canonical_pair(a, b);
    format!("{},{}", lo, hi)
}

pub fn parse_pair_key(key: &str) -> Option<(usize, usize)> {
    let (a, b) = key.split_once(',')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcomes_keep_as_a_winner_or_the_word_tie() {
        assert_eq!(serde_json::to_string(&Outcome::Won(7)).unwrap(), "7");
        assert_eq!(serde_json::to_string(&Outcome::Tie).unwrap(), "\"tie\"");
        assert_eq!(
            serde_json::from_str::<Outcome>("7").unwrap(),
            Outcome::Won(7)
        );
        assert_eq!(
            serde_json::from_str::<Outcome>("\"tie\"").unwrap(),
            Outcome::Tie
        );
        assert!(serde_json::from_str::<Outcome>("\"draw\"").is_err());
        assert_eq!(Outcome::Won(7).score_of(7), 1.0);
        assert_eq!(Outcome::Won(7).score_of(3), 0.0);
        assert_eq!(Outcome::Tie.score_of(3), 0.5);
    }
}

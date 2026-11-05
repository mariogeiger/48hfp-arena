use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Film {
    pub id: usize,
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub videos: Vec<Video>,
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
            videos: self.videos.clone(),
            synopsis: self.synopsis.clone(),
        }
    }
}

/// One version of a film (the original, a director's cut, a subtitled
/// one…): its name, which may stay empty when it is the film's only one, and
/// its link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Video {
    pub label: String,
    pub url: String,
}

impl Video {
    /// The versions written one per line as `label url`, the link being the
    /// line's last word since a link holds no space; a bare link is a version
    /// without a name, and blank lines are skipped.
    pub fn parse_lines(text: &str) -> Vec<Video> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| match line.rsplit_once(char::is_whitespace) {
                Some((label, url)) => Video {
                    label: label.trim().into(),
                    url: url.into(),
                },
                None => Video {
                    label: String::new(),
                    url: line.into(),
                },
            })
            .collect()
    }

    /// The inverse of `parse_lines`.
    pub fn to_lines(videos: &[Video]) -> String {
        videos
            .iter()
            .map(|v| match v.label.as_str() {
                "" => v.url.clone(),
                label => format!("{label} {}", v.url),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Everything about a film that people may edit: a film without its id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "KeptFields")]
pub struct FilmFields {
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    /// In the order people chose, the first one opening by default.
    pub videos: Vec<Video>,
    /// Several lines of free text.
    pub synopsis: String,
}

/// A field of a film, to treat them all alike as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Team,
    City,
    Poster,
    Videos,
    Synopsis,
}

impl FilmFields {
    /// `field` as text, its versions one per line as `Video::to_lines`
    /// writes them.
    pub fn text(&self, field: Field) -> String {
        match field {
            Field::Title => self.title.clone(),
            Field::Team => self.team.clone(),
            Field::City => self.city.clone(),
            Field::Poster => self.poster_url.clone(),
            Field::Videos => Video::to_lines(&self.videos),
            Field::Synopsis => self.synopsis.clone(),
        }
    }
}

/// `FilmFields` as the journal keeps them, which is never rewritten: a
/// revision written before a film had versions holds its one link as
/// `video_url`, and one written before synopses has none.
#[derive(Deserialize)]
struct KeptFields {
    title: String,
    team: String,
    city: String,
    poster_url: String,
    #[serde(default)]
    videos: Vec<Video>,
    #[serde(default)]
    video_url: String,
    #[serde(default)]
    synopsis: String,
}

impl From<KeptFields> for FilmFields {
    fn from(k: KeptFields) -> Self {
        let mut videos = k.videos;
        if videos.is_empty() && !k.video_url.is_empty() {
            videos.push(Video {
                label: String::new(),
                url: k.video_url,
            });
        }
        FilmFields {
            title: k.title,
            team: k.team,
            city: k.city,
            poster_url: k.poster_url,
            videos,
            synopsis: k.synopsis,
        }
    }
}

impl FilmFields {
    pub fn into_film(self, id: usize) -> Film {
        Film {
            id,
            title: self.title,
            team: self.team,
            city: self.city,
            poster_url: self.poster_url,
            videos: self.videos,
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
    /// The pairs the viewer passed on lately, oldest first, as `pair_list`
    /// writes them: drawn only when no other pair is left.
    #[serde(default)]
    pub avoid: String,
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

/// Pairs in order, as their keys joined by dots, which an address keeps
/// as they are.
pub fn pair_list(pairs: impl IntoIterator<Item = (usize, usize)>) -> String {
    pairs
        .into_iter()
        .map(|(a, b)| pair_key(a, b))
        .collect::<Vec<_>>()
        .join(".")
}

/// The pairs `pair_list` wrote, each canonical, skipping what is no pair.
pub fn parse_pair_list(list: &str) -> Vec<(usize, usize)> {
    list.split('.')
        .filter_map(parse_pair_key)
        .map(|(a, b)| canonical_pair(a, b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_read_one_per_line_with_the_link_last() {
        let videos = Video::parse_lines(" Original 48h  https://a.ch/1 \n\nhttps://b.ch/2\r\n");
        assert_eq!(
            videos,
            vec![
                Video {
                    label: "Original 48h".into(),
                    url: "https://a.ch/1".into()
                },
                Video {
                    label: String::new(),
                    url: "https://b.ch/2".into()
                },
            ]
        );
        assert_eq!(
            Video::to_lines(&videos),
            "Original 48h https://a.ch/1\nhttps://b.ch/2"
        );
        assert_eq!(Video::parse_lines(&Video::to_lines(&videos)), videos);
        assert!(Video::parse_lines("").is_empty());
    }

    #[test]
    fn fields_kept_with_one_video_url_read_as_one_version() {
        let old =
            r#"{"title":"T","team":"","city":"","poster_url":"","video_url":"https://a.ch/1"}"#;
        let fields: FilmFields = serde_json::from_str(old).unwrap();
        assert_eq!(fields.videos, Video::parse_lines("https://a.ch/1"));
        let none = r#"{"title":"T","team":"","city":"","poster_url":"","video_url":""}"#;
        let fields: FilmFields = serde_json::from_str(none).unwrap();
        assert!(fields.videos.is_empty());
        let again: FilmFields =
            serde_json::from_str(&serde_json::to_string(&fields).unwrap()).unwrap();
        assert_eq!(again, fields);
    }

    #[test]
    fn pair_lists_read_back_in_order_and_canonical() {
        let list = pair_list([(3, 1), (2, 5)]);
        assert_eq!(list, "1,3.2,5");
        assert_eq!(parse_pair_list(&list), vec![(1, 3), (2, 5)]);
        assert_eq!(parse_pair_list(""), vec![]);
        assert_eq!(parse_pair_list("4,2.x"), vec![(2, 4)]);
    }

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

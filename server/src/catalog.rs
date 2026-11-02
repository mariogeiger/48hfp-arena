//! The films as a wiki: `data.csv` is the seed, and every change is a revision
//! appended to a journal (`edits.jsonl`) that is never rewritten. The current
//! catalog is a pure fold of the journal over the seed, so banning an author
//! is just folding again without their revisions.
//!
//! Edits to a film take effect at once. A proposed film waits for one review
//! by someone else than its proposer: approved, it joins the catalog; rejected,
//! it stays out (and may be proposed again as a new film).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};
use versus_shared::{Change, Film, FilmFields, HistoryEntry, Proposal};

/// One line of the journal. `author` is the user id, which never leaves the
/// server: what is shown is `author_tag`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub rev: u64,
    pub time: u64,
    pub author: String,
    pub film: usize,
    #[serde(flatten)]
    pub change: Change,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Published,
    Pending { proposer: String },
    Rejected,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub fields: FilmFields,
    pub status: Status,
}

/// Why a change cannot be appended.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    UnknownFilm,
    Unchanged,
    NotPending,
    OwnProposal,
    Unwritable,
}

/// The state the journal describes: each film's latest fields and status.
/// Revisions by `banned` authors count for nothing.
pub fn fold(
    seed: &BTreeMap<usize, FilmFields>,
    journal: &[Revision],
    banned: &HashSet<String>,
) -> BTreeMap<usize, Entry> {
    let mut state: BTreeMap<usize, Entry> = seed
        .iter()
        .map(|(&id, fields)| {
            let entry = Entry {
                fields: fields.clone(),
                status: Status::Published,
            };
            (id, entry)
        })
        .collect();
    for r in journal.iter().filter(|r| !banned.contains(&r.author)) {
        apply(&mut state, r);
    }
    state
}

/// The effect of one revision; one that does not fit the state is inert.
fn apply(state: &mut BTreeMap<usize, Entry>, r: &Revision) {
    match &r.change {
        Change::Edit { fields, .. } => {
            if let Some(entry) = state.get_mut(&r.film)
                && entry.status != Status::Rejected
            {
                entry.fields = fields.clone();
            }
        }
        Change::Propose { fields } => {
            state.entry(r.film).or_insert_with(|| Entry {
                fields: fields.clone(),
                status: Status::Pending {
                    proposer: r.author.clone(),
                },
            });
        }
        Change::Review { approve } => {
            if let Some(entry) = state.get_mut(&r.film)
                && let Status::Pending { proposer } = &entry.status
                && *proposer != r.author
            {
                entry.status = if *approve {
                    Status::Published
                } else {
                    Status::Rejected
                };
            }
        }
    }
}

/// A short, stable, anonymous tag for a user id.
pub fn author_tag(user_id: &str) -> String {
    let digest = Sha256::digest(user_id.as_bytes());
    format!("{:02x}{:02x}", digest[0], digest[1])
}

pub struct Catalog {
    seed: BTreeMap<usize, FilmFields>,
    journal: Vec<Revision>,
    state: BTreeMap<usize, Entry>,
    path: String,
}

impl Catalog {
    /// Read the journal at `path` (missing means empty). A torn last line,
    /// left by a crash mid-append, is skipped.
    pub fn load(seed: Vec<Film>, path: &str, banned: &HashSet<String>) -> Self {
        let seed: BTreeMap<usize, FilmFields> = seed.iter().map(|f| (f.id, f.fields())).collect();
        let mut journal = Vec::new();
        if let Ok(file) = std::fs::File::open(path) {
            for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
                let Ok(line) = line else { break };
                match serde_json::from_str::<Revision>(&line) {
                    Ok(r) => journal.push(r),
                    Err(e) => log::error!("{path}:{}: skipped unreadable revision: {e}", i + 1),
                }
            }
        }
        let state = fold(&seed, &journal, banned);
        log::info!("Loaded {} revisions from {path}", journal.len());
        Self {
            seed,
            journal,
            state,
            path: path.to_string(),
        }
    }

    pub fn refold(&mut self, banned: &HashSet<String>) {
        self.state = fold(&self.seed, &self.journal, banned);
    }

    /// The films people can vote on.
    pub fn published(&self) -> HashMap<usize, Film> {
        self.state
            .iter()
            .filter(|(_, e)| e.status == Status::Published)
            .map(|(&id, e)| (id, e.fields.clone().into_film(id)))
            .collect()
    }

    pub fn proposals(&self, user_id: &str) -> Vec<Proposal> {
        self.state
            .iter()
            .filter_map(|(&id, e)| {
                let Status::Pending { proposer } = &e.status else {
                    return None;
                };
                let proposed = self
                    .journal
                    .iter()
                    .find(|r| r.film == id && matches!(r.change, Change::Propose { .. }))?;
                Some(Proposal {
                    film: e.fields.clone().into_film(id),
                    author: author_tag(proposer),
                    time: proposed.time,
                    is_yours: proposer == user_id,
                })
            })
            .collect()
    }

    /// Every revision of `film`, newest first, as `user_id` sees them.
    pub fn history(&self, film: usize, user_id: &str) -> Vec<HistoryEntry> {
        self.journal
            .iter()
            .rev()
            .filter(|r| r.film == film)
            .map(|r| HistoryEntry {
                rev: r.rev,
                time: r.time,
                author: author_tag(&r.author),
                is_you: r.author == user_id,
                change: r.change.clone(),
            })
            .collect()
    }

    /// The fields `film` had right after revision `rev`, 0 being the seed's.
    pub fn fields_at(&self, film: usize, rev: u64) -> Option<FilmFields> {
        if rev == 0 {
            return self.seed.get(&film).cloned();
        }
        let r = self
            .journal
            .iter()
            .find(|r| r.rev == rev && r.film == film)?;
        match &r.change {
            Change::Edit { fields, .. } | Change::Propose { fields } => Some(fields.clone()),
            Change::Review { .. } => None,
        }
    }

    /// Check `change` against the current state; on success, append it to the
    /// journal (durably) and fold it in. Returns the film it concerns.
    pub fn append(
        &mut self,
        author: &str,
        film: Option<usize>,
        change: Change,
        now: u64,
    ) -> Result<usize, Refusal> {
        let film = match (&change, film) {
            (Change::Propose { .. }, _) => self.next_id(),
            (_, Some(film)) => film,
            (_, None) => return Err(Refusal::UnknownFilm),
        };
        let entry = self.state.get(&film);
        match (&change, entry) {
            (Change::Propose { .. }, _) => {}
            (_, None)
            | (
                _,
                Some(Entry {
                    status: Status::Rejected,
                    ..
                }),
            ) => {
                return Err(Refusal::UnknownFilm);
            }
            (Change::Edit { fields, .. }, Some(e)) if *fields == e.fields => {
                return Err(Refusal::Unchanged);
            }
            (Change::Edit { .. }, Some(_)) => {}
            (Change::Review { .. }, Some(e)) => match &e.status {
                Status::Pending { proposer } if proposer == author => {
                    return Err(Refusal::OwnProposal);
                }
                Status::Pending { .. } => {}
                _ => return Err(Refusal::NotPending),
            },
        }
        let revision = Revision {
            rev: self.journal.last().map_or(1, |r| r.rev + 1),
            time: now,
            author: author.to_string(),
            film,
            change,
        };
        self.write(&revision).map_err(|e| {
            log::error!("Cannot append to {}: {e}", self.path);
            Refusal::Unwritable
        })?;
        apply(&mut self.state, &revision);
        self.journal.push(revision);
        Ok(film)
    }

    /// Ids are never reused, not even those of rejected or banned proposals.
    fn next_id(&self) -> usize {
        let seeded = self.seed.keys().max().copied().unwrap_or(0);
        let journaled = self.journal.iter().map(|r| r.film).max().unwrap_or(0);
        seeded.max(journaled) + 1
    }

    fn write(&self, revision: &Revision) -> std::io::Result<()> {
        let mut line = serde_json::to_string(revision)?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.sync_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(title: &str) -> FilmFields {
        FilmFields {
            title: title.into(),
            ..Default::default()
        }
    }

    fn catalog() -> Catalog {
        let path = std::env::temp_dir().join(format!(
            "versus-catalog-{}-{}.jsonl",
            std::process::id(),
            rand::random::<u64>()
        ));
        Catalog::load(
            vec![fields("A").into_film(1), fields("B").into_film(2)],
            path.to_str().unwrap(),
            &HashSet::new(),
        )
    }

    fn edit(title: &str) -> Change {
        Change::Edit {
            fields: fields(title),
            revert_of: None,
        }
    }

    #[test]
    fn edits_apply_at_once_and_unchanged_is_refused() {
        let mut c = catalog();
        assert_eq!(c.append("u", Some(1), edit("A2"), 0), Ok(1));
        assert_eq!(c.published()[&1].title, "A2");
        assert_eq!(
            c.append("v", Some(1), edit("A2"), 0),
            Err(Refusal::Unchanged)
        );
        assert_eq!(
            c.append("v", Some(9), edit("Z"), 0),
            Err(Refusal::UnknownFilm)
        );
    }

    #[test]
    fn proposals_wait_for_someone_else() {
        let mut c = catalog();
        let id = c
            .append(
                "u",
                None,
                Change::Propose {
                    fields: fields("C"),
                },
                0,
            )
            .unwrap();
        assert_eq!(id, 3);
        assert!(!c.published().contains_key(&id));
        let approve = Change::Review { approve: true };
        assert_eq!(
            c.append("u", Some(id), approve.clone(), 0),
            Err(Refusal::OwnProposal)
        );
        assert_eq!(c.append("v", Some(id), approve.clone(), 0), Ok(id));
        assert!(c.published().contains_key(&id));
        assert_eq!(
            c.append("w", Some(id), approve, 0),
            Err(Refusal::NotPending)
        );
    }

    #[test]
    fn rejected_ids_are_never_reused() {
        let mut c = catalog();
        let id = c
            .append(
                "u",
                None,
                Change::Propose {
                    fields: fields("C"),
                },
                0,
            )
            .unwrap();
        c.append("v", Some(id), Change::Review { approve: false }, 0)
            .unwrap();
        assert_eq!(
            c.append("v", Some(id), edit("C2"), 0),
            Err(Refusal::UnknownFilm)
        );
        let again = c
            .append(
                "u",
                None,
                Change::Propose {
                    fields: fields("C"),
                },
                0,
            )
            .unwrap();
        assert_eq!(again, id + 1);
    }

    #[test]
    fn banning_folds_an_author_out_and_the_journal_survives_reload() {
        let mut c = catalog();
        c.append("vandal", Some(1), edit("XXX"), 0).unwrap();
        c.append("u", Some(2), edit("B2"), 0).unwrap();
        let banned: HashSet<String> = ["vandal".to_string()].into();
        c.refold(&banned);
        assert_eq!(c.published()[&1].title, "A");
        assert_eq!(c.published()[&2].title, "B2");
        let reloaded = Catalog::load(
            vec![fields("A").into_film(1), fields("B").into_film(2)],
            &c.path,
            &banned,
        );
        assert_eq!(reloaded.published()[&2].title, "B2");
        assert_eq!(reloaded.history(1, "u")[0].author, author_tag("vandal"));
        let _ = std::fs::remove_file(&c.path);
    }
}

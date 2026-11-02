//! Sliding-window rate limits on the wiki's writes, held in memory only: each
//! request counts against its user id and against its client address, and is
//! refused when either has used up its hour.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Edits and reviews.
    Edit,
    Propose,
    Upload,
}

impl Action {
    fn per_hour(self) -> usize {
        match self {
            Action::Edit => 20,
            Action::Propose => 3,
            Action::Upload => 20,
        }
    }
}

#[derive(Default)]
pub struct Throttle {
    hits: Mutex<HashMap<(Action, String), VecDeque<Instant>>>,
}

impl Throttle {
    /// Count one `action` by each of `keys` if none of them is over its limit.
    pub fn admit(&self, action: Action, keys: &[&str]) -> bool {
        self.admit_at(action, keys, Instant::now())
    }

    fn admit_at(&self, action: Action, keys: &[&str], now: Instant) -> bool {
        let mut hits = self.hits.lock().unwrap();
        hits.retain(|_, times| {
            while times
                .front()
                .is_some_and(|&t| now.duration_since(t) >= WINDOW)
            {
                times.pop_front();
            }
            !times.is_empty()
        });
        let full = keys.iter().any(|key| {
            hits.get(&(action, key.to_string()))
                .is_some_and(|times| times.len() >= action.per_hour())
        });
        if full {
            return false;
        }
        for key in keys {
            hits.entry((action, key.to_string()))
                .or_default()
                .push_back(now);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_address_limits_every_user_behind_it() {
        let throttle = Throttle::default();
        let t0 = Instant::now();
        for i in 0..3 {
            assert!(throttle.admit_at(Action::Propose, &[&format!("u{i}"), "ip"], t0));
        }
        assert!(!throttle.admit_at(Action::Propose, &["u9", "ip"], t0));
        assert!(throttle.admit_at(Action::Propose, &["u9", "other"], t0));
        assert!(throttle.admit_at(Action::Propose, &["u9", "ip"], t0 + WINDOW));
    }
}

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtRating {
    pub film_id: usize,
    /// Bradley-Terry strength parameter β (normalized so geometric mean of active films = 1).
    pub score: f64,
    pub comparisons: u32,
    /// Pairwise win counts: film_id → number of times this film beat that film.
    pub wins_against: HashMap<usize, u32>,
}

impl BtRating {
    pub fn new(film_id: usize) -> Self {
        Self {
            film_id,
            score: 1.0,
            comparisons: 0,
            wins_against: HashMap::new(),
        }
    }
    pub fn wins(&self) -> u32 {
        self.wins_against.values().sum()
    }
    pub fn losses(&self) -> u32 {
        self.comparisons - self.wins()
    }
}

/// Run the Bradley-Terry MM algorithm to convergence.
///
/// The model assigns each film a strength β_i > 0 such that
///   P(i beats j) = β_i / (β_i + β_j).
///
/// The MM update is:
///   β_i ← W_i / Σ_{j≠i} n_ij / (β_i + β_j)
/// where W_i = total wins and n_ij = total comparisons between i and j.
///
/// After each iteration the strengths are normalized so the geometric mean
/// of active films equals 1, which anchors the scale.
///
/// Films with zero wins keep a near-zero score (they sit below all ranked films).
pub fn run_bradley_terry(ratings: &mut HashMap<usize, BtRating>) {
    let active_ids: Vec<usize> = ratings.values()
        .filter(|r| r.comparisons > 0)
        .map(|r| r.film_id)
        .collect();

    if active_ids.is_empty() {
        return;
    }

    // Films with 0 wins cannot be estimated by MLE (β → 0); pin them to a small floor.
    for &id in &active_ids {
        if ratings[&id].wins() == 0 {
            ratings.get_mut(&id).unwrap().score = 1e-6;
        }
    }

    let ranked_ids: Vec<usize> = active_ids.iter()
        .copied()
        .filter(|&id| ratings[&id].wins() > 0)
        .collect();

    if ranked_ids.is_empty() {
        return;
    }

    // Map film_id → index into old_scores; allocated once, reused each iteration.
    let idx: HashMap<usize, usize> = active_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let mut old_scores: Vec<f64> = active_ids.iter().map(|&id| ratings[&id].score).collect();

    for _ in 0..500 {
        // Snapshot scores at the start of this iteration.
        for (pos, &id) in active_ids.iter().enumerate() {
            old_scores[pos] = ratings[&id].score;
        }

        let mut max_rel_change = 0.0_f64;

        for &i in &ranked_ids {
            let w_i = ratings[&i].wins() as f64;
            let score_i = old_scores[idx[&i]];

            let denom: f64 = active_ids.iter()
                .filter(|&&j| j != i)
                .filter_map(|&j| {
                    let n_ij =
                        ratings[&i].wins_against.get(&j).copied().unwrap_or(0) as f64
                        + ratings[&j].wins_against.get(&i).copied().unwrap_or(0) as f64;
                    if n_ij > 0.0 {
                        Some(n_ij / (score_i + old_scores[idx[&j]]))
                    } else {
                        None
                    }
                })
                .sum();

            if denom > 0.0 {
                let new_score = w_i / denom;
                let rel_change = (new_score - score_i).abs() / score_i;
                max_rel_change = max_rel_change.max(rel_change);
                ratings.get_mut(&i).unwrap().score = new_score;
            }
        }

        let log_mean: f64 = ranked_ids.iter()
            .map(|&id| ratings[&id].score.ln())
            .sum::<f64>()
            / ranked_ids.len() as f64;
        let scale = (-log_mean).exp();
        for &id in &ranked_ids {
            ratings.get_mut(&id).unwrap().score *= scale;
        }

        if max_rel_change < 1e-8 {
            break;
        }
    }
}

/// Convert a Bradley-Terry strength β to an ELO-equivalent display rating.
/// With the normalization β̄ = 1 (geometric mean), the average film scores 1500.
pub fn bt_score_to_display(score: f64) -> f64 {
    400.0 * score.max(1e-10).log10() + 1500.0
}

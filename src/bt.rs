use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    let active_ids: Vec<usize> = ratings
        .values()
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

    let ranked_ids: Vec<usize> = active_ids
        .iter()
        .copied()
        .filter(|&id| ratings[&id].wins() > 0)
        .collect();

    if ranked_ids.is_empty() {
        return;
    }

    // Map film_id → index into old_scores; allocated once, reused each iteration.
    let idx: HashMap<usize, usize> = active_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
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

            let denom: f64 = active_ids
                .iter()
                .filter(|&&j| j != i)
                .filter_map(|&j| {
                    let n_ij = ratings[&i].wins_against.get(&j).copied().unwrap_or(0) as f64
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

        let log_mean: f64 = ranked_ids
            .iter()
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

/// Compute D-optimal Fisher Information scores for candidate pairs.
///
/// For the Bradley-Terry model P(i>j) = β_i/(β_i+β_j) parameterized by
/// θ_i = ln(β_i), the Fisher Information from one comparison (i,j) is a
/// rank-1 update: p_ij*(1-p_ij) * v*v^T where v = e_i - e_j.
///
/// The D-optimal criterion selects the pair maximizing:
///   p_ij*(1-p_ij) * v^T F^{-1} v
///
/// This accounts for both match closeness (p near 0.5 → high info) and
/// global uncertainty (films with few comparisons → large F^{-1} entries).
pub fn fisher_pair_scores(
    ratings: &HashMap<usize, BtRating>,
    film_ids: &[usize],
    candidates: &[(usize, usize)],
) -> Vec<f64> {
    let n = film_ids.len();
    if n < 2 || candidates.is_empty() {
        return vec![1.0; candidates.len()];
    }

    let id_to_pos: HashMap<usize, usize> = film_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    // Build n×n Fisher Information matrix with prior regularization.
    // The prior makes F positive definite directly — no need to drop a
    // reference film, which would create special cases in the scoring.
    let mut fisher = vec![0.0f64; n * n];

    for i in 0..n {
        for j in (i + 1)..n {
            let id_i = film_ids[i];
            let id_j = film_ids[j];

            let wins_ij = ratings
                .get(&id_i)
                .and_then(|r| r.wins_against.get(&id_j).copied())
                .unwrap_or(0);
            let wins_ji = ratings
                .get(&id_j)
                .and_then(|r| r.wins_against.get(&id_i).copied())
                .unwrap_or(0);
            let n_ij = wins_ij + wins_ji;
            if n_ij == 0 {
                continue;
            }

            let beta_i = ratings.get(&id_i).map(|r| r.score).unwrap_or(1.0);
            let beta_j = ratings.get(&id_j).map(|r| r.score).unwrap_or(1.0);
            let p = beta_i / (beta_i + beta_j);
            let info = n_ij as f64 * p * (1.0 - p);

            fisher[i * n + i] += info;
            fisher[j * n + j] += info;
            fisher[i * n + j] -= info;
            fisher[j * n + i] -= info;
        }
    }

    let prior = 0.25;
    for k in 0..n {
        fisher[k * n + k] += prior;
    }

    let finv = invert_matrix(&fisher, n);

    // D-optimal score: p*(1-p) * v^T F^{-1} v — uniform for all pairs.
    candidates
        .iter()
        .map(|&(a, b)| {
            let beta_a = ratings.get(&a).map(|r| r.score).unwrap_or(1.0);
            let beta_b = ratings.get(&b).map(|r| r.score).unwrap_or(1.0);
            let p = beta_a / (beta_a + beta_b);
            let pq = p * (1.0 - p);

            let i = id_to_pos[&a];
            let j = id_to_pos[&b];
            pq * (finv[i * n + i] + finv[j * n + j] - 2.0 * finv[i * n + j])
        })
        .collect()
}

/// Invert an n×n matrix using Gauss-Jordan elimination with partial pivoting.
fn invert_matrix(mat: &[f64], n: usize) -> Vec<f64> {
    let w = 2 * n;
    let mut aug = vec![0.0f64; n * w];
    for i in 0..n {
        for j in 0..n {
            aug[i * w + j] = mat[i * n + j];
        }
        aug[i * w + n + i] = 1.0;
    }

    for col in 0..n {
        let mut best = col;
        for row in (col + 1)..n {
            if aug[row * w + col].abs() > aug[best * w + col].abs() {
                best = row;
            }
        }
        if best != col {
            for k in 0..w {
                aug.swap(col * w + k, best * w + k);
            }
        }

        let pivot = aug[col * w + col];
        if pivot.abs() < 1e-15 {
            continue;
        }

        let inv_pivot = 1.0 / pivot;
        for k in 0..w {
            aug[col * w + k] *= inv_pivot;
        }

        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row * w + col];
            for k in 0..w {
                aug[row * w + k] -= factor * aug[col * w + k];
            }
        }
    }

    let mut inv = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            inv[i * n + j] = aug[i * w + n + j];
        }
    }
    inv
}

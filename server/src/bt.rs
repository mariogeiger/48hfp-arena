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
    /// Pairwise tie counts: film_id → number of times this film tied with it.
    #[serde(default)]
    pub ties_with: HashMap<usize, u32>,
}

impl BtRating {
    pub fn new(film_id: usize) -> Self {
        Self {
            film_id,
            score: 1.0,
            comparisons: 0,
            wins_against: HashMap::new(),
            ties_with: HashMap::new(),
        }
    }
    pub fn wins(&self) -> u32 {
        self.wins_against.values().sum()
    }
    pub fn ties(&self) -> u32 {
        self.ties_with.values().sum()
    }
    pub fn losses(&self) -> u32 {
        self.comparisons - self.wins() - self.ties()
    }
    /// What the duels earned it, a tie counting half a win.
    pub fn points(&self) -> f64 {
        self.wins() as f64 + 0.5 * self.ties() as f64
    }
    /// How many duels it fought against `other`.
    pub fn duels_with(&self, other: &BtRating) -> u32 {
        let count = |m: &HashMap<usize, u32>, id: usize| m.get(&id).copied().unwrap_or(0);
        count(&self.wins_against, other.film_id)
            + count(&other.wins_against, self.film_id)
            + count(&self.ties_with, other.film_id)
    }
}

/// Run the Bradley-Terry MM algorithm to convergence. A tie counts as half a
/// win to each side, which is the maximum of the likelihood where a tie
/// weighs as much as the geometric mean of the two ways to win.
pub fn run_bradley_terry(ratings: &mut HashMap<usize, BtRating>) {
    let active_ids: Vec<usize> = ratings
        .values()
        .filter(|r| r.comparisons > 0)
        .map(|r| r.film_id)
        .collect();

    if active_ids.is_empty() {
        return;
    }

    for &id in &active_ids {
        if ratings[&id].points() == 0.0 {
            ratings.get_mut(&id).unwrap().score = 1e-6;
        }
    }

    let ranked_ids: Vec<usize> = active_ids
        .iter()
        .copied()
        .filter(|&id| ratings[&id].points() > 0.0)
        .collect();

    if ranked_ids.is_empty() {
        return;
    }

    let idx: HashMap<usize, usize> = active_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    let mut old_scores: Vec<f64> = active_ids.iter().map(|&id| ratings[&id].score).collect();

    for _ in 0..500 {
        for (pos, &id) in active_ids.iter().enumerate() {
            old_scores[pos] = ratings[&id].score;
        }

        let mut max_rel_change = 0.0_f64;

        for &i in &ranked_ids {
            let w_i = ratings[&i].points();
            let score_i = old_scores[idx[&i]];

            let denom: f64 = active_ids
                .iter()
                .filter(|&&j| j != i)
                .filter_map(|&j| {
                    let n_ij = ratings[&i].duels_with(&ratings[&j]) as f64;
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

pub fn bt_score_to_display(score: f64) -> f64 {
    ((1.0 + score).log2() * 500.0).round()
}

/// Compute D-optimal Fisher Information scores for candidate pairs.
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

    let mut fisher = vec![0.0f64; n * n];

    for i in 0..n {
        for j in (i + 1)..n {
            let id_i = film_ids[i];
            let id_j = film_ids[j];

            let n_ij = match (ratings.get(&id_i), ratings.get(&id_j)) {
                (Some(r_i), Some(r_j)) => r_i.duels_with(r_j),
                _ => 0,
            };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ratings(wins: &[(usize, usize)], ties: &[(usize, usize)]) -> HashMap<usize, BtRating> {
        let mut ratings: HashMap<usize, BtRating> = HashMap::new();
        for &(w, l) in wins {
            for id in [w, l] {
                ratings
                    .entry(id)
                    .or_insert_with(|| BtRating::new(id))
                    .comparisons += 1;
            }
            *ratings
                .get_mut(&w)
                .unwrap()
                .wins_against
                .entry(l)
                .or_insert(0) += 1;
        }
        for &(a, b) in ties {
            for (x, y) in [(a, b), (b, a)] {
                let r = ratings.entry(x).or_insert_with(|| BtRating::new(x));
                r.comparisons += 1;
                *r.ties_with.entry(y).or_insert(0) += 1;
            }
        }
        run_bradley_terry(&mut ratings);
        ratings
    }

    #[test]
    fn two_ties_weigh_as_a_win_each_way() {
        // 1 beats 2 and is beaten by 3; 2 and 3 meet twice.
        let split = ratings(&[(1, 2), (3, 1), (2, 3), (3, 2)], &[]);
        let tied = ratings(&[(1, 2), (3, 1)], &[(2, 3), (2, 3)]);
        for id in [1, 2, 3] {
            assert!((split[&id].score - tied[&id].score).abs() < 1e-6);
        }
        assert_eq!(tied[&2].ties(), 2);
        assert_eq!(tied[&2].losses(), 1);
        assert_eq!(tied[&2].points(), 1.0);
    }

    #[test]
    fn only_ties_rank_alike() {
        let tied = ratings(&[], &[(1, 2)]);
        assert!((tied[&1].score - tied[&2].score).abs() < 1e-9);
        assert!(tied[&1].score > 1e-3);
    }
}

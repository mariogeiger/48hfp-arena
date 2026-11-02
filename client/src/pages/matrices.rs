//! The two vote matrices of the More page: the viewer's own votes, which a
//! tap edits, and everyone's wins against what the model predicts.

use crate::components::matrix::*;
use crate::i18n::{fill, t};
use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn MatrixSections() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Re-render user matrix when data or board changes
    Effect::new({
        let state = state.clone();
        move || {
            let data = state.user_matrix.get();
            let board = state.board.get();
            let Some(data) = data else { return };
            if data.films.is_empty() {
                let doc = web_sys::window().unwrap().document().unwrap();
                if let Some(el) = doc.get_element_by_id("user-matrix") {
                    el.set_inner_html(&format!(
                        r#"<p class="matrix-empty">{}</p>"#,
                        t().matrix_yours_empty
                    ));
                }
                return;
            }

            let films = sort_films_by_board(&data.films, &board);
            let vote_map: std::collections::HashMap<String, usize> = data
                .votes
                .iter()
                .filter_map(|v| {
                    let a = v.film_a.min(v.film_b);
                    let b = v.film_a.max(v.film_b);
                    Some((format!("{},{}", a, b), v.winner?))
                })
                .collect();
            let legacy_set: std::collections::HashSet<String> = data
                .legacy_votes
                .iter()
                .map(|v| {
                    let a = v.film_a.min(v.film_b);
                    let b = v.film_a.max(v.film_b);
                    format!("{},{}", a, b)
                })
                .collect();

            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let vote_map_ci = vote_map.clone();
            let legacy_set_ci = legacy_set.clone();
            let films_tt = films.clone();
            let films_oc = films.clone();
            let vote_map_oc = vote_map.clone();
            let state_oc = state.clone();

            render_matrix_canvas(
                "user-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let row = &films_ci[ri];
                        let col = &films_ci[ci];
                        let a = row.id.min(col.id);
                        let b = row.id.max(col.id);
                        let key = format!("{},{}", a, b);
                        if let Some(&winner) = vote_map_ci.get(&key) {
                            let won = winner == row.id;
                            CellInfo {
                                bg: if won { CellBg::Win } else { CellBg::Loss },
                                text: if won { t().matrix_win } else { t().matrix_loss }
                                    .to_string(),
                            }
                        } else if legacy_set_ci.contains(&key) {
                            CellInfo {
                                bg: CellBg::Legacy,
                                text: "?".to_string(),
                            }
                        } else {
                            CellInfo {
                                bg: CellBg::Empty,
                                text: String::new(),
                            }
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        fill(
                            t().matrix_versus,
                            &[("a", &films_tt[ri].title), ("b", &films_tt[ci].title)],
                        )
                    }),
                    on_click: Some(Box::new(move |ri, ci| {
                        let row = &films_oc[ri];
                        let col = &films_oc[ci];
                        let a = row.id.min(col.id);
                        let b = row.id.max(col.id);
                        let key = format!("{},{}", a, b);
                        let s = state_oc.clone();
                        if let Some(&winner) = vote_map_oc.get(&key) {
                            let loser = if winner == row.id { col.id } else { row.id };
                            wasm_bindgen_futures::spawn_local(async move {
                                crate::api::matrix_action(&s, "/api/unvote", winner, loser).await;
                            });
                        } else {
                            let (w, l) = (row.id, col.id);
                            wasm_bindgen_futures::spawn_local(async move {
                                crate::api::matrix_action(&s, "/api/vote", w, l).await;
                            });
                        }
                    })),
                },
            );
        }
    });

    // Re-render global matrix when data or board changes
    Effect::new({
        let state = state.clone();
        move || {
            let data = state.global_matrix.get();
            let board = state.board.get();
            let Some(data) = data else { return };
            if data.films.is_empty() {
                let doc = web_sys::window().unwrap().document().unwrap();
                if let Some(el) = doc.get_element_by_id("global-matrix") {
                    el.set_inner_html(&format!(
                        r#"<p class="matrix-empty">{}</p>"#,
                        t().matrix_global_empty
                    ));
                }
                return;
            }

            let films = sort_films_by_board(&data.films, &board);
            let win_map: std::collections::HashMap<String, u32> = data
                .wins
                .iter()
                .map(|w| (format!("{},{}", w.winner, w.loser), w.count))
                .collect();
            let score_map: std::collections::HashMap<usize, f64> = data
                .films
                .iter()
                .map(|f| (f.id, f.score.unwrap_or(1.0)))
                .collect();

            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let win_map_ci = win_map.clone();
            let score_map_ci = score_map.clone();
            let films_tt = films.clone();
            let win_map_tt = win_map.clone();
            let score_map_tt = score_map.clone();

            render_matrix_canvas(
                "global-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let row = &films_ci[ri];
                        let col = &films_ci[ci];
                        let w_rc = win_map_ci
                            .get(&format!("{},{}", row.id, col.id))
                            .copied()
                            .unwrap_or(0);
                        let w_cr = win_map_ci
                            .get(&format!("{},{}", col.id, row.id))
                            .copied()
                            .unwrap_or(0);
                        let total = w_rc + w_cr;
                        if total == 0 {
                            return CellInfo {
                                bg: CellBg::Empty,
                                text: String::new(),
                            };
                        }
                        let observed = w_rc as f64 / total as f64;
                        let bi = score_map_ci.get(&row.id).copied().unwrap_or(1.0);
                        let bj = score_map_ci.get(&col.id).copied().unwrap_or(1.0);
                        let predicted = bi / (bi + bj);
                        let residual = observed - predicted;
                        CellInfo {
                            bg: CellBg::Residual(residual),
                            text: w_rc.to_string(),
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        let row = &films_tt[ri];
                        let col = &films_tt[ci];
                        let w_rc = win_map_tt
                            .get(&format!("{},{}", row.id, col.id))
                            .copied()
                            .unwrap_or(0);
                        let w_cr = win_map_tt
                            .get(&format!("{},{}", col.id, row.id))
                            .copied()
                            .unwrap_or(0);
                        let total = w_rc + w_cr;
                        if total == 0 {
                            return fill(
                                t().matrix_no_votes,
                                &[("a", &row.title), ("b", &col.title)],
                            );
                        }
                        let observed = w_rc as f64 / total as f64;
                        let bi = score_map_tt.get(&row.id).copied().unwrap_or(1.0);
                        let bj = score_map_tt.get(&col.id).copied().unwrap_or(1.0);
                        let predicted = bi / (bi + bj);
                        let residual = observed - predicted;
                        let sign = if residual > 0.0 { "+" } else { "" };
                        fill(
                            t().matrix_residual,
                            &[
                                ("a", &row.title),
                                ("b", &col.title),
                                ("o", &((observed * 100.0) as i32)),
                                ("m", &((predicted * 100.0) as i32)),
                                ("r", &format!("{sign}{}", (residual * 100.0) as i32)),
                            ],
                        )
                    }),
                    on_click: None,
                },
            );
        }
    });

    view! {
        <div class="matrix-section">
            <h3>{t().matrix_yours}</h3>
            <p class="matrix-hint">{t().matrix_yours_hint}</p>
            <div class="matrix-scroll-wrapper">
                <div id="user-matrix">
                    <p class="matrix-empty">{t().matrix_yours_empty}</p>
                </div>
            </div>
        </div>
        <div class="matrix-section">
            <h3>{t().matrix_global}</h3>
            <p class="matrix-hint">{t().matrix_global_hint}</p>
            <div class="matrix-scroll-wrapper">
                <div id="global-matrix">
                    <p class="matrix-empty">{t().matrix_global_empty}</p>
                </div>
            </div>
        </div>
    }
}

fn sort_films_by_board(
    films: &[crate::state::MatrixFilm],
    board: &[crate::state::LeaderboardEntry],
) -> Vec<crate::state::MatrixFilm> {
    let rank: std::collections::HashMap<usize, usize> = board
        .iter()
        .enumerate()
        .map(|(i, b)| (b.film_id, i))
        .collect();
    let mut sorted = films.to_vec();
    sorted.sort_by_key(|f| rank.get(&f.id).copied().unwrap_or(usize::MAX));
    sorted
}

//! The two vote matrices: the viewer's own votes, wins, losses and ties, on
//! their page, and everyone's scores (a tie counting half) against what the
//! model predicts, behind the ranking. A
//! tap on a cell of either opens the duel of its pair, where the viewer's
//! vote is set.

use crate::components::matrix::*;
use crate::i18n::{fill, t};
use crate::state::AppState;
use leptos::prelude::*;
use versus_shared::Outcome;

#[component]
pub fn UserMatrix() -> impl IntoView {
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
            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let films_tt = films.clone();
            let films_oc = films.clone();
            let state_oc = state.clone();

            render_matrix_canvas(
                "user-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let (row, col) = (films_ci[ri].id, films_ci[ci].id);
                        let (bg, text) = match data.outcome(row, col) {
                            Some(Outcome::Won(w)) if w == row => (CellBg::Win, t().matrix_win),
                            Some(Outcome::Won(_)) => (CellBg::Loss, t().matrix_loss),
                            Some(Outcome::Tie) => (CellBg::Tie, t().matrix_tie),
                            None if data.compared_before(row, col) => (CellBg::Legacy, "?"),
                            None => (CellBg::Empty, ""),
                        };
                        CellInfo {
                            bg,
                            text: text.to_string(),
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        fill(
                            t().matrix_versus,
                            &[("a", &films_tt[ri].title), ("b", &films_tt[ci].title)],
                        )
                    }),
                    on_click: Box::new(move |ri, ci| {
                        state_oc.duel.set(Some((films_oc[ri].id, films_oc[ci].id)));
                    }),
                },
            );
        }
    });

    view! {
        <div class="matrix-scroll-wrapper">
            <div id="user-matrix">
                <p class="matrix-empty">{t().matrix_yours_empty}</p>
            </div>
        </div>
    }
}

#[component]
pub fn GlobalMatrix() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

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
            let tally = std::rc::Rc::new(data.tally());
            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let tally_ci = tally.clone();
            let films_tt = films.clone();
            let films_oc = films.clone();
            let state_oc = state.clone();

            render_matrix_canvas(
                "global-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let (row, col) = (films_ci[ri].id, films_ci[ci].id);
                        match tally_ci.share(row, col) {
                            None => CellInfo {
                                bg: CellBg::Empty,
                                text: String::new(),
                            },
                            Some(observed) => CellInfo {
                                bg: CellBg::Residual(observed - tally_ci.chance(row, col)),
                                text: tally_ci.record(row, col).0.to_string(),
                            },
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        let (row, col) = (&films_tt[ri], &films_tt[ci]);
                        let Some(observed) = tally.share(row.id, col.id) else {
                            return fill(
                                t().matrix_no_votes,
                                &[("a", &row.title), ("b", &col.title)],
                            );
                        };
                        let predicted = tally.chance(row.id, col.id);
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
                    on_click: Box::new(move |ri, ci| {
                        state_oc.duel.set(Some((films_oc[ri].id, films_oc[ci].id)));
                    }),
                },
            );
        }
    });

    view! {
        <div class="matrix-scroll-wrapper">
            <div id="global-matrix">
                <p class="matrix-empty">{t().matrix_global_empty}</p>
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

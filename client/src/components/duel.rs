//! The duel a matrix cell opens, over its page: the two items side by
//! side, the viewer's vote on the pair, which a tap on an item sets and a
//! button takes back, and how everyone voted against what the model predicts.
//! Nothing changes until the viewer taps, and closing changes nothing.

use crate::api;
use crate::components::card::CardFace;
use crate::components::frame::SheetFrame;
use crate::i18n::{fill, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;
use versus_shared::Film;

/// The viewer's vote on a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vote {
    None,
    /// Compared before winners were recorded.
    Legacy,
    Won(usize),
}

fn own_vote(state: &AppState, a: usize, b: usize) -> Vote {
    let is_pair = |x: usize, y: usize| (x, y) == (a, b) || (x, y) == (b, a);
    state.user_matrix.with(|m| {
        let Some(m) = m else { return Vote::None };
        if let Some(v) = m.votes.iter().find(|v| is_pair(v.film_a, v.film_b)) {
            return v.winner.map_or(Vote::Legacy, Vote::Won);
        }
        if m.legacy_votes.iter().any(|v| is_pair(v.film_a, v.film_b)) {
            return Vote::Legacy;
        }
        Vote::None
    })
}

/// Everyone's wins of `a` over `b` and of `b` over `a`, and the chance the
/// Bradley–Terry model gives `a`.
fn global_tally(state: &AppState, a: usize, b: usize) -> (u32, u32, f64) {
    state.global_matrix.with(|m| {
        let Some(m) = m else { return (0, 0, 0.5) };
        let wins = |w: usize, l: usize| {
            m.wins
                .iter()
                .find(|x| x.winner == w && x.loser == l)
                .map_or(0, |x| x.count)
        };
        let score = |id: usize| {
            m.films
                .iter()
                .find(|f| f.id == id)
                .and_then(|f| f.score)
                .unwrap_or(1.0)
        };
        (wins(a, b), wins(b, a), score(a) / (score(a) + score(b)))
    })
}

#[component]
pub fn DuelSheet() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let busy = RwSignal::new(false);

    move || {
        let (a_id, b_id) = state.duel.get()?;
        let film = |id: usize| {
            state
                .films
                .with(|films| films.iter().find(|f| f.id == id).cloned())
        };
        let (Some(a), Some(b)) = (film(a_id), film(b_id)) else {
            state.duel.set(None);
            return None;
        };
        let duel_question = SiteConfig::get().duel_question;
        let heading = if duel_question.is_empty() {
            fill(t().matrix_versus, &[("a", &a.title), ("b", &b.title)])
        } else {
            duel_question
        };

        let state_vote = state.clone();
        let vote = Memo::new(move |_| own_vote(&state_vote, a_id, b_id));

        let cast = {
            let state = state.clone();
            move |endpoint: &'static str, winner: usize, loser: usize| {
                let state = state.clone();
                busy.set(true);
                wasm_bindgen_futures::spawn_local(async move {
                    api::matrix_action(&state, endpoint, winner, loser).await;
                    busy.set(false);
                });
            }
        };
        let cast_remove = cast.clone();
        let card = {
            move |film: Film, other: usize| {
                let id = film.id;
                let cast = cast.clone();
                view! {
                    <button
                        class="duel-card"
                        class:picked=move || vote.get() == Vote::Won(id)
                        disabled=move || busy.get()
                        on:click=move |_| {
                            if vote.get_untracked() != Vote::Won(id) {
                                cast("/api/vote", id, other);
                            }
                        }
                    >
                        <CardFace film=film />
                        <span class="duel-badge">
                            {move || (vote.get() == Vote::Won(id)).then_some(t().duel_your_pick)}
                        </span>
                    </button>
                }
            }
        };

        let state_tally = state.clone();
        let tally = move || {
            let (wa, wb, p) = global_tally(&state_tally, a_id, b_id);
            let votes = wa + wb;
            let left = (p * 100.0).round() as i32;
            let percent = |n: i32| fill(t().percent, &[("p", &n)]);
            view! {
                {split(t().duel_everyone, wa.to_string(), wb.to_string(), (votes > 0).then(|| wa as f64 / votes as f64))}
                {split(t().duel_model, percent(left), percent(100 - left), Some(p))}
            }
        };

        let duel = state.duel;
        let (card_a, card_b) = (card.clone()(a, b_id), card(b, a_id));
        Some(view! {
            <SheetFrame on_close=Callback::new(move |_| duel.set(None)) class="duel-sheet">
                <h2>{heading}</h2>
                <p class="sheet-hint">
                    {move || match vote.get() {
                        Vote::None => t().duel_none,
                        Vote::Legacy => t().duel_legacy,
                        Vote::Won(_) => t().duel_hint,
                    }}
                </p>
                <div class="arena">
                    {card_a}
                    <div class="vs" aria-hidden="true">"VS"</div>
                    {card_b}
                </div>
                {move || match vote.get() {
                    Vote::Won(winner) => {
                        let loser = if winner == a_id { b_id } else { a_id };
                        let cast = cast_remove.clone();
                        Some(view! {
                            <button
                                class="ghost-btn danger"
                                disabled=move || busy.get()
                                on:click=move |_| cast("/api/unvote", winner, loser)
                            >
                                {t().duel_remove}
                            </button>
                        })
                    }
                    _ => None,
                }}
                <div class="duel-tally">{tally}</div>
            </SheetFrame>
        })
    }
}

/// A bar split between the left item's share and the right one's, `left`
/// and `right` at its ends; grey when there is nothing to share.
fn split(label: &'static str, left: String, right: String, share: Option<f64>) -> impl IntoView {
    view! {
        <div class="split">
            <span class="split-label">{label}</span>
            <div class="split-row">
                <b>{left}</b>
                <span class="split-bar" class:empty=share.is_none()>
                    <i style:width=format!("{:.1}%", 100.0 * share.unwrap_or(0.0))></i>
                </span>
                <b>{right}</b>
            </div>
        </div>
    }
}

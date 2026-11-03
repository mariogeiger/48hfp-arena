//! The duel a matrix cell opens, over its page: the two items side by
//! side, the viewer's vote on the pair, which a tap on an item or the tie
//! button sets and another button takes back, and how everyone voted against what the model predicts.
//! Nothing changes until the viewer taps, and closing changes nothing.

use crate::api;
use crate::components::card::CardFace;
use crate::components::frame::SheetFrame;
use crate::components::icons::{Icon, icon};
use crate::i18n::{fill, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;
use versus_shared::{Film, Outcome};

/// The viewer's vote on a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vote {
    None,
    /// Compared before outcomes were recorded.
    Legacy,
    Done(Outcome),
}

fn own_vote(state: &AppState, a: usize, b: usize) -> Vote {
    state.user_matrix.with(|m| {
        let Some(m) = m else { return Vote::None };
        match m.outcome(a, b) {
            Some(outcome) => Vote::Done(outcome),
            None if m.compared_before(a, b) => Vote::Legacy,
            None => Vote::None,
        }
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
            move |endpoint: &'static str, outcome: Outcome| {
                let state = state.clone();
                busy.set(true);
                wasm_bindgen_futures::spawn_local(async move {
                    api::matrix_action(&state, endpoint, a_id, b_id, outcome).await;
                    busy.set(false);
                });
            }
        };
        let cast_remove = cast.clone();
        let set = move |outcome: Outcome| {
            if vote.get_untracked() != Vote::Done(outcome) {
                cast("/api/vote", outcome);
            }
        };
        let card = {
            let set = set.clone();
            move |film: Film| {
                let won = Vote::Done(Outcome::Won(film.id));
                let set = set.clone();
                let id = film.id;
                let badge = move || match vote.get() {
                    v if v == won => Some(t().duel_your_pick),
                    Vote::Done(Outcome::Tie) => Some(t().tie),
                    _ => None,
                };
                view! {
                    <button
                        class="duel-card"
                        class:picked=move || vote.get() == won
                        class:tied=move || vote.get() == Vote::Done(Outcome::Tie)
                        disabled=move || busy.get()
                        on:click=move |_| set(Outcome::Won(id))
                    >
                        <CardFace film=film />
                        <span class="duel-badge">{badge}</span>
                    </button>
                }
            }
        };

        let state_tally = state.clone();
        let tally = move || {
            let (record, p) = state_tally.global_matrix.with(|m| match m {
                Some(m) => {
                    let tally = m.tally();
                    (tally.record(a_id, b_id), tally.chance(a_id, b_id))
                }
                None => ((0, 0, 0), 0.5),
            });
            let (wa, ties, wb) = record;
            let left = (p * 100.0).round() as i32;
            let percent = |n: i32| fill(t().percent, &[("p", &n)]);
            view! {
                {split(t().duel_everyone, wa.to_string(), wb.to_string(), [wa, ties, wb].map(f64::from))}
                {split(t().duel_model, percent(left), percent(100 - left), [p, 0.0, 1.0 - p])}
            }
        };

        let duel = state.duel;
        let (card_a, card_b) = (card.clone()(a), card(b));
        Some(view! {
            <SheetFrame on_close=Callback::new(move |_| duel.set(None)) class="duel-sheet">
                <h2>{heading}</h2>
                <p class="sheet-hint">
                    {move || match vote.get() {
                        Vote::None => t().duel_none,
                        Vote::Legacy => t().duel_legacy,
                        Vote::Done(_) => t().duel_hint,
                    }}
                </p>
                <div class="arena">
                    {card_a}
                    <div class="vs" aria-hidden="true">"VS"</div>
                    {card_b}
                </div>
                <button
                    class="tie-btn"
                    class:on=move || vote.get() == Vote::Done(Outcome::Tie)
                    disabled=move || busy.get()
                    on:click=move |_| set(Outcome::Tie)
                >
                    {icon(Icon::Equal)}
                    <span>{t().tie}</span>
                </button>
                {move || match vote.get() {
                    Vote::Done(outcome) => {
                        let cast = cast_remove.clone();
                        Some(view! {
                            <button
                                class="ghost-btn danger"
                                disabled=move || busy.get()
                                on:click=move |_| cast("/api/unvote", outcome)
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

/// A bar split in three, `left` and `right` at its ends: the left item's
/// share, the ties between, the right item's share, in proportion to
/// `parts`; grey when there is nothing to share.
fn split(label: &'static str, left: String, right: String, parts: [f64; 3]) -> impl IntoView {
    let total: f64 = parts.iter().sum();
    let width = |part: f64| format!("{:.1}%", 100.0 * part / total.max(f64::MIN_POSITIVE));
    view! {
        <div class="split">
            <span class="split-label">{label}</span>
            <div class="split-row">
                <b>{left}</b>
                <span class="split-bar" class:empty=total == 0.0>
                    <i style:width=width(parts[0])></i>
                    <i class="split-tie" style:width=width(parts[1])></i>
                </span>
                <b>{right}</b>
            </div>
        </div>
    }
}

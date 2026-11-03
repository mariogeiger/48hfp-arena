//! The duels: two items side by side, and the viewer taps the one they
//! prefer, drags towards it or presses ← or →. The winner glows and bursts
//! into confetti, the loser fades, the next pair slides in. Below sit undo,
//! skip, and the votes cast with how far the viewer is through their duels.

use crate::api;
use crate::components::card::CardFace;
use crate::components::confetti;
use crate::components::icons::{Icon, icon};
use crate::i18n::{fill, or_default, say, t};
use crate::state::{AppState, Page, PairDoneReason, Panel, SiteConfig};
use crate::timeout::Timeout;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// The counts of votes worth a shower of confetti.
const MILESTONES: [usize; 9] = [10, 25, 50, 100, 150, 200, 300, 500, 1000];
/// How far a drag must go to pick, in pixels.
const DRAG_PICK: f64 = 70.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    A,
    B,
}

impl Side {
    fn card(self) -> &'static str {
        match self {
            Side::A => "card-a",
            Side::B => "card-b",
        }
    }

    fn other(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

fn card(side: Side) -> Option<web_sys::HtmlElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id(side.card())?
        .dyn_into()
        .ok()
}

fn mark(side: Side, class: &str) {
    if let Some(card) = card(side) {
        let _ = card.style().remove_property("transform");
        let _ = card.style().remove_property("opacity");
        let _ = card.class_list().add_1(class);
    }
}

/// The moves of the duel in play. One runs at a time: `busy` holds the
/// others back until the next pair is in.
#[derive(Clone)]
struct Moves {
    state: AppState,
    busy: RwSignal<bool>,
}

impl Moves {
    fn start(&self) -> bool {
        if self.busy.get_untracked() || self.state.pair.get_untracked().is_none() {
            return false;
        }
        self.busy.set(true);
        true
    }

    fn pick(&self, side: Side) {
        let Some(pair) = self.state.pair.get_untracked() else {
            return;
        };
        if !self.start() {
            return;
        }
        let (winner, loser) = match side {
            Side::A => (pair.a.id, pair.b.id),
            Side::B => (pair.b.id, pair.a.id),
        };
        mark(side, "won");
        mark(side.other(), "lost");
        confetti::tap();
        if let Some((x, y)) = confetti::centre_of(side.card()) {
            confetti::burst(x, y, 18);
        }
        let (state, busy) = (self.state.clone(), self.busy);
        Timeout::new(420, move || {
            wasm_bindgen_futures::spawn_local(async move {
                api::cast_vote(&state, winner, loser).await;
                busy.set(false);
            });
        })
        .forget();
    }

    fn skip(&self) {
        if !self.start() {
            return;
        }
        mark(Side::A, "skipped");
        mark(Side::B, "skipped");
        let (state, busy) = (self.state.clone(), self.busy);
        Timeout::new(260, move || {
            wasm_bindgen_futures::spawn_local(async move {
                api::load_pair(&state).await;
                busy.set(false);
            });
        })
        .forget();
    }

    fn undo(&self) {
        if self.busy.get_untracked() || self.state.vote_history.get_untracked().is_empty() {
            return;
        }
        self.busy.set(true);
        mark(Side::A, "skipped");
        mark(Side::B, "skipped");
        let (state, busy) = (self.state.clone(), self.busy);
        Timeout::new(200, move || {
            wasm_bindgen_futures::spawn_local(async move {
                api::undo_vote(&state).await;
                busy.set(false);
            });
        })
        .forget();
    }

    /// The viewer does not know the item on `side`: out of their selection.
    fn unseen(&self, side: Side) {
        let Some(pair) = self.state.pair.get_untracked() else {
            return;
        };
        if !self.start() {
            return;
        }
        let id = match side {
            Side::A => pair.a.id,
            Side::B => pair.b.id,
        };
        mark(side, "gone");
        let (state, busy) = (self.state.clone(), self.busy);
        Timeout::new(320, move || {
            wasm_bindgen_futures::spawn_local(async move {
                api::deselect_and_skip(&state, id).await;
                busy.set(false);
            });
        })
        .forget();
    }

    /// The cards follow a drag: the one dragged towards leans in.
    fn lean(&self, dx: f64) {
        let t = (dx.abs() / DRAG_PICK).min(1.0);
        let (toward, away) = if dx > 0.0 {
            (Side::B, Side::A)
        } else {
            (Side::A, Side::B)
        };
        if let Some(card) = card(toward) {
            let _ = card.style().set_property(
                "transform",
                &format!(
                    "translateX({:.1}px) rotate({:.2}deg) scale({:.3})",
                    dx * 0.3,
                    dx * 0.03,
                    1.0 + t * 0.04
                ),
            );
            let _ = card.style().remove_property("opacity");
        }
        if let Some(card) = card(away) {
            let _ = card
                .style()
                .set_property("transform", &format!("scale({:.3})", 1.0 - t * 0.06));
            let _ = card
                .style()
                .set_property("opacity", &format!("{:.2}", 1.0 - t * 0.45));
        }
    }

    fn unlean(&self) {
        for side in [Side::A, Side::B] {
            if let Some(card) = card(side) {
                let _ = card.style().remove_property("transform");
                let _ = card.style().remove_property("opacity");
            }
        }
    }
}

#[component]
pub fn DuelsPage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let moves = Moves {
        state: state.clone(),
        busy: RwSignal::new(false),
    };
    install_keys(&moves);
    celebrate_milestones(&state);

    let selects = SiteConfig::get().require_selection;
    let stage = {
        let (state, moves) = (state.clone(), moves.clone());
        move || match (state.pair_status.get(), state.pair.get()) {
            (Some(reason), _) => view! { <Done reason=reason /> }.into_any(),
            (None, Some(pair)) => {
                view! { <Arena a=pair.a b=pair.b moves=moves.clone() /> }.into_any()
            }
            (None, None) => view! {
                <div class="arena arena-loading" aria-hidden="true">
                    <div class="duel-card ghost"></div>
                    <div class="duel-card ghost"></div>
                </div>
            }
            .into_any(),
        }
    };

    let moves_undo = moves.clone();
    let moves_skip = moves.clone();
    let state_bar = state.clone();
    let in_play = move || state_bar.pair.get().is_some();
    let state_undo = state.clone();

    view! {
        <div class="duels">
            <div class="duels-tools">
                <FocusChip />
                {selects.then(|| view! { <SelectionChip /> })}
            </div>
            <div class="duels-stage">{stage}</div>
            <div class="duels-bar" class:idle=move || !in_play()>
                <button
                    class="round-btn"
                    title=t().undo
                    disabled=move || state_undo.vote_history.get().is_empty()
                    on:click=move |_| moves_undo.undo()
                >
                    {icon(Icon::Undo)}
                </button>
                <Tally />
                <button class="round-btn" title=t().skip on:click=move |_| moves_skip.skip()>
                    {icon(Icon::Skip)}
                </button>
            </div>
        </div>
    }
}

#[component]
fn Arena(a: versus_shared::Film, b: versus_shared::Film, moves: Moves) -> impl IntoView {
    let cfg = SiteConfig::get();
    let question = cfg.duel_question.clone();
    let unseen_label = or_default(&cfg.unknown_label, t().skip_unknown);
    // A drag: where it started, how far it went, whether it went far
    // enough to be a drag rather than a tap.
    let start_x = StoredValue::new(None::<f64>);
    let dx = StoredValue::new(0.0f64);
    let dragged = StoredValue::new(false);

    let side_card = {
        let moves = moves.clone();
        move |side: Side, film: versus_shared::Film| {
            let pick = moves.clone();
            let unseen = moves.clone();
            let unseen_label = unseen_label.clone();
            view! {
                <div
                    class=match side { Side::A => "duel-card side-a", Side::B => "duel-card side-b" }
                    id=side.card()
                    role="button"
                    on:click=move |_| {
                        if !dragged.get_value() {
                            pick.pick(side);
                        }
                    }
                >
                    <CardFace film=film />
                    {cfg.require_selection.then(|| view! {
                        <button
                            class="unseen"
                            on:click=move |ev| {
                                ev.stop_propagation();
                                unseen.unseen(side);
                            }
                        >
                            {icon(Icon::EyeOff)}
                            <span>{unseen_label}</span>
                        </button>
                    })}
                </div>
            }
        }
    };

    let (down, over, up, cancel) = (moves.clone(), moves.clone(), moves.clone(), moves.clone());
    // A drag the pointer abandons puts the cards back.
    let abandon = move |moves: &Moves| {
        if start_x.get_value().is_some() {
            start_x.set_value(None);
            moves.unlean();
        }
    };
    view! {
        {(!question.is_empty()).then(|| view! { <h2 class="duel-question">{question}</h2> })}
        <div
            class="arena"
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                if down.busy.get_untracked() || ev.button() != 0 {
                    return;
                }
                start_x.set_value(Some(ev.client_x() as f64));
                dx.set_value(0.0);
                dragged.set_value(false);
            }
            on:pointermove=move |ev: web_sys::PointerEvent| {
                let Some(x0) = start_x.get_value() else { return };
                let d = ev.client_x() as f64 - x0;
                dx.set_value(d);
                if d.abs() > 8.0 {
                    dragged.set_value(true);
                    over.lean(d);
                }
            }
            on:pointerup=move |_| {
                if start_x.get_value().is_none() {
                    return;
                }
                start_x.set_value(None);
                let d = dx.get_value();
                if d.abs() >= DRAG_PICK {
                    up.pick(if d > 0.0 { Side::B } else { Side::A });
                } else if dragged.get_value() {
                    up.unlean();
                }
            }
            on:pointercancel=move |_| abandon(&cancel)
            on:pointerleave=move |_| abandon(&moves)
        >
            {side_card(Side::A, a)}
            <div class="vs" aria-hidden="true">"VS"</div>
            {side_card(Side::B, b)}
        </div>
    }
}

/// The votes cast, and how far the viewer is through their duels.
#[component]
fn Tally() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let progress = move || {
        let votes = state.vote_count.get();
        state
            .pairs_left
            .get()
            .map(|left| 100.0 * votes as f64 / (votes + left).max(1) as f64)
            .unwrap_or(0.0)
    };
    view! {
        <div class="tally">
            <div class="tally-count">
                <b>{move || state.vote_count.get()}</b>
                <span>{move || if state.vote_count.get() == 1 { t().vote_one } else { t().vote_many }}</span>
            </div>
            <div class="tally-bar">
                <i style:width=move || format!("{:.1}%", progress())></i>
            </div>
        </div>
    }
}

/// The item every duel features, or the random pairs, opening its picker.
#[component]
fn FocusChip() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    move || {
        let focus = state.focus_film_id.get().and_then(|id| {
            state
                .films
                .with(|films| films.iter().find(|f| f.id == id).map(|f| f.title.clone()))
        });
        let open = {
            let state = state.clone();
            move |_| state.panel.set(Some(Panel::Focus))
        };
        match focus {
            None => view! {
                <button class="chip" on:click=open>
                    {icon(Icon::Shuffle)}
                    <span>{t().focus_random}</span>
                    {icon(Icon::ChevronDown)}
                </button>
            }
            .into_any(),
            Some(title) => {
                let state = state.clone();
                view! {
                    <div class="chip chip-on">
                        <button class="chip-main" on:click=open>
                            {icon(Icon::Target)}
                            <span>{title}</span>
                        </button>
                        <button
                            class="chip-x"
                            title=t().focus_random
                            on:click=move |_| {
                                state.focus_film_id.set(None);
                                let s = state.clone();
                                wasm_bindgen_futures::spawn_local(async move { api::load_pair(&s).await });
                            }
                        >
                            {icon(Icon::X)}
                        </button>
                    </div>
                }
                .into_any()
            }
        }
    }
}

/// How many items the viewer compares, opening their selection.
#[component]
fn SelectionChip() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let count = move || state.selected_ids.with(|ids| ids.len());
    let panel = state.panel;
    view! {
        <button class="chip" on:click=move |_| panel.set(Some(Panel::Selection))>
            {icon(Icon::ListCheck)}
            <span>{move || say(t().selection_chip, &[("n", &count())])}</span>
        </button>
    }
}

/// What the stage shows once no pair is left to draw.
#[component]
fn Done(reason: PairDoneReason) -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let votes = state.vote_count.get_untracked();
    let focus_title = state.focus_film_id.get_untracked().and_then(|id| {
        state
            .films
            .with_untracked(|films| films.iter().find(|f| f.id == id).map(|f| f.title.clone()))
    });
    let (emoji, heading, text, action) = match reason {
        PairDoneReason::NotEnough => (
            "\u{1F440}",
            say(t().done_none_heading, &[]),
            say(t().done_none_text, &[]),
            say(t().done_none_action, &[]),
        ),
        PairDoneReason::FocusDone => (
            "\u{1F3AF}",
            fill(
                t().done_focus_heading,
                &[("title", &focus_title.unwrap_or_default())],
            ),
            String::new(),
            t().done_focus_action.to_string(),
        ),
        PairDoneReason::AllDone => (
            "\u{1F3C6}",
            t().done_all_heading.to_string(),
            say(t().done_all_text, &[("n", &votes)]),
            t().done_all_action.to_string(),
        ),
    };
    // Finishing every duel just now is worth a shower.
    if reason == PairDoneReason::AllDone && !state.vote_history.get_untracked().is_empty() {
        confetti::shower(120);
    }
    let act = move |_| match reason {
        PairDoneReason::NotEnough => state.panel.set(Some(Panel::Selection)),
        PairDoneReason::FocusDone => {
            state.focus_film_id.set(None);
            let s = state.clone();
            wasm_bindgen_futures::spawn_local(async move { api::load_pair(&s).await });
        }
        PairDoneReason::AllDone => state.navigate(Page::Board),
    };
    view! {
        <div class="done">
            <div class="done-emoji">{emoji}</div>
            <h2>{heading}</h2>
            {(!text.is_empty()).then(|| view! { <p>{text}</p> })}
            <button class="cta" on:click=act>{action}</button>
        </div>
    }
}

/// ← and → pick, ↓ skips, Z undoes, while the duels are in view.
fn install_keys(moves: &Moves) {
    let moves = moves.clone();
    let on_key =
        Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let state = &moves.state;
            let covered = state.sheet.get_untracked().is_some()
                || state.duel.get_untracked().is_some()
                || state.panel.get_untracked().is_some()
                || state.handoff.get_untracked().is_some();
            if state.page.get_untracked() != Page::Duels
                || covered
                || ev.ctrl_key()
                || ev.meta_key()
            {
                return;
            }
            let typing = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA"));
            if typing {
                return;
            }
            match ev.key().as_str() {
                "ArrowLeft" => moves.pick(Side::A),
                "ArrowRight" => moves.pick(Side::B),
                "ArrowDown" => moves.skip(),
                "z" | "Z" => moves.undo(),
                _ => return,
            }
            ev.prevent_default();
        });
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        let _ = doc.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
    }
    on_key.forget();
}

/// A shower of confetti and a word when a vote reaches a milestone.
fn celebrate_milestones(state: &AppState) {
    let state = state.clone();
    let last = StoredValue::new(None::<usize>);
    Effect::new(move || {
        let votes = state.vote_count.get();
        let before = last.get_value();
        last.set_value(Some(votes));
        if before == Some(votes.wrapping_sub(1)) && MILESTONES.contains(&votes) {
            confetti::shower(90);
            state.add_toast(fill(t().milestone, &[("n", &votes)]));
        }
    });
}

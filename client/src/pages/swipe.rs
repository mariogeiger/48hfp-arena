use crate::api;
use crate::components::poster::Poster;
use crate::state::{AppState, Page, PairDoneReason};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

fn get_cards() -> (Option<web_sys::HtmlElement>, Option<web_sys::HtmlElement>) {
    let doc = web_sys::window().unwrap().document().unwrap();
    let a = doc
        .get_element_by_id("film-a")
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
    let b = doc
        .get_element_by_id("film-b")
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
    (a, b)
}

/// Shared animation helper: guards on `animating`, applies CSS to both cards,
/// then fires `after` callback after `duration_ms`.
fn animate_cards_out(
    animating: &Rc<Cell<bool>>,
    transforms: (&str, &str),
    opacity: (&str, &str),
    transition: &str,
    duration_ms: u32,
    after: impl FnOnce() + 'static,
) {
    if animating.get() {
        return;
    }
    animating.set(true);
    let (film_a, film_b) = get_cards();
    if let Some(a) = film_a {
        let _ = a.style().set_property("transition", transition);
        let _ = a.style().set_property("transform", transforms.0);
        let _ = a.style().set_property("opacity", opacity.0);
    }
    if let Some(b) = film_b {
        let _ = b.style().set_property("transition", transition);
        let _ = b.style().set_property("transform", transforms.1);
        let _ = b.style().set_property("opacity", opacity.1);
    }
    let animating2 = animating.clone();
    gloo_timers::callback::Timeout::new(duration_ms, move || {
        animating2.set(false);
        after();
    })
    .forget();
}

#[derive(Clone)]
pub struct SwipeController {
    pick_left: Rc<dyn Fn()>,
    pick_right: Rc<dyn Fn()>,
    skip: Rc<dyn Fn()>,
    undo: Rc<dyn Fn()>,
    was_drag: Rc<dyn Fn() -> bool>,
}

impl SwipeController {
    pub fn pick_left(&self) {
        (self.pick_left)()
    }
    pub fn pick_right(&self) {
        (self.pick_right)()
    }
    pub fn skip(&self) {
        (self.skip)()
    }
    pub fn undo(&self) {
        (self.undo)()
    }
    pub fn was_drag(&self) -> bool {
        (self.was_drag)()
    }
}

/// Set up window-level pointer/keyboard event handlers for the swipe gesture.
/// Returns a `SwipeController` that the view uses for button handlers.
pub fn setup_swipe_controller(state: &AppState) -> SwipeController {
    let animating = Rc::new(Cell::new(false));
    let start_x = Rc::new(Cell::new(0.0f64));
    let dx = Rc::new(Cell::new(0.0f64));
    let dragging = Rc::new(Cell::new(false));
    let did_drag_rc = Rc::new(Cell::new(false));

    let window = web_sys::window().unwrap();

    let pick_side = {
        let state = state.clone();
        let animating = animating.clone();
        Rc::new(move |right: bool| {
            let pair = state.pair.get_untracked();
            let Some(pair) = pair else { return };

            let (w_id, l_id) = if right {
                (pair.b.id, pair.a.id)
            } else {
                (pair.a.id, pair.b.id)
            };
            let dir: f64 = if right { 1.0 } else { -1.0 };

            let ease = "transform 0.4s cubic-bezier(.4,0,.2,1), opacity 0.4s ease";
            let winner_transform =
                format!("translateX({}px) rotate({}deg)", dir * 300.0, dir * 12.0);
            let loser_transform = "translateY(200px) scale(0.7)".to_string();

            // For pick_side, card_a is winner when !right, card_b is winner when right
            let (a_transform, b_transform) = if right {
                (loser_transform.as_str(), winner_transform.as_str())
            } else {
                (winner_transform.as_str(), loser_transform.as_str())
            };

            let s = state.clone();
            // Need to own the strings for the closure
            let a_t = a_transform.to_string();
            let b_t = b_transform.to_string();
            let ease_owned = ease.to_string();
            let animating_ref = animating.clone();
            // Inline the animation since we need asymmetric transforms per card
            if animating_ref.get() {
                return;
            }
            animating_ref.set(true);
            let (film_a, film_b) = get_cards();
            if film_a.is_none() && film_b.is_none() {
                animating_ref.set(false);
                return;
            }
            if let Some(a) = film_a {
                let _ = a.style().set_property("transition", &ease_owned);
                let _ = a.style().set_property("transform", &a_t);
                let _ = a.style().set_property("opacity", "0");
            }
            if let Some(b) = film_b {
                let _ = b.style().set_property("transition", &ease_owned);
                let _ = b.style().set_property("transform", &b_t);
                let _ = b.style().set_property("opacity", "0");
            }
            let animating2 = animating_ref.clone();
            gloo_timers::callback::Timeout::new(350, move || {
                animating2.set(false);
                wasm_bindgen_futures::spawn_local(async move {
                    api::cast_vote(&s, w_id, l_id).await;
                });
            })
            .forget();
        })
    };

    // pointerdown
    {
        let dragging = dragging.clone();
        let start_x = start_x.clone();
        let dx = dx.clone();
        let did_drag_rc = did_drag_rc.clone();
        let animating = animating.clone();
        let state = state.clone();
        let on_down =
            Closure::<dyn Fn(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
                if animating.get() || state.page.get_untracked() != Page::Swipe {
                    return;
                }
                let target: web_sys::Element = e.target().unwrap().dyn_into().unwrap();
                if target.closest("#swipe-arena").ok().flatten().is_none() {
                    return;
                }
                if target.closest(".deselect-btn").ok().flatten().is_some() {
                    return;
                }
                start_x.set(e.client_x() as f64);
                dx.set(0.0);
                dragging.set(true);
                did_drag_rc.set(false);
                let (a, b) = get_cards();
                if let Some(a) = a {
                    let _ = a.style().set_property("transition", "none");
                }
                if let Some(b) = b {
                    let _ = b.style().set_property("transition", "none");
                }
            });
        window
            .add_event_listener_with_callback("pointerdown", on_down.as_ref().unchecked_ref())
            .unwrap();
        on_down.forget();
    }

    // pointermove
    {
        let dragging = dragging.clone();
        let start_x = start_x.clone();
        let dx = dx.clone();
        let did_drag_rc = did_drag_rc.clone();
        let on_move =
            Closure::<dyn Fn(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
                if !dragging.get() {
                    return;
                }
                let d = e.client_x() as f64 - start_x.get();
                dx.set(d);
                if d.abs() > 5.0 {
                    did_drag_rc.set(true);
                }
                let (fa, fb) = get_cards();
                let (Some(fa), Some(fb)) = (fa, fb) else {
                    return;
                };
                let t = (d.abs() / 150.0).min(1.0);
                let left = d < 0.0;
                let (winner, loser) = if left { (fa, fb) } else { (fb, fa) };
                let dir: f64 = if left { -1.0 } else { 1.0 };
                let _ = winner.style().set_property(
                    "transform",
                    &format!(
                        "translateX({}px) rotate({}deg)",
                        dir * t * 100.0,
                        dir * t * 5.0
                    ),
                );
                let _ = winner
                    .style()
                    .set_property("opacity", &format!("{}", 1.0 - t * 0.3));
                let _ = loser.style().set_property(
                    "transform",
                    &format!("translateY({}px) scale({})", t * 80.0, 1.0 - t * 0.15),
                );
                let _ = loser
                    .style()
                    .set_property("opacity", &format!("{}", 1.0 - t * 0.4));
            });
        window
            .add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref())
            .unwrap();
        on_move.forget();
    }

    // pointerup
    {
        let dragging = dragging.clone();
        let dx = dx.clone();
        let pick_side = pick_side.clone();
        let on_up =
            Closure::<dyn Fn(web_sys::PointerEvent)>::new(move |_: web_sys::PointerEvent| {
                if !dragging.get() {
                    return;
                }
                dragging.set(false);
                let d = dx.get();
                if d.abs() > 80.0 {
                    pick_side(d > 0.0);
                } else {
                    let ease = "transform 0.3s ease, opacity 0.3s ease";
                    let (a, b) = get_cards();
                    if let Some(a) = a {
                        let _ = a.style().set_property("transition", ease);
                        let _ = a.style().remove_property("transform");
                        let _ = a.style().remove_property("opacity");
                    }
                    if let Some(b) = b {
                        let _ = b.style().set_property("transition", ease);
                        let _ = b.style().remove_property("transform");
                        let _ = b.style().remove_property("opacity");
                    }
                }
            });
        window
            .add_event_listener_with_callback("pointerup", on_up.as_ref().unchecked_ref())
            .unwrap();
        on_up.forget();
    }

    // Keyboard
    {
        let pick_side = pick_side.clone();
        let state = state.clone();
        let on_key =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if state.page.get_untracked() != Page::Swipe {
                    return;
                }
                match e.key().as_str() {
                    "ArrowLeft" => pick_side(false),
                    "ArrowRight" => pick_side(true),
                    _ => {}
                }
            });
        let doc = web_sys::window().unwrap().document().unwrap();
        doc.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
            .unwrap();
        on_key.forget();
    }

    // Build closures for the controller
    let pick_left: Rc<dyn Fn()> = {
        let ps = pick_side.clone();
        Rc::new(move || ps(false))
    };

    let pick_right: Rc<dyn Fn()> = {
        let ps = pick_side.clone();
        Rc::new(move || ps(true))
    };

    let skip: Rc<dyn Fn()> = {
        let state = state.clone();
        let animating = animating.clone();
        Rc::new(move || {
            let s = state.clone();
            animate_cards_out(
                &animating,
                ("translateY(60px) scale(0.9)", "translateY(60px) scale(0.9)"),
                ("0", "0"),
                "transform 0.3s ease, opacity 0.3s ease",
                300,
                move || {
                    wasm_bindgen_futures::spawn_local(async move {
                        api::load_pair(&s).await;
                    });
                },
            );
        })
    };

    let undo: Rc<dyn Fn()> = {
        let state = state.clone();
        let animating = animating.clone();
        Rc::new(move || {
            if state.vote_history.get_untracked().is_empty() {
                return;
            }
            let s = state.clone();
            animate_cards_out(
                &animating,
                ("scale(0.8)", "scale(0.8)"),
                ("0", "0"),
                "transform 0.2s ease, opacity 0.2s ease",
                200,
                move || {
                    wasm_bindgen_futures::spawn_local(async move {
                        api::undo_vote(&s).await;
                    });
                },
            );
        })
    };

    let was_drag: Rc<dyn Fn() -> bool> = {
        let did_drag_rc = did_drag_rc.clone();
        Rc::new(move || did_drag_rc.get())
    };

    SwipeController {
        pick_left,
        pick_right,
        skip,
        undo,
        was_drag,
    }
}

#[component]
pub fn SwipePage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Set up gesture controller once, wrapped in SendWrapper for Leptos reactivity
    let ctrl = SendWrapper::new(Rc::new(RefCell::new(None::<SwipeController>)));

    Effect::new({
        let state = state.clone();
        let ctrl = ctrl.clone();
        move || {
            let c = setup_swipe_controller(&state);
            *ctrl.borrow_mut() = Some(c);
        }
    });

    // Entrance animation when pair changes: start scaled down, animate to full
    Effect::new({
        let state = state.clone();
        move || {
            let _pair = state.pair.get(); // track pair changes
            // Use requestAnimationFrame to run after Leptos has patched the DOM
            let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(|| {
                let doc = web_sys::window().unwrap().document().unwrap();
                for id in &["film-a", "film-b"] {
                    if let Some(el) = doc.get_element_by_id(id)
                        && let Ok(el) = el.dyn_into::<web_sys::HtmlElement>()
                    {
                        // Start from scaled-down state
                        let _ = el.style().set_property("transition", "none");
                        let _ = el.style().set_property("transform", "scale(0.85)");
                        let _ = el.style().set_property("opacity", "0");
                        // Force reflow
                        let _ = el.offset_height();
                        // Animate to full
                        let _ = el.style().set_property(
                            "transition",
                            "transform 0.3s cubic-bezier(.4,0,.2,1), opacity 0.3s ease",
                        );
                        let _ = el.style().set_property("transform", "scale(1)");
                        let _ = el.style().set_property("opacity", "1");
                    }
                }
            });
            let _ = web_sys::window()
                .unwrap()
                .request_animation_frame(cb.as_ref().unchecked_ref());
            cb.forget();
        }
    });

    // Focus picker
    let selected_films = Memo::new(move |_| {
        let films = state.films.get();
        let selected = state.selected_ids.get();
        films
            .into_iter()
            .filter(|f| selected.contains(&f.id))
            .collect::<Vec<_>>()
    });

    view! {
        <div id="focus-picker-container">
            <FocusPicker films=selected_films />
        </div>
        <div class="swipe-container" id="swipe-container">
            {
                let ctrl = ctrl.clone();
                move || {
                    let pair_status = state.pair_status.get();
                    let pair = state.pair.get();

                    if let Some(reason) = pair_status {
                        let vote_count = state.vote_count.get();
                        let (heading, msg, action_label) = match reason {
                            PairDoneReason::NotEnough => (
                                "Select films first",
                                "Go back and pick at least 2 films you've watched.".to_string(),
                                "Select Films",
                            ),
                            PairDoneReason::FocusDone => (
                                "All done!",
                                "You've compared this film against all others.".to_string(),
                                "Compare All",
                            ),
                            PairDoneReason::AllDone => (
                                "All done!",
                                format!("You've compared all possible pairs. ({} votes cast)", vote_count),
                                "View Leaderboard",
                            ),
                        };
                        let state2 = state.clone();
                        view! {
                            <div class="swipe-done">
                                <h2>{heading}</h2>
                                <p>{msg}</p>
                                <button on:click=move |_| {
                                    match reason {
                                        PairDoneReason::NotEnough => state2.navigate(Page::Select),
                                        PairDoneReason::FocusDone => {
                                            state2.focus_film_id.set(None);
                                            let s = state2.clone();
                                            wasm_bindgen_futures::spawn_local(async move {
                                                api::load_pair(&s).await;
                                            });
                                        }
                                        PairDoneReason::AllDone => state2.navigate(Page::Board),
                                    }
                                }>{action_label}</button>
                            </div>
                        }.into_any()
                    } else if let Some(pair) = pair {
                        let a = pair.a.clone();
                        let b = pair.b.clone();
                        let a_id = a.id;
                        let b_id = b.id;
                        let a_poster = a.poster_url.clone();
                        let b_poster = b.poster_url.clone();
                        let a_title = a.title.clone();
                        let b_title = b.title.clone();
                        let a_meta = format!("{}{}", a.team, if a.city.is_empty() { String::new() } else { format!(" \u{00B7} {}", a.city) });
                        let b_meta = format!("{}{}", b.team, if b.city.is_empty() { String::new() } else { format!(" \u{00B7} {}", b.city) });

                        let state_da = state.clone();
                        let state_db = state.clone();

                        // Clone ctrl for each closure that needs it
                        let ctrl_a = ctrl.clone();
                        let ctrl_a2 = ctrl.clone();
                        let ctrl_b = ctrl.clone();
                        let ctrl_b2 = ctrl.clone();
                        let ctrl_left = ctrl.clone();
                        let ctrl_right = ctrl.clone();
                        let ctrl_undo = ctrl.clone();
                        let ctrl_skip = ctrl.clone();

                        view! {
                            <div>
                                <div class="vs-badge">"VS"</div>
                                <div class="swipe-arena" id="swipe-arena">
                                    <div
                                        class="film-card"
                                        id="film-a"
                                        on:click=move |e| {
                                            if ctrl_a.borrow().as_ref().map(|c| c.was_drag()).unwrap_or(false) { return; }
                                            e.stop_propagation();
                                            if let Some(c) = ctrl_a2.borrow().as_ref() { c.pick_left(); }
                                        }
                                    >
                                        <Poster url=a_poster />
                                        <div class="title">{a_title}</div>
                                        <div class="meta">{a_meta}</div>
                                        <button
                                            class="deselect-btn"
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                let s = state_da.clone();
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    api::deselect_and_skip(&s, a_id).await;
                                                });
                                            }
                                        >
                                            "Haven't seen it"
                                        </button>
                                    </div>
                                    <div
                                        class="film-card"
                                        id="film-b"
                                        on:click=move |e| {
                                            if ctrl_b.borrow().as_ref().map(|c| c.was_drag()).unwrap_or(false) { return; }
                                            e.stop_propagation();
                                            if let Some(c) = ctrl_b2.borrow().as_ref() { c.pick_right(); }
                                        }
                                    >
                                        <Poster url=b_poster />
                                        <div class="title">{b_title}</div>
                                        <div class="meta">{b_meta}</div>
                                        <button
                                            class="deselect-btn"
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                let s = state_db.clone();
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    api::deselect_and_skip(&s, b_id).await;
                                                });
                                            }
                                        >
                                            "Haven't seen it"
                                        </button>
                                    </div>
                                </div>
                                <div class="swipe-buttons">
                                    <button class="swipe-arrow-btn" title="Pick left"
                                        on:click=move |_| { if let Some(c) = ctrl_left.borrow().as_ref() { c.pick_left(); } }
                                    >"\u{2190}"</button>
                                    <button class="swipe-arrow-btn" title="Pick right"
                                        on:click=move |_| { if let Some(c) = ctrl_right.borrow().as_ref() { c.pick_right(); } }
                                    >"\u{2192}"</button>
                                </div>
                                <div class="swipe-progress">
                                    {move || format!("{} comparisons made", state.vote_count.get())}
                                </div>
                                <div class="swipe-bottom-actions">
                                    <button
                                        class="undo-btn"
                                        disabled=move || state.vote_history.get().is_empty()
                                        on:click=move |_| { if let Some(c) = ctrl_undo.borrow().as_ref() { c.undo(); } }
                                    >
                                        "Undo"
                                    </button>
                                    <button
                                        class="skip-btn"
                                        on:click=move |_| { if let Some(c) = ctrl_skip.borrow().as_ref() { c.skip(); } }
                                    >
                                        "Skip"
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }
            }
        </div>
    }
}

#[component]
fn FocusPicker(films: Memo<Vec<filmrank_shared::Film>>) -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let state2 = state.clone();

    view! {
        <div class="focus-picker">
            <label for="focus-film">"Compare against:"</label>
            <select
                id="focus-film"
                on:change={
                    let state = state.clone();
                    move |ev| {
                        let val = event_target_value(&ev);
                        let id = val.parse::<usize>().ok();
                        state.focus_film_id.set(id);
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            api::load_pair(&s).await;
                        });
                    }
                }
                prop:value=move || {
                    state2.focus_film_id.get().map(|id| id.to_string()).unwrap_or_default()
                }
            >
                <option value="">"Random pairs"</option>
                {move || {
                    films.get().into_iter().map(|film| {
                        view! {
                            <option value={film.id.to_string()}>{film.title.clone()}</option>
                        }
                    }).collect_view()
                }}
            </select>
        </div>
    }
}

mod api;
mod components;
mod pages;
mod state;

use leptos::prelude::*;
use state::{AppState, Page};
use wasm_bindgen::JsCast;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let state = AppState::new();
    provide_context(state.clone());

    // Boot: fetch films + board, init SSE
    let state_boot = state.clone();
    Effect::new(move || {
        let s = state_boot.clone();
        wasm_bindgen_futures::spawn_local(async move {
            api::boot(&s).await;
        });
    });

    // Init SSE
    let state_sse = state.clone();
    Effect::new(move || {
        api::init_vote_stream(&state_sse);
    });

    // Visibility change handler
    let state_vis = state.clone();
    Effect::new(move || {
        let s = state_vis.clone();
        let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            let doc = web_sys::window().unwrap().document().unwrap();
            if doc.visibility_state() == web_sys::VisibilityState::Visible {
                let s2 = s.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let page = s2.page.get_untracked();
                    if page == Page::Board {
                        api::load_board(&s2).await;
                    }
                    if page == Page::More {
                        api::load_more(&s2).await;
                    }
                });
            }
        });
        let doc = web_sys::window().unwrap().document().unwrap();
        doc.add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    });

    // Rank swap detection — toast when films overtake each other
    {
        let state = state.clone();
        let prev_board: std::cell::RefCell<Vec<state::LeaderboardEntry>> =
            std::cell::RefCell::new(Vec::new());
        Effect::new(move || {
            let board = state.board.get();
            let prev = prev_board.borrow();
            if !prev.is_empty() && !board.is_empty() {
                let old_rank: std::collections::HashMap<usize, usize> = prev
                    .iter()
                    .enumerate()
                    .map(|(i, item)| (item.film_id, i))
                    .collect();
                let mut notified = std::collections::HashSet::new();
                let mut swaps = Vec::new();
                for (i, film) in board.iter().enumerate() {
                    let Some(&old_idx) = old_rank.get(&film.film_id) else {
                        continue;
                    };
                    if old_idx <= i || notified.contains(&film.film_id) {
                        continue;
                    }
                    if let Some(displaced) = prev.get(i)
                        && !notified.contains(&displaced.film_id)
                    {
                        notified.insert(film.film_id);
                        notified.insert(displaced.film_id);
                        swaps.push(format!(
                                "\u{2B06}\u{FE0F} <strong>{}</strong> overtook <strong>{}</strong> \u{2192} #{}",
                                api::html_escape(&film.title),
                                api::html_escape(&displaced.title),
                                i + 1,
                            ));
                    }
                }
                if !swaps.is_empty() {
                    state.add_toast(swaps.join("<br>"));
                }
            }
            drop(prev);
            *prev_board.borrow_mut() = board;
        });
    }

    let page = state.page;

    view! {
        <components::nav::Nav />

        <div id="page-select" class="page" class:active=move || page.get() == Page::Select>
            <pages::select::SelectPage />
        </div>
        <div id="page-swipe" class="page" class:active=move || page.get() == Page::Swipe>
            <pages::swipe::SwipePage />
        </div>
        <div id="page-board" class="page" class:active=move || page.get() == Page::Board>
            <pages::board::BoardPage />
        </div>
        <div id="page-more" class="page" class:active=move || page.get() == Page::More>
            <pages::more::MorePage />
        </div>

        <components::toast::ToastContainer />

        <components::overlays::WelcomeOverlay />
        <components::overlays::BannedOverlay />
    }
}

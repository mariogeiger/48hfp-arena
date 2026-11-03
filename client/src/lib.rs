//! The app: it fetches the site's configuration, speaks its language, and
//! mounts the header, the tabs, the pages and what opens over them.

mod api;
mod components;
mod google;
mod i18n;
mod identity;
mod pages;
mod state;
mod timeout;
mod wiki;

use leptos::prelude::*;
use state::{AppState, Page};
use wasm_bindgen::JsCast;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    // Fetch per-site config first, install it, then mount.
    wasm_bindgen_futures::spawn_local(async {
        let config = api::fetch_config().await;
        i18n::set_speech(config.lang, &config.noun, &config.noun_plural);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            doc.set_title(&config.display_name);
            if let Some(root) = doc.document_element() {
                let _ = root.set_attribute("lang", config.lang.tag());
            }
        }
        config.install();
        leptos::mount::mount_to_body(|| view! { <App /> });
    });
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

    // The address changed without a reload: follow it.
    let state_hash = state.clone();
    Effect::new(move || {
        let s = state_hash.clone();
        let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || s.follow_hash());
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
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
                    if page != Page::Duels {
                        s2.load_page(page).await;
                    }
                });
            }
        });
        let doc = web_sys::window().unwrap().document().unwrap();
        doc.add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    });

    // On the ranking, a toast tells which items overtook which.
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
                        let line = i18n::fill(
                            i18n::t().overtook,
                            &[
                                ("a", &api::html_escape(&film.title)),
                                ("b", &api::html_escape(&displaced.title)),
                                ("n", &(i + 1)),
                            ],
                        );
                        swaps.push(format!("\u{2B06}\u{FE0F} {line}"));
                    }
                }
                if !swaps.is_empty() && state.page.get_untracked() == Page::Board {
                    state.add_toast(swaps.join("<br>"));
                }
            }
            drop(prev);
            *prev_board.borrow_mut() = board;
        });
    }

    close_on_escape(&state);
    hold_page_under_sheets(&state);

    let page = state.page;
    view! {
        <components::shell::Header />
        <components::shell::Tabs />
        <main class="main">
            <div class="page page-duels" class:active=move || page.get() == Page::Duels>
                <pages::duels::DuelsPage />
            </div>
            <div class="page page-board" class:active=move || page.get() == Page::Board>
                <pages::board::BoardPage />
            </div>
            <div class="page page-me" class:active=move || page.get() == Page::Me>
                <pages::me::MePage />
            </div>
            {move || (page.get() == Page::About).then(|| view! {
                <div class="page page-about active">
                    <pages::about::AboutPage />
                </div>
            })}
        </main>

        <components::sheet::ItemSheet />
        <components::duel::DuelSheet />
        <components::pickers::SelectionSheet />
        <components::pickers::FocusSheet />
        <components::account::AccountSheet />
        <components::toast::ToastContainer />

        <components::overlays::WelcomeOverlay />
        <components::overlays::BannedOverlay />
        <components::account::HandoffOverlay />
    }
}

/// Escape closes what lies over the page, the topmost first.
fn close_on_escape(state: &AppState) {
    let state = state.clone();
    let on_key = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(
        move |ev: web_sys::KeyboardEvent| {
            if ev.key() != "Escape" {
                return;
            }
            if state.duel.get_untracked().is_some() {
                state.duel.set(None);
            } else if state.panel.get_untracked().is_some() {
                state.panel.set(None);
            } else if state.sheet.get_untracked().is_some() {
                state.close_sheet();
            }
        },
    );
    let doc = web_sys::window().unwrap().document().unwrap();
    doc.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
        .unwrap();
    on_key.forget();
}

/// The page stays still under an open sheet.
fn hold_page_under_sheets(state: &AppState) {
    let (sheet, duel, panel) = (state.sheet, state.duel, state.panel);
    Effect::new(move || {
        let covered = sheet.get().is_some() || duel.get().is_some() || panel.get().is_some();
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.class_list().toggle_with_force("sheet-open", covered);
        }
    });
}

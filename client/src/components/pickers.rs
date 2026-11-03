//! The two lists the duels open: the items the viewer knows, which the
//! duels draw from, and the item every duel features, if any.

use crate::api;
use crate::components::card::meta;
use crate::components::frame::SheetFrame;
use crate::components::icons::{Icon, icon};
use crate::components::poster::Poster;
use crate::i18n::{say, t};
use crate::state::{AppState, Panel, SiteConfig};
use leptos::prelude::*;
use versus_shared::Film;

fn matches(film: &Film, query: &str) -> bool {
    query.is_empty()
        || [&film.title, &film.team, &film.city]
            .iter()
            .any(|field| field.to_lowercase().contains(query))
}

/// A search field that narrows a list to what `query` holds.
#[component]
fn SearchBox(query: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="search">
            {icon(Icon::Search)}
            <input
                type="search"
                placeholder=say(t().select_search, &[])
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
            />
        </label>
    }
}

/// A row's thumbnail, on a site with images.
fn thumb(film: &Film) -> impl IntoView + use<> {
    SiteConfig::get().has_image.then(|| {
        view! {
            <span class="pick-thumb">
                <Poster url=film.poster_url.clone() title=film.title.clone() id=film.id />
            </span>
        }
    })
}

#[component]
pub fn SelectionSheet() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    move || {
        (state.panel.get() == Some(Panel::Selection)).then(|| {
            let state = state.clone();
            view! { <Selection state=state /> }
        })
    }
}

#[component]
fn Selection(state: AppState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    // Each tap is kept at once, so that a reload keeps it. The server, which
    // draws the pairs, hears of the change when the sheet closes, however it
    // closes, and only then is the next pair drawn.
    let picked_before = state.selected_ids.get_untracked();
    {
        let state = state.clone();
        on_cleanup(move || {
            if state.selected_ids.get_untracked() != picked_before {
                wasm_bindgen_futures::spawn_local(async move {
                    api::save_selection(&state).await;
                    api::load_pair(&state).await;
                });
            }
        });
    }
    let save = {
        let state = state.clone();
        move || state.store_selection()
    };
    let visible = {
        let state = state.clone();
        Memo::new(move |_| {
            let q = query.get().to_lowercase();
            state.films.with(|films| {
                films
                    .iter()
                    .filter(|f| matches(f, &q))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    };
    let set_all = {
        let (state, save) = (state.clone(), save.clone());
        move |on: bool| {
            let shown: Vec<usize> = visible.get_untracked().iter().map(|f| f.id).collect();
            state.selected_ids.update(|ids| {
                for id in shown {
                    if on {
                        ids.insert(id);
                    } else {
                        ids.remove(&id);
                    }
                }
            });
            save();
        }
    };
    let set_none = set_all.clone();
    let selected = state.selected_ids;
    let count = move || selected.with(|ids| ids.len());
    let panel = state.panel;
    let close = Callback::new(move |_| panel.set(None));

    view! {
        <SheetFrame on_close=close class="list-sheet">
            <h2>{say(t().selection_title, &[])}</h2>
            <p class="sheet-hint">{say(t().selection_hint, &[])}</p>
            <div class="list-tools">
                <SearchBox query=query />
                <button class="mini-btn" on:click=move |_| set_all(true)>{t().select_all}</button>
                <button class="mini-btn" on:click=move |_| set_none(false)>{t().deselect_all}</button>
            </div>
            <div class="pick-list">
                {move || {
                    let save = save.clone();
                    visible
                        .get()
                        .into_iter()
                        .map(|film| {
                            let id = film.id;
                            let save = save.clone();
                            view! {
                                <button
                                    class="pick-row"
                                    class:on=move || selected.with(|ids| ids.contains(&id))
                                    on:click=move |_| {
                                        selected.update(|ids| {
                                            if !ids.remove(&id) {
                                                ids.insert(id);
                                            }
                                        });
                                        save();
                                    }
                                >
                                    <span class="pick-check">{icon(Icon::Check)}</span>
                                    {thumb(&film)}
                                    <span class="pick-text">
                                        <span class="pick-title">{film.title.clone()}</span>
                                        <span class="pick-meta">{meta(&film)}</span>
                                    </span>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="sheet-footer">
                <button class="cta" disabled=move || count() < 2 on:click=move |_| close.run(())>
                    {move || {
                        let n = count();
                        if n < 2 {
                            say(t().select_too_few, &[])
                        } else {
                            say(t().select_count, &[("n", &n)])
                        }
                    }}
                </button>
            </div>
        </SheetFrame>
    }
}

#[component]
pub fn FocusSheet() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    move || {
        (state.panel.get() == Some(Panel::Focus)).then(|| {
            let state = state.clone();
            view! { <Focus state=state /> }
        })
    }
}

#[component]
fn Focus(state: AppState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let selects = SiteConfig::get().require_selection;
    // The items the duels draw from.
    let visible = {
        let state = state.clone();
        Memo::new(move |_| {
            let q = query.get().to_lowercase();
            let selected = state.selected_ids.get();
            state.films.with(|films| {
                films
                    .iter()
                    .filter(|f| !selects || selected.contains(&f.id))
                    .filter(|f| matches(f, &q))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    };
    let focus = state.focus_film_id;
    let choose = {
        let state = state.clone();
        move |id: Option<usize>| {
            focus.set(id);
            state.panel.set(None);
            let s = state.clone();
            wasm_bindgen_futures::spawn_local(async move { api::load_pair(&s).await });
        }
    };
    let choose_random = choose.clone();
    let state_close = state.clone();
    let close = Callback::new(move |_| state_close.panel.set(None));

    view! {
        <SheetFrame on_close=close class="list-sheet">
            <h2>{say(t().focus_title, &[])}</h2>
            <div class="list-tools">
                <SearchBox query=query />
            </div>
            <div class="pick-list">
                <button
                    class="pick-row"
                    class:on=move || focus.get().is_none()
                    on:click=move |_| choose_random(None)
                >
                    <span class="pick-icon">{icon(Icon::Shuffle)}</span>
                    <span class="pick-text">
                        <span class="pick-title">{t().focus_random}</span>
                    </span>
                </button>
                {move || {
                    let choose = choose.clone();
                    visible
                        .get()
                        .into_iter()
                        .map(|film| {
                            let id = film.id;
                            let choose = choose.clone();
                            view! {
                                <button
                                    class="pick-row"
                                    class:on=move || focus.get() == Some(id)
                                    on:click=move |_| choose(Some(id))
                                >
                                    {thumb(&film)}
                                    <span class="pick-text">
                                        <span class="pick-title">{film.title.clone()}</span>
                                        <span class="pick-meta">{meta(&film)}</span>
                                    </span>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </SheetFrame>
    }
}

//! The sheet an item opens in, over the page: the item itself, with
//! its video, and on an editable site its edit form, its history and, for a
//! proposed item, its review.

use crate::components::form::ItemForm;
use crate::components::frame::SheetFrame;
use crate::components::history::ItemHistory;
use crate::components::poster::Poster;
use crate::components::video::Versions;
use crate::i18n::{or_default, say, t};
use crate::state::{AppState, Sheet, SiteConfig};
use leptos::prelude::*;
use versus_shared::{Film, Proposal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    View,
    Edit,
    History,
}

/// The item `id`, published or proposed, with its proposal if it is one.
fn find(state: &AppState, id: usize) -> Option<(Film, Option<Proposal>)> {
    if let Some(film) = state
        .films
        .with(|films| films.iter().find(|f| f.id == id).cloned())
    {
        return Some((film, None));
    }
    state.proposals.with(|proposals| {
        proposals
            .iter()
            .find(|p| p.film.id == id)
            .map(|p| (p.film.clone(), Some(p.clone())))
    })
}

#[component]
pub fn ItemSheet() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let mode = RwSignal::new(Mode::View);

    // Each item opens on its view.
    Effect::new({
        let state = state.clone();
        move || {
            state.sheet.track();
            mode.set(Mode::View);
        }
    });

    let state_close = state.clone();
    let close = move || state_close.close_sheet();

    move || {
        let sheet = state.sheet.get()?;
        let close = close.clone();
        let body = match sheet {
            Sheet::New => {
                let state = state.clone();
                view! {
                    <h2>{say(t().add_item, &[])}</h2>
                    <p class="sheet-hint">{t().form_new_hint}</p>
                    <ItemForm
                        film=None
                        on_done=Callback::new(move |id: Option<usize>| match id {
                            Some(id) => state.open_sheet(Sheet::Item(id)),
                            None => state.close_sheet(),
                        })
                    />
                }
                .into_any()
            }
            Sheet::Item(id) => {
                let state = state.clone();
                (move || {
                    // Only the view follows the item live: a change arriving
                    // meanwhile must not rebuild the form or the history.
                    let mode_now = mode.get();
                    let found = if mode_now == Mode::View {
                        find(&state, id)
                    } else {
                        untrack(|| find(&state, id))
                    };
                    match (found, mode_now) {
                        (None, _) => {
                            view! { <p class="sheet-hint">{t().error_unknown_film}</p> }.into_any()
                        }
                        (Some((film, _)), Mode::Edit) => view! {
                            <ItemForm
                                film=Some(film)
                                on_done=Callback::new(move |_| mode.set(Mode::View))
                            />
                        }
                        .into_any(),
                        (Some((film, _)), Mode::History) => view! {
                            <button class="sheet-back" on:click=move |_| mode.set(Mode::View)>
                                {format!("\u{2190} {}", t().sheet_back)}
                            </button>
                            <h2>{film.title.clone()}</h2>
                            <ItemHistory film=film.id />
                        }
                        .into_any(),
                        (Some((film, proposal)), Mode::View) => view! {
                            <ItemView film=film proposal=proposal mode=mode />
                        }
                        .into_any(),
                    }
                })
                .into_any()
            }
        };
        Some(view! {
            <SheetFrame on_close=Callback::new(move |_| close()) class="item-sheet">
                {body}
            </SheetFrame>
        })
    }
}

#[component]
fn ItemView(film: Film, proposal: Option<Proposal>, mode: RwSignal<Mode>) -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let link_label = or_default(&cfg.link_label, t().open_link);
    let meta = [film.team.as_str(), film.city.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{00B7} ");
    let id = film.id;
    let error = RwSignal::new(None::<String>);
    // Where the ranking puts it, if it does.
    let board = state.board;
    let standing = move || {
        board.with(|board| {
            let (rank, entry) = board.iter().enumerate().find(|(_, e)| e.film_id == id)?;
            Some(view! {
                <div class="item-standing">
                    <span class="standing-rank">{format!("#{}", rank + 1)}</span>
                    <span>{format!("{} pts", entry.rating.round() as i64)}</span>
                    <span>{crate::i18n::record(entry.wins, entry.ties, entry.losses)}</span>
                </div>
            })
        })
    };

    let review = move |approve: bool| {
        let state = state.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match crate::wiki::review(&state, id, approve).await {
                Ok(()) => {
                    state.add_toast(if approve { t().approved } else { t().rejected }.into());
                    if !approve {
                        state.close_sheet();
                    }
                }
                Err(code) => {
                    error.try_set(Some(t().error(&code).into()));
                }
            }
        });
    };
    let review_approve = review.clone();

    let review_block = proposal.map(|p| {
        if p.is_yours {
            view! { <p class="sheet-notice">{t().proposal_yours}</p> }.into_any()
        } else {
            view! {
                <div class="sheet-actions">
                    <button class="primary" on:click=move |_| review_approve(true)>{t().approve}</button>
                    <button class="danger" on:click=move |_| review(false)>{t().reject}</button>
                </div>
            }
            .into_any()
        }
    });

    view! {
        <div class="item-view">
            {cfg.has_image.then(|| view! {
                <div class="item-poster">
                    <Poster url=film.poster_url.clone() title=film.title.clone() id=film.id />
                </div>
            })}
            <div class="item-text">
                <h2>{film.title.clone()}</h2>
                <p class="item-meta">{meta}</p>
                {standing}
                {cfg.editable.then(|| view! {
                    <button class="edit-pill" on:click=move |_| mode.set(Mode::Edit)>
                        <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true">
                            <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zm17.71-10.21a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" />
                        </svg>
                        {t().sheet_edit}
                    </button>
                })}
                {review_block}
            </div>
        </div>
        {(!film.synopsis.is_empty()).then(|| view! { <p class="item-synopsis">{film.synopsis.clone()}</p> })}
        <Versions videos=film.videos.clone() link_label=link_label />
        {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
        {cfg.editable.then(|| view! {
            <button class="history-link" on:click=move |_| mode.set(Mode::History)>
                <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden="true">
                    <path fill="currentColor" d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6a7 7 0 1 1 2.05 4.95l-1.42 1.42A9 9 0 1 0 13 3zm-1 5v5l4.25 2.52.77-1.28-3.52-2.09V8H12z" />
                </svg>
                {t().sheet_history}
            </button>
        })}
    }
}

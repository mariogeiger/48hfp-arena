//! The sheet an item opens in, over the leaderboard: the item itself, with
//! its video, and on an editable site its edit form, its history and, for a
//! proposed item, its review.

use crate::components::form::ItemForm;
use crate::components::history::ItemHistory;
use crate::components::poster::Poster;
use crate::components::video::Video;
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
        let close_backdrop = close.clone();
        let body = match sheet {
            Sheet::New => {
                let state = state.clone();
                view! {
                    <h2>{say(t().form_new_heading, &[])}</h2>
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
            <div
                class="sheet-backdrop"
                on:click=move |ev| {
                    if ev.target() == ev.current_target() {
                        close_backdrop();
                    }
                }
            >
                <div class="sheet" role="dialog" aria-modal="true">
                    <button class="sheet-close" title=t().sheet_close on:click=move |_| close()>
                        "\u{00D7}"
                    </button>
                    {body}
                </div>
            </div>
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
            {cfg.has_image.then(|| view! { <div class="item-poster"><Poster url=film.poster_url.clone() /></div> })}
            <div class="item-text">
                <h2>{film.title.clone()}</h2>
                <p class="item-meta">{meta}</p>
                {review_block}
            </div>
        </div>
        {(!film.synopsis.is_empty()).then(|| view! { <p class="item-synopsis">{film.synopsis.clone()}</p> })}
        {(!film.video_url.is_empty()).then(|| view! { <Video url=film.video_url.clone() link_label=link_label /> })}
        {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
        {cfg.editable.then(|| view! {
            <div class="sheet-actions">
                <button on:click=move |_| mode.set(Mode::Edit)>{t().sheet_edit}</button>
                <button on:click=move |_| mode.set(Mode::History)>{t().sheet_history}</button>
            </div>
        })}
    }
}

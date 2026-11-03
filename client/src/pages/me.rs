//! The viewer's own page: how many votes they cast, where that puts them
//! among the voters, how far they are through their duels, every vote of
//! theirs in a matrix whose cells reopen their duels, and the way to start
//! over.

use crate::api;
use crate::components::icons::{Icon, icon};
use crate::i18n::{fill, say, t};
use crate::pages::matrices::UserMatrix;
use crate::state::{AppState, Panel, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn MePage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let matrix = state.user_matrix;
    let votes = move || {
        matrix.with(|m| {
            m.as_ref()
                .map_or(0, |m| m.votes.len() + m.legacy_votes.len())
        })
    };
    // Every pair among the items the viewer compares or compared.
    let pairs = move || {
        matrix.with(|m| {
            m.as_ref()
                .map_or(0, |m| m.films.len() * m.films.len().saturating_sub(1) / 2)
        })
    };
    let progress = move || 100.0 * votes() as f64 / pairs().max(1) as f64;
    let contributions = state.contributions;
    let standing = move || {
        contributions.with(|all| {
            all.iter()
                .find(|c| c.is_you)
                .map(|c| fill(t().me_rank, &[("n", &c.rank), ("m", &all.len())]))
        })
    };
    let panel = state.panel;
    let (google, name) = (state.google, state.user_name);
    let anonymous = move || google.get().is_none() && name.get().is_none() && votes() > 0;
    let google_on = !cfg.google_client_id.is_empty();
    let selected = state.selected_ids;

    view! {
        <div class="me">
            <section class="me-hero">
                <div class="me-count">
                    <b>{votes}</b>
                    <span>{move || if votes() == 1 { t().vote_one } else { t().vote_many }}</span>
                </div>
                {move || standing().map(|s| view! { <div class="me-rank">{"\u{1F3C5} "}{s}</div> })}
                <div class="tally-bar">
                    <i style:width=move || format!("{:.1}%", progress())></i>
                </div>
                <div class="me-pairs">
                    {move || fill(t().me_pairs, &[("done", &votes()), ("total", &pairs())])}
                </div>
            </section>

            {move || (google_on && anonymous()).then(|| view! {
                <button class="nudge" on:click=move |_| panel.set(Some(Panel::Account))>
                    <span class="nudge-icon">{icon(Icon::User)}</span>
                    <span class="nudge-text">
                        <b>{t().keep_votes}</b>
                        <span>{t().keep_votes_text}</span>
                    </span>
                    <span class="nudge-go">{t().sign_in}</span>
                </button>
            })}

            {cfg.require_selection.then(|| view! {
                <button class="row-btn" on:click=move |_| panel.set(Some(Panel::Selection))>
                    {icon(Icon::ListCheck)}
                    <span>{say(t().selection_title, &[])}</span>
                    <b>{move || selected.with(|ids| ids.len())}</b>
                </button>
            })}

            <section class="me-matrix">
                <h2 class="section-title">{t().matrix_yours}</h2>
                <p class="fine-print">{t().matrix_yours_hint}</p>
                <UserMatrix />
            </section>

            <button
                class="danger-link"
                on:click=move |_| {
                    let window = web_sys::window().unwrap();
                    if window.confirm_with_message(t().reset_confirm).unwrap_or(false) {
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move { api::reset_votes(&s).await });
                    }
                }
            >
                {icon(Icon::Trash)}
                <span>{t().reset_button}</span>
            </button>
        </div>
    }
}

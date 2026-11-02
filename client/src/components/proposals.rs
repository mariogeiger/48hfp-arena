//! Below the leaderboard of an editable site: the button to propose an item
//! and the proposed items awaiting review, each opening its sheet.

use crate::components::poster::Poster;
use crate::i18n::{say, t};
use crate::state::{AppState, Sheet, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn Proposals() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let has_image = SiteConfig::get().has_image;
    let state_add = state.clone();

    view! {
        <div class="proposals">
            <button class="add-item" on:click=move |_| state_add.open_sheet(Sheet::New)>
                {say(t().add_item, &[])}
            </button>
            {move || {
                let proposals = state.proposals.get();
                let state = state.clone();
                (!proposals.is_empty()).then(move || view! {
                    <h3>{say(t().proposals_heading, &[])}</h3>
                    <p class="proposals-hint">{t().proposals_hint}</p>
                    {proposals.into_iter().map(|p| {
                        let id = p.film.id;
                        let meta = [p.film.team.as_str(), p.film.city.as_str()]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" \u{00B7} ");
                        let state = state.clone();
                        view! {
                            <div class="board-item board-proposal" on:click=move |_| state.open_sheet(Sheet::Item(id))>
                                <div class="board-rank">"+"</div>
                                {has_image.then(|| view! { <Poster url=p.film.poster_url.clone() /> })}
                                <div class="board-info">
                                    <div class="board-title">{p.film.title.clone()}</div>
                                    <div class="board-meta">{meta}</div>
                                </div>
                                <span class="badge">{t().proposal_badge}</span>
                            </div>
                        }
                    }).collect_view()}
                })
            }}
        </div>
    }
}

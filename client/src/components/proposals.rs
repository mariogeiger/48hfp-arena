//! Below the ranking of an editable site: the button to propose an item
//! and the proposed items awaiting review, each opening its sheet.

use crate::components::card::meta;
use crate::components::icons::{Icon, icon};
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
                {icon(Icon::Plus)}
                <span>{say(t().add_item, &[])}</span>
            </button>
            {move || {
                let proposals = state.proposals.get();
                let state = state.clone();
                (!proposals.is_empty()).then(move || view! {
                    <h3 class="sub-title">{say(t().proposals_heading, &[])}</h3>
                    <p class="fine-print">{t().proposals_hint}</p>
                    <div class="board-rows">
                        {proposals.into_iter().map(|p| {
                            let id = p.film.id;
                            let state = state.clone();
                            view! {
                                <div class="board-row" on:click=move |_| state.open_sheet(Sheet::Item(id))>
                                    <span class="board-rank">"+"</span>
                                    {has_image.then(|| view! {
                                        <span class="board-art">
                                            <Poster url=p.film.poster_url.clone() title=p.film.title.clone() id=p.film.id />
                                        </span>
                                    })}
                                    <span class="board-info">
                                        <span class="board-title">{p.film.title.clone()}</span>
                                        <span class="board-meta">{meta(&p.film)}</span>
                                    </span>
                                    <span class="badge">{t().proposal_badge}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                })
            }}
        </div>
    }
}

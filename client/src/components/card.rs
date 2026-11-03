//! What a card shows of an item in a duel: its poster and title, or on a
//! site without images, its title and description as a text card.

use crate::components::poster::Poster;
use crate::state::SiteConfig;
use leptos::prelude::*;
use versus_shared::Film;

/// The item's team and city, or whichever it has.
pub fn meta(film: &Film) -> String {
    [film.team.as_str(), film.city.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{00B7} ")
}

#[component]
pub fn CardFace(film: Film) -> impl IntoView {
    let meta = meta(&film);
    if SiteConfig::get().has_image {
        view! {
            <div class="card-art">
                <Poster url=film.poster_url.clone() title=film.title.clone() id=film.id />
            </div>
            <div class="card-text">
                <div class="card-title">{film.title.clone()}</div>
                <div class="card-meta">{meta}</div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="card-text card-text-only">
                <div class="card-title">{film.title.clone()}</div>
                <div class="card-meta">{meta}</div>
            </div>
        }
        .into_any()
    }
}

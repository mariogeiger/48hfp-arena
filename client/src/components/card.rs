//! What a card shows of an item in a duel: its poster and title, or on a
//! site without images, its title and description as a text card. What the
//! caller lays over its face, such as a video, covers the poster, or the text.

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

/// The face of `film`, which follows it as it changes, with `children` laid
/// over the poster, or over the text when there is no poster.
#[component]
pub fn CardFace(
    #[prop(into)] film: Signal<Film>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let title = move || film.with(|f| f.title.clone());
    let meta = move || film.with(meta);
    let over = children.map(|children| children());
    if SiteConfig::get().has_image {
        let poster = move || {
            film.with(
                |f| view! { <Poster url=f.poster_url.clone() title=f.title.clone() id=f.id /> },
            )
        };
        view! {
            <div class="card-art">{poster} {over}</div>
            <div class="card-text">
                <div class="card-title">{title}</div>
                <div class="card-meta">{meta}</div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="card-text card-text-only">
                <div class="card-title">{title}</div>
                <div class="card-meta">{meta}</div>
                {over}
            </div>
        }
        .into_any()
    }
}

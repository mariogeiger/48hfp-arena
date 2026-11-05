//! An item's video, large over everything: the screen darkens, the player
//! takes all the width it can at 16:9, the versions over it, and its cross, a
//! tap beside it or the Escape key (see `lib.rs`) bring back what lay under.

use crate::components::icons::{Icon, icon};
use crate::components::video::Versions;
use crate::i18n::{or_default, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn Screening() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let screening = state.screening;
    let link_label = or_default(&SiteConfig::get().link_label, t().open_link);
    move || {
        let id = screening.get()?;
        let film = state
            .films
            .with_untracked(|films| films.iter().find(|f| f.id == id).cloned())?;
        let close = move || screening.set(None);
        Some(view! {
            <div
                class="screening"
                role="dialog"
                aria-modal="true"
                aria-label=film.title.clone()
                on:click=move |ev| {
                    if ev.target() == ev.current_target() {
                        close();
                    }
                }
            >
                <div class="screening-stage">
                    <div class="screening-head">
                        <h2>{film.title.clone()}</h2>
                        <button class="screening-close" title=t().sheet_close on:click=move |_| close()>
                            {icon(Icon::X)}
                        </button>
                    </div>
                    <Versions videos=film.videos link_label=link_label.clone() autoplay=true />
                </div>
            </div>
        })
    }
}

//! The bar on top of every page, with the site's name and the account, and
//! the tabs: in the middle of the bar on a wide screen, at the bottom of
//! the screen under the thumb on a phone (see `style.css`).

use crate::components::account::AccountButton;
use crate::components::icons::{Icon, icon};
use crate::i18n::t;
use crate::state::{AppState, Page, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let home = Page::from_str(&cfg.start_page).unwrap_or(Page::Duels);
    view! {
        <header class="topbar">
            <button class="brand" on:click=move |_| state.navigate(home)>
                <span class="brand-mark">"VS"</span>
                <span class="brand-name">{cfg.display_name.clone()}</span>
            </button>
            <AccountButton />
        </header>
    }
}

#[component]
pub fn Tabs() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let page = state.page;
    let pulse = state.live_pulse;
    // Others' votes the viewer has not looked at on the ranking yet.
    let seen = RwSignal::new(0u32);
    Effect::new(move || {
        if matches!(page.get(), Page::Board | Page::About) {
            seen.set(pulse.get());
        }
    });

    let tab = move |target: Page, glyph: Icon, label: &'static str| {
        let state = state.clone();
        let active = move || {
            let now = page.get();
            now == target || (target == Page::Board && now == Page::About)
        };
        let fresh = move || target == Page::Board && pulse.get() > seen.get() && !active();
        view! {
            <button class="tab" class:active=active on:click=move |_| state.navigate(target)>
                <span class="tab-icon">
                    {icon(glyph)}
                    {move || fresh().then(|| view! { <i class="pulse"></i> })}
                </span>
                <span class="tab-label">{label}</span>
            </button>
        }
    };

    view! {
        <nav class="tabs">
            {tab(Page::Duels, Icon::Swords, t().tab_duels)}
            {tab(Page::Board, Icon::Trophy, t().tab_board)}
            {tab(Page::Me, Icon::Ballot, t().tab_me)}
        </nav>
    }
}

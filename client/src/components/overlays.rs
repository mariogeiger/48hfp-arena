//! What covers the whole page: the welcome, with its promise to vote
//! honestly, and the notice of a suspended account.

use crate::i18n::{or_default, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;

const PROMISED_KEY: &str = "filmrank_promised";

#[component]
pub fn WelcomeOverlay() -> impl IntoView {
    let welcome_text = or_default(&SiteConfig::get().welcome_text, t().welcome_text);
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let promised = storage.get_item(PROMISED_KEY).ok().flatten().is_some();
    let show = RwSignal::new(!promised);

    move || {
        show.get().then(|| {
            let welcome_text = welcome_text.clone();
            view! {
                <div class="overlay">
                    <div class="overlay-card welcome">
                        <div class="overlay-emoji">"\u{1F91E}"</div>
                        <h2>{t().welcome_heading}</h2>
                        <p>{welcome_text}</p>
                        <div class="overlay-actions">
                            <button
                                class="cta"
                                on:click=move |_| {
                                    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
                                    let _ = storage.set_item(PROMISED_KEY, "1");
                                    show.set(false);
                                }
                            >
                                {t().welcome_promise}
                            </button>
                            <a href="https://google.com" class="ghost-btn">{t().welcome_refuse}</a>
                        </div>
                    </div>
                </div>
            }
        })
    }
}

#[component]
pub fn BannedOverlay() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    move || {
        state.banned.get().then(|| {
            view! {
                <div class="overlay">
                    <div class="overlay-card">
                        <div class="overlay-emoji">"\u{1F6AB}"</div>
                        <h2>{t().banned_heading}</h2>
                        <p>{t().banned_text}</p>
                    </div>
                </div>
            }
        })
    }
}

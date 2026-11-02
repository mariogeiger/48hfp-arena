use crate::i18n::{or_default, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn WelcomeOverlay() -> impl IntoView {
    let welcome_text = or_default(&SiteConfig::get().welcome_text, t().welcome_text);
    let promised = web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap()
        .get_item("filmrank_promised")
        .ok()
        .flatten()
        .is_some();

    let show = RwSignal::new(!promised);

    view! {
        <div
            class="welcome-overlay"
            class:hidden=move || !show.get()
            id="welcome-overlay"
        >
            <div class="welcome-bubble">
                <h2>{t().welcome_heading}</h2>
                <p>
                    {welcome_text}
                </p>
                <div class="welcome-buttons">
                    <button
                        class="welcome-btn-promise"
                        on:click=move |_| {
                            let storage = web_sys::window()
                                .unwrap()
                                .local_storage()
                                .unwrap()
                                .unwrap();
                            let _ = storage.set_item("filmrank_promised", "1");
                            show.set(false);
                        }
                    >
                        {t().welcome_promise}
                    </button>
                    <a href="https://google.com" class="welcome-btn-nope">
                        {t().welcome_refuse}
                    </a>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn BannedOverlay() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    view! {
        <div
            class="welcome-overlay"
            class:hidden=move || !state.banned.get()
            id="banned-overlay"
        >
            <div class="welcome-bubble">
                <h2>{t().banned_heading}</h2>
                <p>{t().banned_text}</p>
            </div>
        </div>
    }
}

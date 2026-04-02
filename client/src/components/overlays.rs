use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn WelcomeOverlay() -> impl IntoView {
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
                <h2>"Welcome!"</h2>
                <p>
                    "Please only vote on pairs of films you have actually watched. Your honest comparisons make the ranking meaningful for everyone."
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
                        "I promise"
                    </button>
                    <a href="https://google.com" class="welcome-btn-nope">
                        "I can't promise that"
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
                <h2>"Account Suspended"</h2>
                <p>
                    "Your account has been suspended for violating the voting rules. If you believe this is a mistake, please contact the site administrator."
                </p>
            </div>
        </div>
    }
}

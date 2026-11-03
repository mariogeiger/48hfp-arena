//! The identity's two faces in the interface: the section of the More page
//! that signs it in with Google, names it and shares the link continuing it,
//! and the question a `#continue/…` link asks before this device takes up
//! that identity.

use crate::i18n::{fill, t};
use crate::identity;
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn SessionSection() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let draft = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);

    // The field shows the name as the server knows it.
    Effect::new({
        let state = state.clone();
        move || draft.set(state.user_name.get().unwrap_or_default())
    });

    let state_save = state.clone();
    let save = move || {
        let state = state_save.clone();
        let name = draft.get_untracked();
        wasm_bindgen_futures::spawn_local(async move {
            match identity::save_name(&state, &name).await {
                Ok(()) => {
                    error.set(None);
                    state.add_toast(t().saved.into());
                }
                Err(code) => error.set(Some(t().error(&code))),
            }
        });
    };
    let save_key = save.clone();
    let state_unsaved = state.clone();
    let unsaved = move || draft.get().trim() != state_unsaved.user_name.get().unwrap_or_default();

    let shared = move || {
        let state = state.clone();
        state.user_name.get().map(|_| {
            let qr = identity::link_qr_svg(&state);
            view! {
                <button
                    class="session-share"
                    on:click=move |_| {
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            identity::share_link(&s).await;
                        });
                    }
                >
                    {t().session_share}
                </button>
                <p class="session-qr-hint">{t().session_qr}</p>
                <div class="session-qr" inner_html=qr></div>
            }
        })
    };

    let google = (!SiteConfig::get().google_client_id.is_empty()).then(GoogleSignIn);

    view! {
        <div class="stats-section session-section">
            <h3>{t().session_heading}</h3>
            {google}
            <p class="matrix-hint">{t().session_hint}</p>
            <div class="session-name">
                <input
                    type="text"
                    maxlength="40"
                    aria-label=t().session_name
                    placeholder=t().session_name_placeholder
                    prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            save_key();
                        }
                    }
                />
                <button disabled=move || !unsaved() on:click=move |_| save()>
                    {t().form_save}
                </button>
            </div>
            {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
            {move || {
                let state = use_context::<AppState>().unwrap();
                state.user_name.get().is_none().then(|| view! {
                    <p class="matrix-hint">{t().session_name_needed}</p>
                })
            }}
            {shared}
        </div>
    }
}

/// Google's button, or the account signed in and the way out.
#[component]
fn GoogleSignIn() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let button = NodeRef::<leptos::html::Div>::new();

    Effect::new({
        let state = state.clone();
        move || {
            if let Some(div) = button.get() {
                crate::google::render_button(&state, div.unchecked_into());
            }
        }
    });

    view! {
        <div class="session-google">
            {move || match state.google_email.get() {
                Some(email) => view! {
                    <p class="matrix-hint">{fill(t().google_signed_in, &[("email", &email)])}</p>
                    <button
                        class="session-sign-out"
                        on:click=move |_| {
                            let window = web_sys::window().unwrap();
                            if window.confirm_with_message(t().google_sign_out_confirm).unwrap_or(false) {
                                identity::sign_out();
                            }
                        }
                    >
                        {t().google_sign_out}
                    </button>
                }
                .into_any(),
                None => view! {
                    <p class="matrix-hint">{t().google_hint}</p>
                    <div class="google-button" node_ref=button></div>
                    <p class="matrix-hint">{t().session_or_link}</p>
                }
                .into_any(),
            }}
        </div>
    }
}

#[component]
pub fn HandoffOverlay() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let error = RwSignal::new(None::<&'static str>);
    let busy = RwSignal::new(false);

    move || {
        let handoff = state.handoff.get()?;
        let question = match &handoff.name {
            Some(name) => fill(t().handoff_named, &[("name", name), ("n", &handoff.votes)]),
            None => fill(t().handoff_unnamed, &[("n", &handoff.votes)]),
        };
        let fold =
            (handoff.own_votes > 0).then(|| fill(t().handoff_fold, &[("n", &handoff.own_votes)]));
        let state_accept = state.clone();
        let state_refuse = state.clone();
        Some(view! {
            <div class="welcome-overlay">
                <div class="welcome-bubble">
                    <h2>{t().handoff_heading}</h2>
                    <p>{question}</p>
                    {fold.map(|f| view! { <p>{f}</p> })}
                    <p class="handoff-warning">{t().handoff_warning}</p>
                    {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
                    <div class="welcome-buttons">
                        <button
                            class="welcome-btn-promise"
                            disabled=move || busy.get()
                            on:click=move |_| {
                                let state = state_accept.clone();
                                let handoff = handoff.clone();
                                busy.set(true);
                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Err(code) = identity::accept_handoff(&state, &handoff).await {
                                        error.set(Some(t().error(&code)));
                                        busy.set(false);
                                    }
                                });
                            }
                        >
                            {t().handoff_accept}
                        </button>
                        <button
                            class="welcome-btn-nope"
                            on:click=move |_| {
                                error.set(None);
                                state_refuse.handoff.set(None);
                            }
                        >
                            {t().handoff_refuse}
                        </button>
                    </div>
                </div>
            </div>
        })
    }
}

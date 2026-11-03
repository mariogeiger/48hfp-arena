//! The account, top right. Signed out, a button says so, and opens a sheet
//! to sign in with Google or to continue on another device through a link or
//! its QR code. Signed in with Google, which does both and better, it shows
//! one's picture or initial, and its sheet who one is and how to sign out.
//! A `#continue/…` link asks here whether the viewer takes up the identity
//! it carries.

use crate::components::frame::SheetFrame;
use crate::components::icons::{Icon, icon};
use crate::i18n::{fill, t};
use crate::identity;
use crate::state::{AppState, GoogleProfile, Panel, SiteConfig};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// The first letter of `name`, in upper case.
fn initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

/// A hue of its own for each name.
fn hue(name: &str) -> u32 {
    name.bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32))
        % 360
}

/// What the viewer is called: their Google name, else the name they gave,
/// else their Google e-mail.
fn called(google: Option<&GoogleProfile>, name: Option<&str>) -> Option<String> {
    let given = |n: &str| Some(n.trim().to_string()).filter(|n| !n.is_empty());
    google
        .and_then(|g| given(&g.name))
        .or_else(|| name.and_then(given))
        .or_else(|| google.and_then(|g| given(&g.email)))
}

/// A face: the Google picture, else the initial of `name`, else a
/// silhouette.
fn avatar(google: Option<&GoogleProfile>, name: Option<&str>) -> AnyView {
    let picture = google.map(|g| g.picture.as_str()).filter(|p| !p.is_empty());
    match (picture, name) {
        (Some(picture), _) => view! {
            <img class="avatar" src=picture.to_string() alt="" referrerpolicy="no-referrer" />
        }
        .into_any(),
        (None, Some(name)) => view! {
            <span class="avatar avatar-initial" style=format!("--hue:{}", hue(name))>
                {initial(name)}
            </span>
        }
        .into_any(),
        (None, None) => {
            view! { <span class="avatar avatar-blank">{icon(Icon::User)}</span> }.into_any()
        }
    }
}

#[component]
pub fn AccountButton() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let (panel, google, name) = (state.panel, state.google, state.user_name);
    let known = move || google.with(Option::is_some) || name.with(Option::is_some);
    // The button stays while what it shows changes, so that no tap is lost.
    view! {
        <button
            class="account-btn"
            class:known=known
            on:click=move |_| panel.set(Some(Panel::Account))
        >
            {move || {
                let google = google.get();
                avatar(google.as_ref(), called(google.as_ref(), name.get().as_deref()).as_deref())
            }}
            {move || (!known()).then(|| view! { <span class="account-label">{t().sign_in}</span> })}
        </button>
    }
}

#[component]
pub fn AccountSheet() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let panel = state.panel;
    move || {
        (panel.get() == Some(Panel::Account)).then(|| {
            view! {
                <SheetFrame on_close=Callback::new(move |_| panel.set(None)) class="account-sheet">
                    <AccountBody />
                </SheetFrame>
            }
        })
    }
}

#[component]
fn AccountBody() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let google_on = !SiteConfig::get().google_client_id.is_empty();
    let (google, name) = (state.google, state.user_name);

    let head = move || {
        let google = google.get();
        let who = called(google.as_ref(), name.get().as_deref());
        let (title, subtitle) = match &google {
            Some(g) => {
                let title = who.clone().unwrap_or_default();
                let subtitle = (title != g.email).then(|| g.email.clone());
                (title, subtitle)
            }
            None => (
                who.clone().unwrap_or_else(|| t().keep_votes.to_string()),
                Some(t().keep_votes_text.to_string()),
            ),
        };
        view! {
            <div class="account-head">
                {avatar(google.as_ref(), who.as_deref())}
                <h2>{title}</h2>
                {subtitle.map(|s| view! { <p>{s}</p> })}
            </div>
        }
    };
    // Signed in with Google, another device signs in the same way: no link
    // is needed, and none is offered.
    let ways = move || match google.get() {
        None => view! {
            {google_on.then(|| view! {
                <GoogleButton />
                <div class="or"><span>{t().or}</span></div>
            })}
            <details class="handoff">
                <summary>
                    {icon(Icon::Smartphone)}
                    <span>{t().other_device}</span>
                    {icon(Icon::ChevronDown)}
                </summary>
                <LinkHandoff />
            </details>
        }
        .into_any(),
        Some(_) => view! {
            <button
                class="ghost-btn"
                on:click=move |_| {
                    let window = web_sys::window().unwrap();
                    if window.confirm_with_message(t().google_sign_out_confirm).unwrap_or(false) {
                        identity::sign_out();
                    }
                }
            >
                {icon(Icon::LogOut)}
                <span>{t().google_sign_out}</span>
            </button>
        }
        .into_any(),
    };

    view! {
        {head}
        {ways}
    }
}

/// Google's button, which Google draws and runs (see `google.rs`).
#[component]
fn GoogleButton() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let button = NodeRef::<leptos::html::Div>::new();
    Effect::new(move || {
        if let Some(div) = button.get() {
            crate::google::render_button(&state, div.unchecked_into());
        }
    });
    view! { <div class="google-button" node_ref=button></div> }
}

/// The link that continues this identity elsewhere, and its QR code. It
/// asks whoever opens it whether they are the one named here, so it needs a
/// name first.
#[component]
fn LinkHandoff() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let draft = RwSignal::new(state.user_name.get_untracked().unwrap_or_default());
    let error = RwSignal::new(None::<&'static str>);
    let name = state.user_name;

    let save = {
        let state = state.clone();
        move || {
            let state = state.clone();
            let typed = draft.get_untracked();
            wasm_bindgen_futures::spawn_local(async move {
                match identity::save_name(&state, &typed).await {
                    Ok(()) => error.set(None),
                    Err(code) => error.set(Some(t().error(&code))),
                }
            });
        }
    };
    let save_key = save.clone();
    let unsaved = move || draft.get().trim() != name.get().unwrap_or_default();

    let shared = move || {
        let state = state.clone();
        name.get().map(|_| {
            let qr = identity::link_qr_svg(&state);
            view! {
                <div class="session-qr" inner_html=qr></div>
                <button
                    class="cta"
                    on:click=move |_| {
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move { identity::share_link(&s).await });
                    }
                >
                    {icon(Icon::Share)}
                    <span>{t().session_share}</span>
                </button>
                <p class="fine-print">{t().session_hint}</p>
            }
        })
    };

    view! {
        <div class="session">
            {move || name.get().is_none().then(|| view! { <p class="sheet-hint">{t().session_name_needed}</p> })}
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
                <button class="mini-btn" disabled=move || !unsaved() on:click=move |_| save()>
                    {t().form_save}
                </button>
            </div>
            {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
            {shared}
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
            <div class="overlay">
                <div class="overlay-card">
                    {avatar(None, handoff.name.as_deref())}
                    <h2>{t().handoff_heading}</h2>
                    <p>{question}</p>
                    {fold.map(|f| view! { <p>{f}</p> })}
                    <p class="fine-print">{t().handoff_warning}</p>
                    {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
                    <div class="overlay-actions">
                        <button
                            class="cta"
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
                            class="ghost-btn"
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

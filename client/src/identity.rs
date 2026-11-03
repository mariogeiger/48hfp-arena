//! This browser's identity on the server: its private name, the link that
//! continues it on another device, taking up the identity such a link or a
//! Google account offers, which folds this device's votes into it, and
//! leaving it.

use crate::api::{get, post};
use crate::i18n::t;
use crate::state::{AppState, GoogleProfile, Handoff, SiteConfig, USER_ID_KEY};
use leptos::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(serde::Deserialize)]
struct Identity {
    name: Option<String>,
    votes: usize,
    #[serde(default)]
    google: Option<GoogleProfile>,
}

async fn identity(user_id: &str) -> Option<Identity> {
    get(&format!("/api/identity?user_id={user_id}")).await
}

pub async fn load_identity(state: &AppState) {
    let identity = identity(&state.user_id).await;
    let (name, google) = identity.map_or((None, None), |i| (i.name, i.google));
    state.user_name.set(name);
    state.google.set(google);
}

pub async fn save_name(state: &AppState, name: &str) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "name": name });
    post("/api/identity/name", body).await?;
    let name = name.trim();
    state
        .user_name
        .set((!name.is_empty()).then(|| name.to_string()));
    Ok(())
}

/// The address that continues this identity, on the page it is open on.
pub fn continue_link(state: &AppState) -> String {
    let location = web_sys::window().unwrap().location();
    format!(
        "{}{}#continue/{}",
        location.origin().unwrap_or_default(),
        location.pathname().unwrap_or_default(),
        state.user_id
    )
}

/// Hand the link to the system's share sheet where there is one, else to the
/// clipboard.
pub async fn share_link(state: &AppState) {
    let link = continue_link(state);
    let navigator = web_sys::window().unwrap().navigator();
    let can_share = js_sys::Reflect::has(&navigator, &"share".into()).unwrap_or(false);
    if can_share {
        let data = web_sys::ShareData::new();
        data.set_url(&link);
        // Dismissing the sheet rejects the promise: nothing to say then.
        let _ = JsFuture::from(navigator.share_with_data(&data)).await;
    } else if JsFuture::from(navigator.clipboard().write_text(&link))
        .await
        .is_ok()
    {
        state.add_toast(t().session_copied.into());
    }
}

/// The link as a QR code, in SVG.
pub fn link_qr_svg(state: &AppState) -> String {
    use qrcode::render::svg;
    match qrcode::QrCode::new(continue_link(state).as_bytes()) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(180, 180)
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}

/// Ask whether the viewer takes up the identity `other` that a link names.
pub async fn offer_handoff(state: &AppState, other: &str) {
    if !is_uuid(other) {
        state.add_toast(t().handoff_invalid.into());
        return;
    }
    if other == state.user_id {
        state.add_toast(t().handoff_already.into());
        return;
    }
    let Some(theirs) = identity(other).await else {
        state.add_toast(t().handoff_unknown.into());
        return;
    };
    let own_votes = identity(&state.user_id).await.map_or(0, |i| i.votes);
    state.handoff.set(Some(Handoff {
        user_id: other.to_string(),
        name: theirs.name,
        votes: theirs.votes,
        own_votes,
    }));
}

/// Fold this identity into the one offered and become it.
pub async fn accept_handoff(state: &AppState, handoff: &Handoff) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "into": handoff.user_id });
    let answer = post("/api/identity/merge", body).await?;
    become_identity(state, &handoff.user_id, &answer["selection"])
}

/// Sign in with the Google account an ID token names: the server answers
/// with the identity to continue, this one or the one the account had.
pub async fn sign_in_with_google(state: &AppState, credential: &str) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "credential": credential });
    let answer = post("/api/identity/google", body).await?;
    let user_id = answer["user_id"].as_str().unwrap_or_default();
    if user_id == state.user_id {
        load_identity(state).await;
        Ok(())
    } else {
        become_identity(state, user_id, &answer["selection"])
    }
}

/// Leave this identity: the page reloads as a new, empty one. The votes
/// stay with the identity, which signing in finds again.
pub fn sign_out() {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    let _ = storage.remove_item(USER_ID_KEY);
    let _ = window.location().reload();
}

/// Become the identity `user_id`, with its `selection`: the page reloads as it.
fn become_identity(
    state: &AppState,
    user_id: &str,
    selection: &serde_json::Value,
) -> Result<(), String> {
    let selection: Vec<usize> = serde_json::from_value(selection.clone()).unwrap_or_default();
    if SiteConfig::get().require_selection && !selection.is_empty() {
        state.selected_ids.set(selection.into_iter().collect());
        state.store_selection();
    }
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    storage
        .set_item(USER_ID_KEY, user_id)
        .map_err(|_| "storage".to_string())?;
    let _ = window.location().reload();
    Ok(())
}

/// Whether `s` is a UUID in its usual form, as the browser makes them.
fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

#[cfg(test)]
mod tests {
    use super::is_uuid;

    #[test]
    fn uuids_are_recognized() {
        assert!(is_uuid("0f8fad5b-d9cb-469f-a165-70867728950e"));
        assert!(!is_uuid("0f8fad5b-d9cb-469f-a165-70867728950"));
        assert!(!is_uuid("0f8fad5bxd9cb-469f-a165-70867728950e"));
        assert!(!is_uuid("../api/x"));
    }
}

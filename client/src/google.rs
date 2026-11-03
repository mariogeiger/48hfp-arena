//! The "Sign in with Google" button, which Google Identity Services draws
//! and runs: its script is loaded on first use, and the ID token it hands
//! back goes to the server (see `identity::sign_in_with_google`).

use crate::i18n::{lang, t};
use crate::state::{AppState, SiteConfig};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

const SCRIPT: &str = "https://accounts.google.com/gsi/client";

thread_local! {
    /// Whether Google's script was told the client and the callback, which
    /// it takes once per page.
    static INITIALIZED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["google", "accounts", "id"], js_name = initialize)]
    fn gis_initialize(config: &JsValue);
    #[wasm_bindgen(js_namespace = ["google", "accounts", "id"], js_name = renderButton)]
    fn gis_render_button(parent: &web_sys::HtmlElement, options: &JsValue);
}

/// Draw the button in `parent`, loading Google's script first if need be.
pub fn render_button(state: &AppState, parent: web_sys::HtmlElement) {
    let window = web_sys::window().unwrap();
    if js_sys::Reflect::has(&window, &"google".into()).unwrap_or(false) {
        draw(state, &parent);
        return;
    }
    let doc = window.document().unwrap();
    let state = state.clone();
    let onload = Closure::once_into_js(move || draw(&state, &parent));
    // Another button may be waiting for the script already.
    if let Ok(Some(script)) = doc.query_selector(&format!("script[src='{SCRIPT}']")) {
        let _ = script.add_event_listener_with_callback("load", onload.unchecked_ref());
        return;
    }
    let script: web_sys::HtmlScriptElement =
        doc.create_element("script").unwrap().dyn_into().unwrap();
    script.set_src(SCRIPT);
    script.set_async(true);
    let _ = script.add_event_listener_with_callback("load", onload.unchecked_ref());
    doc.head().unwrap().append_child(&script).unwrap();
}

fn draw(state: &AppState, parent: &web_sys::HtmlElement) {
    if !INITIALIZED.replace(true) {
        initialize(state);
    }
    let options = object(&[
        ("type", "standard".into()),
        ("theme", "outline".into()),
        ("size", "large".into()),
        ("shape", "pill".into()),
        ("text", "signin_with".into()),
        // Google knows "fr", not "fr-CH".
        (
            "locale",
            lang().tag().split('-').next().unwrap_or("en").into(),
        ),
    ]);
    gis_render_button(parent, &options);
}

fn initialize(state: &AppState) {
    let state = state.clone();
    let callback = Closure::<dyn Fn(JsValue)>::new(move |response: JsValue| {
        let credential = js_sys::Reflect::get(&response, &"credential".into())
            .ok()
            .and_then(|c| c.as_string())
            .unwrap_or_default();
        let state = state.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if crate::identity::sign_in_with_google(&state, &credential)
                .await
                .is_err()
            {
                state.add_toast(t().google_failed.into());
            }
        });
    });
    let config = object(&[
        ("client_id", SiteConfig::get().google_client_id.into()),
        ("callback", callback.into_js_value()),
    ]);
    gis_initialize(&config);
}

fn object(fields: &[(&str, JsValue)]) -> JsValue {
    let object = js_sys::Object::new();
    for (key, value) in fields {
        js_sys::Reflect::set(&object, &(*key).into(), value).unwrap();
    }
    object.into()
}

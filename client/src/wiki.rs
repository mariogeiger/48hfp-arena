//! The wiki's calls to the server, and the poster upload: the browser scales
//! the picked image down to a JPEG before sending it.

use crate::api::{get, post, send};
use crate::state::AppState;
use leptos::prelude::*;
use versus_shared::{FilmFields, HistoryEntry, Proposal};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// The largest poster sent, in pixels; larger images are scaled down to fit.
const POSTER_WIDTH: f64 = 600.0;
const POSTER_HEIGHT: f64 = 900.0;

/// The film's history as the server tells it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct History {
    pub revisions: Vec<HistoryEntry>,
    pub original: Option<FilmFields>,
}

pub async fn load_proposals(state: &AppState) {
    if !crate::state::SiteConfig::get().editable {
        return;
    }
    if let Some(proposals) =
        get::<Vec<Proposal>>(&format!("/api/proposals?user_id={}", state.user_id)).await
    {
        state.proposals.set(proposals);
    }
}

pub async fn history(state: &AppState, film: usize) -> Option<History> {
    get(&format!(
        "/api/films/{film}/history?user_id={}",
        state.user_id
    ))
    .await
}

/// After a change: the films, the board and the proposals as they now are.
pub async fn reload(state: &AppState) {
    crate::api::reload_films(state).await;
    crate::api::load_board(state).await;
    load_proposals(state).await;
}

pub async fn edit(state: &AppState, film: usize, fields: FilmFields) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "fields": fields });
    post(&format!("/api/films/{film}"), body).await?;
    reload(state).await;
    Ok(())
}

pub async fn revert(state: &AppState, film: usize, rev: u64) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "rev": rev });
    post(&format!("/api/films/{film}/revert"), body).await?;
    reload(state).await;
    Ok(())
}

/// Propose a new item; returns its id.
pub async fn propose(state: &AppState, fields: FilmFields) -> Result<usize, String> {
    let body = serde_json::json!({ "user_id": state.user_id, "fields": fields });
    let json = post("/api/films", body).await?;
    reload(state).await;
    json["film"]
        .as_u64()
        .map(|id| id as usize)
        .ok_or_else(|| "server".to_string())
}

pub async fn review(state: &AppState, film: usize, approve: bool) -> Result<(), String> {
    let body = serde_json::json!({ "user_id": state.user_id, "approve": approve });
    post(&format!("/api/films/{film}/review"), body).await?;
    reload(state).await;
    Ok(())
}

/// Scale `file` down to a JPEG and upload it; returns the poster's URL.
pub async fn upload_poster(state: &AppState, file: web_sys::File) -> Result<String, String> {
    let jpeg = downscaled_jpeg(&file)
        .await
        .map_err(|_| "not_jpeg".to_string())?;
    let json = send(
        &format!("/api/uploads?user_id={}", state.user_id),
        &jpeg,
        "image/jpeg",
    )
    .await?;
    json["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "server".to_string())
}

async fn downscaled_jpeg(file: &web_sys::File) -> Result<web_sys::Blob, JsValue> {
    let url = web_sys::Url::create_object_url_with_blob(file)?;
    let img = web_sys::HtmlImageElement::new()?;
    img.set_src(&url);
    let decoded = JsFuture::from(img.decode()).await;
    web_sys::Url::revoke_object_url(&url)?;
    decoded?;

    let (w, h) = (img.natural_width() as f64, img.natural_height() as f64);
    if w == 0.0 || h == 0.0 {
        return Err(JsValue::NULL);
    }
    let scale = (POSTER_WIDTH / w).min(POSTER_HEIGHT / h).min(1.0);
    let (cw, ch) = ((w * scale).round(), (h * scale).round());
    let canvas: web_sys::HtmlCanvasElement = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")?
        .unchecked_into();
    canvas.set_width(cw as u32);
    canvas.set_height(ch as u32);
    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or(JsValue::NULL)?
        .unchecked_into();
    ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, cw, ch)?;

    let blob = js_sys::Promise::new(&mut |resolve, reject| {
        if canvas
            .to_blob_with_type_and_encoder_options(&resolve, "image/jpeg", &JsValue::from(0.85))
            .is_err()
        {
            let _ = reject.call0(&JsValue::NULL);
        }
    });
    let blob = JsFuture::from(blob).await?;
    blob.dyn_into::<web_sys::Blob>()
}

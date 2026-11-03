//! The calls to the server that fill the state: the boot, the next pair, the
//! votes and their undoing, each page's data, and the live stream of
//! everyone's votes and changes to the catalog.

use crate::state::*;
use leptos::prelude::*;
use std::collections::HashMap;
use versus_shared::{Outcome, canonical_pair, pair_list};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub async fn fetch_json(
    url: &str,
    opts: &web_sys::RequestInit,
) -> Result<(u16, serde_json::Value), String> {
    let window = web_sys::window().unwrap();
    let resp: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(url, opts))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;
    let status = resp.status();
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?
        .as_string()
        .unwrap_or_default();
    let json = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok((status, json))
}

pub fn check_banned(status: u16) {
    if status == 403
        && let Some(state) = leptos::prelude::use_context::<AppState>()
    {
        state.banned.set(true);
    }
}

async fn api_get(url: &str) -> Result<serde_json::Value, String> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let (status, json) = fetch_json(url, &opts).await?;
    check_banned(status);
    Ok(json)
}

async fn api_post(url: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    let headers = web_sys::Headers::new().unwrap();
    headers.set("Content-Type", "application/json").unwrap();
    opts.set_headers(&headers);
    opts.set_body(&JsValue::from_str(&serde_json::to_string(body).unwrap()));
    let (status, json) = fetch_json(url, &opts).await?;
    check_banned(status);
    Ok(json)
}

/// Send `body` to `url`; a refusal comes back as its error code.
pub async fn send(
    url: &str,
    body: &JsValue,
    content_type: &str,
) -> Result<serde_json::Value, String> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    let headers = web_sys::Headers::new().unwrap();
    headers.set("Content-Type", content_type).unwrap();
    opts.set_headers(&headers);
    opts.set_body(body);
    let (status, json) = fetch_json(url, &opts)
        .await
        .map_err(|_| "server".to_string())?;
    check_banned(status);
    if status == 200 {
        Ok(json)
    } else {
        Err(json["error"].as_str().unwrap_or("server").to_string())
    }
}

pub async fn post(url: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    send(
        url,
        &JsValue::from_str(&body.to_string()),
        "application/json",
    )
    .await
}

pub async fn get<T: serde::de::DeserializeOwned>(url: &str) -> Option<T> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let (_, json) = fetch_json(url, &opts).await.ok()?;
    serde_json::from_value(json).ok()
}

/// Fetch the per-site config before mount. Does not go through `api_get`
/// (which touches Leptos context) since it runs before any reactive owner
/// exists. Falls back to defaults on any error.
pub async fn fetch_config() -> SiteConfig {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    match fetch_json("/api/config", &opts).await {
        Ok((_, json)) => serde_json::from_value(json).unwrap_or_default(),
        Err(_) => SiteConfig::default(),
    }
}

pub async fn boot(state: &AppState) {
    let (films_res, board_res) = futures::join!(api_get("/api/films"), api_get("/api/leaderboard"));

    if let Ok(films_json) = films_res
        && let Ok(mut films) = serde_json::from_value::<Vec<versus_shared::Film>>(films_json)
    {
        if let Ok(board_json) = &board_res
            && let Ok(board) = serde_json::from_value::<Vec<LeaderboardEntry>>(board_json.clone())
        {
            let rank_by_id: HashMap<usize, usize> = board
                .iter()
                .enumerate()
                .map(|(i, b)| (b.film_id, i))
                .collect();
            films.sort_by_key(|f| rank_by_id.get(&f.id).copied().unwrap_or(usize::MAX));
            state.board.set(board);
        }
        state.films.set(films);
        state.restore_selection();
    }

    let cfg = SiteConfig::get();
    // The server draws pairs from the selection it was last sent.
    if cfg.require_selection && state.selected_ids.get_untracked().len() >= 2 {
        save_selection(state).await;
    }

    let hash = web_sys::window()
        .unwrap()
        .location()
        .hash()
        .unwrap_or_default();
    let hash = hash.trim_start_matches('#').to_string();

    crate::wiki::load_proposals(state).await;
    if let Some(id) = hash.strip_prefix("film/").and_then(|id| id.parse().ok()) {
        state.navigate(Page::from_str(&cfg.start_page).unwrap_or(Page::Duels));
        state.open_sheet(Sheet::Item(id));
    } else {
        let page = Page::from_str(&hash)
            .or_else(|| Page::from_str(&cfg.start_page))
            .unwrap_or(Page::Duels);
        state.navigate(page);
        // Someone who has picked nothing yet starts by picking.
        let unpicked = cfg.require_selection && state.selected_ids.get_untracked().len() < 2;
        if hash == "select" || (page == Page::Duels && unpicked) {
            state.panel.set(Some(Panel::Selection));
        }
    }

    crate::identity::load_identity(state).await;
    if let Some(other) = hash.strip_prefix("continue/") {
        crate::identity::offer_handoff(state, other).await;
    }
}

pub async fn save_selection(state: &AppState) {
    state.store_selection();
    let ids: Vec<usize> = state.selected_ids.get_untracked().into_iter().collect();

    let _ = api_post(
        "/api/selection",
        &serde_json::json!({
            "user_id": state.user_id,
            "film_ids": ids,
        }),
    )
    .await;
}

pub async fn load_pair(state: &AppState) {
    let focus = state.focus_film_id.get_untracked();
    let mut url = format!("/api/pair?user_id={}", state.user_id);
    if let Some(fid) = focus {
        url.push_str(&format!("&focus_film={}", fid));
    }
    let avoid = state
        .passed
        .with_value(|passed| pair_list(passed.iter().map(|p| p.pair)));
    if !avoid.is_empty() {
        url.push_str(&format!("&avoid={avoid}"));
    }

    let Ok(data) = api_get(&url).await else {
        return;
    };
    let votes = data.get("votes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    if data.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
        let selected = state.selected_ids.get_untracked();
        let reason = if SiteConfig::get().require_selection && selected.len() < 2 {
            PairDoneReason::NotEnough
        } else if state.focus_film_id.get_untracked().is_some() {
            PairDoneReason::FocusDone
        } else {
            PairDoneReason::AllDone
        };
        state.pair.set(None);
        state.pair_status.set(Some(reason));
        state.vote_count.set(votes);
        state.pairs_left.set(Some(0));
    } else if let (Some(a), Some(b)) = (data.get("a"), data.get("b"))
        && let (Ok(a), Ok(b)) = (
            serde_json::from_value::<versus_shared::Film>(a.clone()),
            serde_json::from_value::<versus_shared::Film>(b.clone()),
        )
    {
        state.pair.set(Some(FilmPair { a, b }));
        state.pair_status.set(None);
        state.vote_count.set(votes);
        let left = data.get("remaining").and_then(|v| v.as_u64());
        state.pairs_left.set(left.map(|n| n as usize));
    }
}

/// The body that votes `outcome` on `a` and `b`, or takes the vote back.
fn vote_body(state: &AppState, a: usize, b: usize, outcome: Outcome) -> serde_json::Value {
    let (winner, loser) = match outcome {
        Outcome::Won(w) if w == b => (b, a),
        _ => (a, b),
    };
    serde_json::json!({
        "user_id": state.user_id,
        "winner_id": winner,
        "loser_id": loser,
        "tie": outcome == Outcome::Tie,
    })
}

/// The duels a pair passed on waits before it comes back.
const PASS_VOTES: usize = 10;

/// Pass on the pair shown, which waits `PASS_VOTES` votes on other duels
/// before it comes back, then draw the next.
pub async fn pass_pair(state: &AppState) {
    if let Some(shown) = state.pair.get_untracked() {
        let pair = canonical_pair(shown.a.id, shown.b.id);
        state.passed.update_value(|passed| {
            passed.retain(|p| p.pair != pair);
            passed.push(PassedPair {
                pair,
                votes_left: PASS_VOTES,
            });
        });
    }
    load_pair(state).await;
}

/// Vote `outcome` on the pair shown, `a` against `b`, then draw the next.
/// Each pair passed on waits one vote fewer.
pub async fn cast_vote(state: &AppState, a: usize, b: usize, outcome: Outcome) {
    let _ = api_post("/api/vote", &vote_body(state, a, b, outcome)).await;
    let pair = canonical_pair(a, b);
    state.passed.update_value(|passed| {
        passed.retain_mut(|p| {
            p.votes_left = p.votes_left.saturating_sub(1);
            p.votes_left > 0 && p.pair != pair
        })
    });
    state
        .vote_history
        .update(|h| h.push(VoteRecord { a, b, outcome }));
    load_pair(state).await;
}

pub async fn undo_vote(state: &AppState) {
    let history = state.vote_history.get_untracked();
    let Some(last) = history.last().cloned() else {
        return;
    };

    let _ = api_post(
        "/api/unvote",
        &vote_body(state, last.a, last.b, last.outcome),
    )
    .await;

    // The pair comes back as it was shown.
    let films = state.films.get_untracked();
    let a = films.iter().find(|f| f.id == last.a).cloned();
    let b = films.iter().find(|f| f.id == last.b).cloned();

    state.vote_history.update(|h| {
        h.pop();
    });
    state.vote_count.update(|c| *c = c.saturating_sub(1));
    state.pairs_left.update(|left| *left = left.map(|n| n + 1));
    if let (Some(a), Some(b)) = (a, b) {
        state.pair.set(Some(FilmPair { a, b }));
        state.pair_status.set(None);
    }
}

pub async fn deselect_and_skip(state: &AppState, film_id: usize) {
    state.selected_ids.update(|ids| {
        ids.remove(&film_id);
    });
    save_selection(state).await;
    load_pair(state).await;
}

/// Fetch the films again, keeping them in leaderboard order.
pub async fn reload_films(state: &AppState) {
    if let Ok(json) = api_get("/api/films").await
        && let Ok(mut films) = serde_json::from_value::<Vec<versus_shared::Film>>(json)
    {
        let board = state.board.get_untracked();
        let rank_by_id: HashMap<usize, usize> = board
            .iter()
            .enumerate()
            .map(|(i, b)| (b.film_id, i))
            .collect();
        films.sort_by_key(|f| (rank_by_id.get(&f.id).copied().unwrap_or(usize::MAX), f.id));
        state.films.set(films);
        state.restore_selection();
    }
}

pub async fn load_board(state: &AppState) {
    if let Ok(data) = api_get("/api/leaderboard").await
        && let Ok(board) = serde_json::from_value::<Vec<LeaderboardEntry>>(data)
    {
        state.board.set(board);
    }
}

/// What the ranking shows behind the scenes: the stats, the voters'
/// contributions and everyone's duels.
pub async fn load_backstage(state: &AppState) {
    let ts = js_sys::Date::now() as u64;
    let contrib_url = format!("/api/user-contributions?user_id={}", state.user_id);
    let global_url = format!("/api/global-matrix?_={ts}");
    let (stats, contribs, global) = futures::join!(
        api_get("/api/stats"),
        api_get(&contrib_url),
        api_get(&global_url),
    );
    if let Ok(data) = stats
        && let Ok(stats) = serde_json::from_value::<Stats>(data)
    {
        state.stats.set(Some(stats));
    }
    if let Ok(data) = contribs
        && let Ok(contribs) = serde_json::from_value::<Vec<Contribution>>(data)
    {
        state.contributions.set(contribs);
    }
    if let Ok(data) = global
        && let Ok(matrix) = serde_json::from_value::<GlobalMatrixData>(data)
    {
        state.global_matrix.set(Some(matrix));
    }
}

/// What the viewer's own page shows: their duels and their place among
/// the voters.
pub async fn load_mine(state: &AppState) {
    let ts = js_sys::Date::now() as u64;
    let mine_url = format!("/api/user-matrix?user_id={}&_={ts}", state.user_id);
    let contrib_url = format!("/api/user-contributions?user_id={}", state.user_id);
    let global_url = format!("/api/global-matrix?_={ts}");
    let (mine, contribs, global) = futures::join!(
        api_get(&mine_url),
        api_get(&contrib_url),
        api_get(&global_url),
    );
    if let Ok(data) = mine
        && let Ok(matrix) = serde_json::from_value::<UserMatrixData>(data)
    {
        state.user_matrix.set(Some(matrix));
    }
    if let Ok(data) = contribs
        && let Ok(contribs) = serde_json::from_value::<Vec<Contribution>>(data)
    {
        state.contributions.set(contribs);
    }
    // The duel a cell opens tells how everyone voted.
    if let Ok(data) = global
        && let Ok(matrix) = serde_json::from_value::<GlobalMatrixData>(data)
    {
        state.global_matrix.set(Some(matrix));
    }
}

/// Vote `outcome` on `a` and `b` from a duel's sheet, or with `/api/unvote`
/// take the vote back, then show the matrices anew.
pub async fn matrix_action(state: &AppState, endpoint: &str, a: usize, b: usize, outcome: Outcome) {
    let _ = api_post(endpoint, &vote_body(state, a, b, outcome)).await;
    load_mine(state).await;
}

pub async fn reset_votes(state: &AppState) {
    let _ = api_post(
        "/api/reset-votes",
        &serde_json::json!({ "user_id": state.user_id }),
    )
    .await;

    state.vote_history.set(Vec::new());
    load_mine(state).await;
    load_board(state).await;
    load_pair(state).await;
}

pub fn init_vote_stream(state: &AppState) {
    let state = state.clone();
    let es = web_sys::EventSource::new("/api/vote/stream").unwrap();

    let state_msg = state.clone();
    let onmessage =
        Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            let Some(data) = e.data().as_string() else {
                return;
            };
            let Ok(event) = serde_json::from_str::<versus_shared::LiveEvent>(&data) else {
                return;
            };
            let s = state_msg.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let mut known = titles(&s);
                match event.change {
                    versus_shared::LiveChange::Film { .. } => crate::wiki::reload(&s).await,
                    _ => load_board(&s).await,
                }
                match s.page.get_untracked() {
                    Page::Board => load_backstage(&s).await,
                    Page::Me => load_mine(&s).await,
                    _ => {}
                }
                if event.user_id == s.user_id {
                    return;
                }
                // Others' votes only make the ranking's tab pulse; what
                // changes the catalog is told.
                if let versus_shared::LiveChange::Film { .. } = event.change {
                    known.extend(titles(&s));
                    s.add_toast(live_message(&event.change, &known));
                } else {
                    s.live_pulse.update(|n| *n += 1);
                }
            });
        });
    es.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let es_clone = es.clone();
    let onerror = Closure::<dyn Fn()>::new(move || {
        es_clone.close();
        let s = state.clone();
        crate::timeout::Timeout::new(5_000, move || {
            init_vote_stream(&s);
        })
        .forget();
    });
    es.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
}

/// Every item's title, published or proposed.
fn titles(state: &AppState) -> HashMap<usize, String> {
    let mut titles: HashMap<usize, String> = state
        .proposals
        .get_untracked()
        .into_iter()
        .map(|p| (p.film.id, p.film.title))
        .collect();
    titles.extend(
        state
            .films
            .get_untracked()
            .into_iter()
            .map(|f| (f.id, f.title)),
    );
    titles
}

/// What a change to the catalog by someone else is called in the toast, as
/// escaped HTML; `titles` names the items, including any the change removed.
fn live_message(change: &versus_shared::LiveChange, titles: &HashMap<usize, String>) -> String {
    use crate::i18n::{say, t};
    use versus_shared::{FilmChange, LiveChange};
    let title = |id: usize| titles.get(&id).cloned().unwrap_or_else(|| "?".into());
    let LiveChange::Film { film, change } = *change else {
        return String::new();
    };
    let template = match change {
        FilmChange::Edited => t().live_edited,
        FilmChange::Proposed => t().live_proposed,
        FilmChange::Approved => t().live_approved,
        FilmChange::Rejected => t().live_rejected,
    };
    html_escape(&say(template, &[("title", &title(film))]))
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

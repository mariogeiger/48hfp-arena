use crate::state::*;
use gloo_net::http::Request;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

async fn api_get(url: &str) -> Result<serde_json::Value, String> {
    let resp = Request::get(url).send().await.map_err(|e| e.to_string())?;
    if resp.status() == 403
        && let Some(state) = leptos::prelude::use_context::<AppState>()
    {
        state.banned.set(true);
    }
    resp.json().await.map_err(|e| e.to_string())
}

async fn api_post(url: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let resp = Request::post(url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 403
        && let Some(state) = leptos::prelude::use_context::<AppState>()
    {
        state.banned.set(true);
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn boot(state: &AppState) {
    let films_url = "/api/films";
    let board_url = "/api/leaderboard";

    let (films_res, board_res) = futures::join!(async { api_get(films_url).await }, async {
        api_get(board_url).await
    },);

    if let Ok(films_json) = films_res
        && let Ok(mut films) = serde_json::from_value::<Vec<filmrank_shared::Film>>(films_json)
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
    }

    // Save selection if user already has films selected
    let selected = state.selected_ids.get_untracked();
    if selected.len() >= 2 {
        save_selection(state).await;
    }

    // Navigate to initial page
    let hash = web_sys::window()
        .unwrap()
        .location()
        .hash()
        .unwrap_or_default();
    let hash = hash.trim_start_matches('#');
    let selected_count = state.selected_ids.get_untracked().len();

    if let Some(page) = Page::from_str(hash) {
        state.navigate(page);
    } else if selected_count == 0 {
        state.navigate(Page::Select);
    } else {
        state.navigate(Page::Swipe);
    }
}

pub async fn save_selection(state: &AppState) {
    let ids: Vec<usize> = state.selected_ids.get_untracked().into_iter().collect();
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let _ = storage.set_item("filmrank_selected", &serde_json::to_string(&ids).unwrap());

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

    let Ok(data) = api_get(&url).await else {
        return;
    };

    if data.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
        let selected = state.selected_ids.get_untracked();
        let fid = state.focus_film_id.get_untracked();
        let reason = if selected.len() < 2 {
            PairDoneReason::NotEnough
        } else if fid.is_some() {
            PairDoneReason::FocusDone
        } else {
            PairDoneReason::AllDone
        };
        state.pair.set(None);
        state.pair_status.set(Some(reason));
        state
            .vote_count
            .set(data.get("votes").and_then(|v| v.as_u64()).unwrap_or(0) as usize);
    } else if let (Some(a), Some(b)) = (data.get("a"), data.get("b"))
        && let (Ok(a), Ok(b)) = (
            serde_json::from_value::<filmrank_shared::Film>(a.clone()),
            serde_json::from_value::<filmrank_shared::Film>(b.clone()),
        )
    {
        state.pair.set(Some(FilmPair { a, b }));
        state.pair_status.set(None);
        state
            .vote_count
            .set(data.get("votes").and_then(|v| v.as_u64()).unwrap_or(0) as usize);
    }
}

pub async fn cast_vote(state: &AppState, winner_id: usize, loser_id: usize) {
    let _ = api_post(
        "/api/vote",
        &serde_json::json!({
            "user_id": state.user_id,
            "winner_id": winner_id,
            "loser_id": loser_id,
        }),
    )
    .await;

    state.vote_history.update(|h| {
        h.push(VoteRecord {
            winner_id,
            loser_id,
        });
    });

    load_pair(state).await;
}

pub async fn undo_vote(state: &AppState) {
    let history = state.vote_history.get_untracked();
    let Some(last) = history.last().cloned() else {
        return;
    };

    let _ = api_post(
        "/api/unvote",
        &serde_json::json!({
            "user_id": state.user_id,
            "winner_id": last.winner_id,
            "loser_id": last.loser_id,
        }),
    )
    .await;

    let films = state.films.get_untracked();
    let a = films.iter().find(|f| f.id == last.winner_id).cloned();
    let b = films.iter().find(|f| f.id == last.loser_id).cloned();

    state.vote_history.update(|h| {
        h.pop();
    });
    state.vote_count.update(|c| *c = c.saturating_sub(1));
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

pub async fn load_board(state: &AppState) {
    if let Ok(data) = api_get("/api/leaderboard").await
        && let Ok(board) = serde_json::from_value::<Vec<LeaderboardEntry>>(data)
    {
        state.board.set(board);
    }
}

pub async fn load_more(state: &AppState) {
    let stats_url = "/api/stats";
    let contrib_url = format!("/api/user-contributions?user_id={}", state.user_id);

    let (stats_res, contrib_res) = futures::join!(async { api_get(stats_url).await }, async {
        api_get(&contrib_url).await
    },);

    if let Ok(data) = stats_res
        && let Ok(stats) = serde_json::from_value::<Stats>(data)
    {
        state.stats.set(Some(stats));
    }
    if let Ok(data) = contrib_res
        && let Ok(contribs) = serde_json::from_value::<Vec<Contribution>>(data)
    {
        state.contributions.set(contribs);
    }

    refresh_matrices(state).await;
}

async fn refresh_matrices(state: &AppState) {
    let ts = js_sys::Date::now() as u64;
    let user_matrix_url = format!("/api/user-matrix?user_id={}&_={}", state.user_id, ts);
    let global_matrix_url = format!("/api/global-matrix?_={}", ts);

    let (um_res, gm_res) = futures::join!(async { api_get(&user_matrix_url).await }, async {
        api_get(&global_matrix_url).await
    },);

    if let Ok(data) = um_res
        && let Ok(matrix) = serde_json::from_value::<UserMatrixData>(data)
    {
        state.user_matrix.set(Some(matrix));
    }
    if let Ok(data) = gm_res
        && let Ok(matrix) = serde_json::from_value::<GlobalMatrixData>(data)
    {
        state.global_matrix.set(Some(matrix));
    }
}

pub async fn matrix_action(state: &AppState, endpoint: &str, w: usize, l: usize) {
    let _ = api_post(
        endpoint,
        &serde_json::json!({
            "user_id": state.user_id,
            "winner_id": w,
            "loser_id": l,
        }),
    )
    .await;

    refresh_matrices(state).await;
}

pub async fn reset_votes(state: &AppState) {
    let _ = api_post(
        "/api/reset-votes",
        &serde_json::json!({ "user_id": state.user_id }),
    )
    .await;

    load_more(state).await;
    load_board(state).await;
    load_pair(state).await;
}

pub fn init_vote_stream(state: &AppState) {
    let state = state.clone();
    let es = web_sys::EventSource::new("/api/vote/stream").unwrap();

    let state_msg = state.clone();
    let onmessage =
        Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if let Some(data_str) = e.data().as_string()
                && let Ok(event) = serde_json::from_str::<serde_json::Value>(&data_str)
            {
                let user_id = event
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let message = event
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let s = state_msg.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    load_board(&s).await;
                    let page = s.page.get_untracked();
                    if page == Page::More {
                        load_more(&s).await;
                    }
                });

                if user_id != state_msg.user_id {
                    state_msg.add_toast(html_escape(&message));
                }
            }
        });
    es.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let es_clone = es.clone();
    let onerror = Closure::<dyn Fn()>::new(move || {
        es_clone.close();
        let s = state.clone();
        gloo_timers::callback::Timeout::new(5_000, move || {
            init_vote_stream(&s);
        })
        .forget();
    });
    es.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

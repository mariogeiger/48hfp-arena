//! The ranking: a podium for the first three, the others below, then the
//! items without enough votes yet in a random order, then on an editable
//! site the proposals, and behind the scenes how it all adds up. Drawn as
//! HTML so that items slide to their new place when the ranking moves.

use crate::api::html_escape;
use crate::components::card::meta;
use crate::components::poster::{give_way_to_tile, poster_html};
use crate::i18n::{or_default, record, say, t};
use crate::state::{AppState, LeaderboardEntry, Page, Sheet, SiteConfig};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use versus_shared::Film;
use wasm_bindgen::JsCast;

#[component]
pub fn BoardPage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let editable = cfg.editable;
    let unranked_note = or_default(&cfg.unranked_banner, t().board_unranked);
    // Each item's random sort key, drawn once per page load so the unranked
    // order stays put while votes arrive.
    let shuffle_keys = StoredValue::new(HashMap::<usize, f64>::new());

    Effect::new({
        let state = state.clone();
        move || {
            let board = state.board.get();
            let films = state.films.get();
            // Drawn while in view only, so that it slides when looked at.
            if state.page.get() != Page::Board {
                return;
            }
            let Some(list) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("board-list"))
            else {
                return;
            };

            let ranked_ids: HashSet<usize> = board.iter().map(|item| item.film_id).collect();
            let mut unranked: Vec<&Film> = films
                .iter()
                .filter(|film| !ranked_ids.contains(&film.id))
                .collect();
            shuffle_keys.update_value(|keys| {
                for film in &unranked {
                    keys.entry(film.id).or_insert_with(js_sys::Math::random);
                }
            });
            shuffle_keys.with_value(|keys| {
                unranked.sort_by(|a, b| keys[&a.id].total_cmp(&keys[&b.id]));
            });
            let by_id: HashMap<usize, &Film> = films.iter().map(|f| (f.id, f)).collect();
            let ranked: Vec<(&LeaderboardEntry, &Film)> = board
                .iter()
                .filter_map(|item| Some((item, *by_id.get(&item.film_id)?)))
                .collect();

            let before = snapshot(&list);
            list.set_inner_html(&board_html(&ranked, &unranked, &unranked_note));
            slide(&list, &before);
        }
    });

    // A tap on an item opens its sheet.
    let open_item = move |ev: web_sys::MouseEvent| {
        let id = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("[data-film-id]").ok().flatten())
            .and_then(|item| item.get_attribute("data-film-id"))
            .and_then(|id| id.parse().ok());
        if let Some(id) = id {
            state.open_sheet(Sheet::Item(id));
        }
    };

    view! {
        <div class="board">
            <div
                class="board-list"
                id="board-list"
                on:click=open_item
                on:error:capture=move |ev| give_way_to_tile(&ev)
            ></div>
            {editable.then(|| view! { <crate::components::proposals::Proposals /> })}
            <crate::pages::backstage::Backstage />
        </div>
    }
}

fn board_html(
    ranked: &[(&LeaderboardEntry, &Film)],
    unranked: &[&Film],
    unranked_note: &str,
) -> String {
    let mut html = String::new();
    let podium = ranked.len() >= 3;
    if podium {
        // Without posters, the tiles are square.
        html.push_str(if SiteConfig::get().has_image {
            r#"<div class="podium">"#
        } else {
            r#"<div class="podium podium-tiles">"#
        });
        // Second, first, third: the winner in the middle.
        for place in [2, 1, 3] {
            let (item, film) = ranked[place - 1];
            html.push_str(&format!(
                r#"<button class="podium-spot place-{place}" data-film-id="{id}">
                    <span class="podium-art">{art}<span class="podium-medal">{place}</span></span>
                    <span class="podium-title">{title}</span>
                    <span class="board-score">{score}</span>
                    <span class="podium-step"></span>
                </button>"#,
                id = film.id,
                art = poster_html(&film.poster_url, &film.title, film.id),
                title = html_escape(&film.title),
                score = item.rating.round() as i64,
            ));
        }
        html.push_str("</div>");
    }
    let rest = if podium { &ranked[3..] } else { ranked };
    let first_rank = if podium { 4 } else { 1 };
    if !rest.is_empty() {
        html.push_str(r#"<div class="board-rows">"#);
        for (i, (item, film)) in rest.iter().enumerate() {
            let stats = format!(
                r#"<span class="board-score">{}</span><span class="board-record">{}</span>"#,
                item.rating.round() as i64,
                record(item.wins, item.ties, item.losses),
            );
            html.push_str(&row_html(&(first_rank + i).to_string(), film, &stats));
        }
        html.push_str("</div>");
    }
    if !unranked.is_empty() {
        html.push_str(&format!(
            r#"<div class="board-unranked"><h3>{}</h3><p>{}</p></div><div class="board-rows unranked">"#,
            html_escape(&say(t().board_unranked_title, &[])),
            html_escape(unranked_note),
        ));
        for film in unranked {
            html.push_str(&row_html("?", film, ""));
        }
        html.push_str("</div>");
    }
    html
}

fn row_html(rank: &str, film: &Film, stats: &str) -> String {
    let art = if SiteConfig::get().has_image {
        format!(
            r#"<span class="board-art">{}</span>"#,
            poster_html(&film.poster_url, &film.title, film.id)
        )
    } else {
        String::new()
    };
    format!(
        r#"<div class="board-row" data-film-id="{id}">
            <span class="board-rank">{rank}</span>
            {art}
            <span class="board-info">
                <span class="board-title">{title}</span>
                <span class="board-meta">{meta}</span>
            </span>
            <span class="board-stats">{stats}</span>
        </div>"#,
        id = film.id,
        title = html_escape(&film.title),
        meta = html_escape(&meta(film)),
    )
}

/// Where each item stood and what it scored, before a redraw.
struct Snapshot {
    top: HashMap<String, f64>,
    score: HashMap<String, String>,
}

fn items(list: &web_sys::Element) -> Vec<web_sys::HtmlElement> {
    let Ok(nodes) = list.query_selector_all("[data-film-id]") else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|i| nodes.item(i)?.dyn_into::<web_sys::HtmlElement>().ok())
        .collect()
}

fn snapshot(list: &web_sys::Element) -> Snapshot {
    let mut top = HashMap::new();
    let mut score = HashMap::new();
    for el in items(list) {
        let id = el.get_attribute("data-film-id").unwrap_or_default();
        top.insert(id.clone(), el.get_bounding_client_rect().top());
        if let Ok(Some(s)) = el.query_selector(".board-score") {
            score.insert(id, s.text_content().unwrap_or_default());
        }
    }
    Snapshot { top, score }
}

/// Slide each item from where it stood to where it stands (FLIP), and make
/// the scores that changed jump.
fn slide(list: &web_sys::Element, before: &Snapshot) {
    if before.top.is_empty() {
        return;
    }
    let mut moved = Vec::new();
    for el in items(list) {
        let id = el.get_attribute("data-film-id").unwrap_or_default();
        if let Some(&old) = before.top.get(&id) {
            let dy = old - el.get_bounding_client_rect().top();
            if dy.abs() >= 1.0 {
                moved.push((el.clone(), dy));
            }
        }
        if let Ok(Some(score)) = el.query_selector(".board-score")
            && before
                .score
                .get(&id)
                .is_some_and(|old| *old != score.text_content().unwrap_or_default())
        {
            let _ = score.class_list().add_1("changed");
        }
    }
    for (el, dy) in &moved {
        let _ = el.style().set_property("transition", "none");
        let _ = el
            .style()
            .set_property("transform", &format!("translateY({dy}px)"));
    }
    if let Some((first, _)) = moved.first() {
        let _ = first.offset_height();
    }
    for (el, _) in &moved {
        let _ = el
            .style()
            .set_property("transition", "transform 0.5s cubic-bezier(.2,.8,.2,1)");
        let _ = el.style().set_property("transform", "");
    }
}

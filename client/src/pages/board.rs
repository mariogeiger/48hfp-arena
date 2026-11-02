use crate::api;
use crate::components::poster;
use crate::i18n::{or_default, say, t};
use crate::state::{AppState, Sheet, SiteConfig};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use versus_shared::Film;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[component]
pub fn BoardPage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let has_image = cfg.has_image;
    let editable = cfg.editable;
    let unranked_banner = or_default(&cfg.unranked_banner, t().board_unranked);
    // Each item's random sort key, drawn once per page load so the unranked
    // order stays put while votes arrive.
    let shuffle_keys = StoredValue::new(HashMap::<usize, f64>::new());

    // Render board imperatively for FLIP animation
    // Track both board data AND page — only render+animate when board page is active
    Effect::new({
        let state = state.clone();
        move || {
            let board = state.board.get();
            let films = state.films.get();
            let page = state.page.get();
            if page != crate::state::Page::Board {
                return; // Don't render/animate while hidden — wait until user navigates here
            }
            let doc = web_sys::window().unwrap().document().unwrap();
            let Some(list) = doc.get_element_by_id("board-list") else {
                return;
            };

            // FLIP step 1: snapshot old positions and scores
            let mut old_pos: HashMap<String, f64> = HashMap::new();
            let mut old_scores: HashMap<String, String> = HashMap::new();
            if let Ok(nodes) = list.query_selector_all("[data-film-id]") {
                for i in 0..nodes.length() {
                    if let Some(el) = nodes.item(i)
                        && let Ok(el) = el.dyn_into::<web_sys::Element>()
                    {
                        let id = el.get_attribute("data-film-id").unwrap_or_default();
                        old_pos.insert(id.clone(), el.get_bounding_client_rect().top());
                        if let Ok(Some(score_el)) = el.query_selector(".board-score") {
                            old_scores.insert(id, score_el.text_content().unwrap_or_default());
                        }
                    }
                }
            }

            // Render new DOM: the ranked items, then those without enough votes
            // in this page load's random order, under a banner saying so.
            let ranked: HashSet<usize> = board.iter().map(|item| item.film_id).collect();
            let mut unranked: Vec<&Film> = films
                .iter()
                .filter(|film| !ranked.contains(&film.id))
                .collect();
            shuffle_keys.update_value(|keys| {
                for film in &unranked {
                    keys.entry(film.id).or_insert_with(js_sys::Math::random);
                }
            });
            shuffle_keys.with_value(|keys| {
                unranked.sort_by(|a, b| keys[&a.id].total_cmp(&keys[&b.id]));
            });

            let item_html = |class: &str, rank: &str, film: &Film, stats: &str| {
                let poster = if has_image {
                    poster::poster_html(&film.poster_url)
                } else {
                    String::new()
                };
                let city = if film.city.is_empty() {
                    String::new()
                } else {
                    format!(" &middot; {}", api::html_escape(&film.city))
                };
                format!(
                    r#"<div class="{}" data-film-id="{}">
                        <div class="board-rank">{}</div>
                        {}
                        <div class="board-info">
                            <div class="board-title">{}</div>
                            <div class="board-meta">{}{}</div>
                        </div>
                        <div class="board-stats">{}</div>
                    </div>"#,
                    class,
                    film.id,
                    rank,
                    poster,
                    api::html_escape(&film.title),
                    api::html_escape(&film.team),
                    city,
                    stats,
                )
            };

            let by_id: HashMap<usize, &Film> = films.iter().map(|f| (f.id, f)).collect();
            let mut html: String = board
                .iter()
                .filter_map(|item| Some((item, by_id.get(&item.film_id)?)))
                .enumerate()
                .map(|(i, (item, film))| {
                    let stats = format!(
                        r#"<div class="board-score">{}</div>
                        <div class="board-record">{}</div>"#,
                        item.rating.round() as i64,
                        say(t().board_record, &[("w", &item.wins), ("l", &item.losses)]),
                    );
                    item_html("board-item", &(i + 1).to_string(), &film, &stats)
                })
                .collect();
            if !unranked.is_empty() {
                html.push_str(&format!(
                    r#"<div class="board-banner">{}</div>"#,
                    api::html_escape(&unranked_banner)
                ));
                for film in &unranked {
                    html.push_str(&item_html("board-item board-unranked", "?", film, ""));
                }
            }

            list.set_inner_html(&html);

            // FLIP steps 2-4: invert + play (batched to avoid layout thrashing)
            if !old_pos.is_empty() {
                // Collect elements that moved
                let mut moved: Vec<(web_sys::HtmlElement, f64)> = Vec::new();
                if let Ok(nodes) = list.query_selector_all("[data-film-id]") {
                    for i in 0..nodes.length() {
                        if let Some(node) = nodes.item(i)
                            && let Ok(el) = node.dyn_into::<web_sys::HtmlElement>()
                        {
                            let id = el.get_attribute("data-film-id").unwrap_or_default();
                            if let Some(&old_top) = old_pos.get(&id) {
                                let new_top = el.get_bounding_client_rect().top();
                                let dy = old_top - new_top;
                                if dy.abs() >= 1.0 {
                                    moved.push((el, dy));
                                }
                            }
                        }
                    }
                }

                if !moved.is_empty() {
                    // Step 1: Set inverse transforms with no transition (batch writes)
                    for (el, dy) in &moved {
                        let _ = el.style().set_property("transition", "none");
                        let _ = el
                            .style()
                            .set_property("transform", &format!("translateY({}px)", dy));
                    }

                    // Step 2: Single reflow to commit the starting positions
                    let _ = moved[0].0.offset_height();

                    // Step 3: Enable transition and clear transform (batch writes)
                    for (el, _) in &moved {
                        let _ = el
                            .style()
                            .set_property("transition", "transform 0.4s cubic-bezier(.4,0,.2,1)");
                        let _ = el.style().set_property("transform", "");
                    }
                }

                // Score-changed highlight — only bump scores that actually changed
                if !old_scores.is_empty()
                    && let Ok(items) = list.query_selector_all("[data-film-id]")
                {
                    for i in 0..items.length() {
                        if let Some(node) = items.item(i)
                            && let Ok(el) = node.dyn_into::<web_sys::Element>()
                        {
                            let id = el.get_attribute("data-film-id").unwrap_or_default();
                            if let Ok(Some(score_el)) = el.query_selector(".board-score") {
                                let new_score = score_el.text_content().unwrap_or_default();
                                let changed = old_scores
                                    .get(&id)
                                    .map(|old| *old != new_score)
                                    .unwrap_or(false);
                                if changed
                                    && let Ok(hel) = score_el.dyn_into::<web_sys::HtmlElement>()
                                {
                                    hel.class_list().add_1("score-changed").ok();
                                    let hel2 = hel.clone();
                                    let cb = Closure::<dyn Fn()>::new(move || {
                                        hel2.class_list().remove_1("score-changed").ok();
                                    });
                                    hel.add_event_listener_with_callback(
                                        "animationend",
                                        cb.as_ref().unchecked_ref(),
                                    )
                                    .ok();
                                    cb.forget();
                                }
                            }
                        }
                    }
                }
            }

            // Poster error handling for innerHTML-rendered images
            if let Ok(imgs) = list.query_selector_all("img.poster") {
                for i in 0..imgs.length() {
                    if let Some(node) = imgs.item(i)
                        && let Ok(img) = node.dyn_into::<web_sys::HtmlElement>()
                    {
                        let img2 = img.clone();
                        let cb = Closure::<dyn Fn()>::new(move || {
                            let ph = web_sys::window()
                                .unwrap()
                                .document()
                                .unwrap()
                                .create_element("div")
                                .unwrap();
                            ph.set_class_name("poster-ph");
                            ph.set_inner_html("&#127916;");
                            if let Some(parent) = img2.parent_node() {
                                let _ = parent.replace_child(&ph, &img2);
                            }
                        });
                        img.add_event_listener_with_callback("error", cb.as_ref().unchecked_ref())
                            .ok();
                        cb.forget();
                    }
                }
            }
        }
    });

    // A tap on an item opens its sheet, which plays its video.
    let open_item = move |ev: web_sys::MouseEvent| {
        let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let id = target
            .closest("[data-film-id]")
            .ok()
            .flatten()
            .and_then(|item| item.get_attribute("data-film-id"))
            .and_then(|id| id.parse().ok());
        if let Some(id) = id {
            state.open_sheet(Sheet::Item(id));
        }
    };

    view! {
        <div class="page-header">
            <h1>{t().board_title}</h1>
            <p>{t().board_subtitle}</p>
        </div>
        <div class="board-list" id="board-list" on:click=open_item></div>
        {editable.then(|| view! { <crate::components::proposals::Proposals /> })}
        <div class="board-export">
            <a href="/api/leaderboard.csv" target="_blank">{t().board_export}</a>
        </div>
    }
}

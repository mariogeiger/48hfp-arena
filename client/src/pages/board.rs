use crate::api;
use crate::components::poster;
use crate::state::AppState;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[component]
pub fn BoardPage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Render board imperatively for FLIP animation
    // Track both board data AND page — only render+animate when board page is active
    Effect::new({
        let state = state.clone();
        move || {
            let board = state.board.get();
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

            // Render new DOM
            if board.is_empty() {
                list.set_inner_html(
                    r#"<div class="board-empty"><h3>No votes yet</h3><p>Start comparing films to build the leaderboard!</p></div>"#,
                );
                return;
            }

            let html: String = board
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let poster = poster::poster_html(&item.poster_url);
                    let title = api::html_escape(&item.title);
                    let team = api::html_escape(&item.team);
                    let city = if item.city.is_empty() {
                        String::new()
                    } else {
                        format!(" &middot; {}", api::html_escape(&item.city))
                    };
                    let video = if item.video_url.is_empty() {
                        String::new()
                    } else {
                        format!(
                            r#" &middot; <a href="{}" target="_blank" class="board-video">Watch</a>"#,
                            api::html_escape(&item.video_url)
                        )
                    };
                    let rating = item.rating.round() as i64;
                    format!(
                        r#"<div class="board-item" data-film-id="{}">
                            <div class="board-rank">{}</div>
                            {}
                            <div class="board-info">
                                <div class="board-title">{}</div>
                                <div class="board-meta">{}{}{}</div>
                            </div>
                            <div class="board-stats">
                                <div class="board-score">{}</div>
                                <div class="board-record">{}W - {}L</div>
                            </div>
                        </div>"#,
                        item.film_id,
                        i + 1,
                        poster,
                        title,
                        team,
                        city,
                        video,
                        rating,
                        item.wins,
                        item.losses,
                    )
                })
                .collect();

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

    view! {
        <div class="page-header">
            <h1>"Leaderboard"</h1>
            <p>"Bradley-Terry rankings based on all voter comparisons"</p>
        </div>
        <div class="board-list" id="board-list"></div>
        <div class="board-export">
            <a href="/api/leaderboard.csv" target="_blank">"Export as CSV"</a>
        </div>
    }
}

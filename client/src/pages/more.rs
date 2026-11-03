//! The More page: how the ranking works, the voting stats, the reset of
//! one's votes, continuing on another device, everyone's contributions and
//! the vote matrices.

use crate::api;
use crate::i18n::{fill, say, t};
use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn MorePage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    view! {
        <div class="page-header">
            <h1>{t().more_title}</h1>
            <p>{t().more_subtitle}</p>
        </div>

        <crate::pages::about::HowItWorks />

        <StatsSection />

        <div class="reset-section">
            <button
                class="reset-btn"
                on:click=move |_| {
                    let window = web_sys::window().unwrap();
                    if window.confirm_with_message(t().reset_confirm).unwrap_or(false) {
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            api::reset_votes(&s).await;
                        });
                    }
                }
            >
                {t().reset_button}
            </button>
        </div>

        <crate::components::identity::SessionSection />
        <ContributionsSection />
        <crate::pages::matrices::MatrixSections />
    }
}
#[component]
fn StatsSection() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let item_label = say("{Items}", &[]);
    let voted_label = say(t().stats_voted_on, &[]);

    // Imperative rendering with change detection for bump animation
    Effect::new({
        let state = state.clone();
        let item_label = item_label.clone();
        let voted_label = voted_label.clone();
        let prev_stats: std::cell::RefCell<Option<crate::state::Stats>> =
            std::cell::RefCell::new(None);
        move || {
            let stats = state.stats.get();
            let Some(s) = stats else { return };
            let doc = web_sys::window().unwrap().document().unwrap();
            let Some(container) = doc.get_element_by_id("stats-content") else {
                return;
            };

            let ps = prev_stats.borrow();
            let changed_votes = ps
                .as_ref()
                .map(|p| p.total_votes != s.total_votes)
                .unwrap_or(false);
            let changed_users = ps
                .as_ref()
                .map(|p| p.active_users != s.active_users)
                .unwrap_or(false);
            let changed_films = ps
                .as_ref()
                .map(|p| p.total_films != s.total_films)
                .unwrap_or(false);
            let changed_fwv = ps
                .as_ref()
                .map(|p| p.films_with_votes != s.films_with_votes)
                .unwrap_or(false);

            // Render without animation class first
            let html = format!(
                r#"<div class="stats-section">
                    <h3>{}</h3>
                    <div class="stats-grid">
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">{}</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">{}</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">{}</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">{}</div></div>
                    </div>
                </div>"#,
                t().stats_heading,
                changed_votes,
                s.total_votes,
                t().stats_votes,
                changed_users,
                s.active_users,
                t().stats_voters,
                changed_films,
                s.total_films,
                item_label,
                changed_fwv,
                s.films_with_votes,
                voted_label,
            );
            container.set_inner_html(&html);

            // After reflow, add the animation class to changed elements
            trigger_bump_animations(&container);

            drop(ps);
            *prev_stats.borrow_mut() = Some(s);
        }
    });

    view! {
        <div id="stats-content"></div>
    }
}

#[component]
fn ContributionsSection() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Imperative rendering with change detection for bump animation
    Effect::new({
        let state = state.clone();
        let prev_votes: std::cell::RefCell<std::collections::HashMap<u64, u64>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
        move || {
            let contribs = state.contributions.get();
            let doc = web_sys::window().unwrap().document().unwrap();
            let Some(container) = doc.get_element_by_id("contributions-content") else {
                return;
            };

            if contribs.is_empty() {
                container.set_inner_html(&format!("<p>{}</p>", t().contrib_none));
                return;
            }

            let old_votes = prev_votes.borrow();
            let max = contribs.first().map(|c| c.votes).unwrap_or(1);

            let rows: String = contribs.iter().map(|u| {
                let changed = old_votes.get(&u.rank).map(|&v| v != u.votes).unwrap_or(false);
                let label = if u.is_you {
                    t().contrib_you.to_string()
                } else {
                    fill(t().contrib_voter, &[("n", &u.rank)])
                };
                let pairs = u.films_voted * (u.films_voted.saturating_sub(1)) / 2;
                let pct = if pairs > 0 {
                    format!("{:.1}%", (u.votes as f64 / pairs as f64) * 100.0)
                } else {
                    "0".to_string()
                };
                let bar_pct = if max > 0 { (u.votes as f64 / max as f64) * 100.0 } else { 0.0 };
                let you_class = if u.is_you { " contrib-you" } else { "" };
                format!(
                    r#"<div class="contrib-row"><span class="contrib-label">{}</span><div class="contrib-bar-track"><div class="contrib-bar{}" style="width:{}%"></div></div><span class="contrib-count" data-bump="{}" title="{}">{}</span></div>"#,
                    crate::api::html_escape(&label),
                    you_class,
                    bar_pct,
                    changed,
                    fill(t().contrib_coverage, &[("v", &u.votes), ("p", &pairs), ("pct", &pct)]),
                    say(t().contrib_count, &[("v", &u.votes), ("f", &u.films_voted)]),
                )
            }).collect();

            container.set_inner_html(&format!(r#"<div class="contrib-bars">{}</div>"#, rows));

            // Trigger bump animations on changed elements
            trigger_bump_animations(&container);

            drop(old_votes);
            *prev_votes.borrow_mut() = contribs.iter().map(|c| (c.rank, c.votes)).collect();
        }
    });

    view! {
        <div class="stats-section">
            <h3>{t().contrib_heading}</h3>
            <div id="contributions-content"></div>
        </div>
    }
}
/// Find elements with `data-bump="true"`, force a reflow, then add `changed` class
/// so the CSS animation triggers reliably.
fn trigger_bump_animations(container: &web_sys::Element) {
    use wasm_bindgen::JsCast;
    if let Ok(nodes) = container.query_selector_all("[data-bump='true']") {
        // Collect elements first
        let mut els = Vec::new();
        for i in 0..nodes.length() {
            if let Some(node) = nodes.item(i)
                && let Ok(el) = node.dyn_into::<web_sys::HtmlElement>()
            {
                els.push(el);
            }
        }
        if els.is_empty() {
            return;
        }
        // Force reflow
        if let Some(first) = els.first() {
            let _ = first.offset_height();
        }
        // Now add the class — animation starts from this frame
        for el in els {
            el.class_list().add_1("changed").ok();
        }
    }
}

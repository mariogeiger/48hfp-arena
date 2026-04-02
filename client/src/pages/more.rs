use crate::api;
use crate::components::matrix::*;
use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn MorePage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    view! {
        <div class="page-header">
            <h1>"More"</h1>
            <p>"Voting stats, your votes matrix, and global results"</p>
        </div>
        <div class="reset-section">
            <button
                class="reset-btn"
                on:click=move |_| {
                    let window = web_sys::window().unwrap();
                    if window.confirm_with_message("This will permanently delete all your votes. Are you sure?").unwrap_or(false) {
                        let s = state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            api::reset_votes(&s).await;
                        });
                    }
                }
            >
                "Reset All My Votes"
            </button>
        </div>

        <StatsSection />
        <ContributionsSection />
        <SuggestSection />
        <MatrixSections />

        <div class="about-section">
            <h3>"How It Works"</h3>
            <div class="about-content">
                <p inner_html=r#"Each head-to-head vote feeds a <a href="https://en.wikipedia.org/wiki/Bradley%E2%80%93Terry_model" target="_blank">Bradley&ndash;Terry model</a>, a statistical method for ranking items from pairwise comparisons. Every film gets a strength parameter &beta;. The probability that film&nbsp;A beats film&nbsp;B is simply &beta;<sub>A</sub>&thinsp;/&thinsp;(&beta;<sub>A</sub>&nbsp;+&nbsp;&beta;<sub>B</sub>)."# />
                <p inner_html=r#"Strengths are estimated using the <a href="https://en.wikipedia.org/wiki/MM_algorithm" target="_blank">MM&nbsp;algorithm</a> (minorization&ndash;maximization), which iterates until convergence. Films with zero wins are pinned to a near-zero score. The displayed score is <code>500 &times; log<sub>2</sub>(1 + &beta;)</code>, mapping the raw strength to a human-friendly number."# />
                <p inner_html=r#"Pairs are not presented randomly. The system uses <a href="https://en.wikipedia.org/wiki/Optimal_design#D-optimality" target="_blank">D-optimal experimental design</a> based on the Fisher Information matrix to pick the most informative pair next &mdash; prioritizing matchups between closely-ranked films and films with fewer comparisons. This means your votes reduce uncertainty as fast as possible."# />
                <p>"A film appears on the leaderboard once it has at least 10 comparisons from at least 2 different voters. All votes from all users are aggregated into one global ranking."</p>
            </div>
        </div>
    }
}

#[component]
fn StatsSection() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Imperative rendering with change detection for bump animation
    Effect::new({
        let state = state.clone();
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
                    <h3>Voting Stats</h3>
                    <div class="stats-grid">
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">Total Votes</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">Voters</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">Films</div></div>
                        <div class="stat-card"><div class="stat-value" data-bump="{}">{}</div><div class="stat-label">Films Voted On</div></div>
                    </div>
                </div>"#,
                changed_votes,
                s.total_votes,
                changed_users,
                s.active_users,
                changed_films,
                s.total_films,
                changed_fwv,
                s.films_with_votes,
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
        let prev_votes: std::cell::RefCell<std::collections::HashMap<String, u64>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
        move || {
            let contribs = state.contributions.get();
            let doc = web_sys::window().unwrap().document().unwrap();
            let Some(container) = doc.get_element_by_id("contributions-content") else {
                return;
            };

            if contribs.is_empty() {
                container.set_inner_html("<p>No contributions yet.</p>");
                return;
            }

            let old_votes = prev_votes.borrow();
            let max = contribs.first().map(|c| c.votes).unwrap_or(1);

            let rows: String = contribs.iter().map(|u| {
                let changed = old_votes.get(&u.label).map(|&v| v != u.votes).unwrap_or(false);
                let pairs = u.films_voted * (u.films_voted.saturating_sub(1)) / 2;
                let pct = if pairs > 0 {
                    format!("{:.1}%", (u.votes as f64 / pairs as f64) * 100.0)
                } else {
                    "0".to_string()
                };
                let bar_pct = if max > 0 { (u.votes as f64 / max as f64) * 100.0 } else { 0.0 };
                let you_class = if u.is_you { " contrib-you" } else { "" };
                format!(
                    r#"<div class="contrib-row"><span class="contrib-label">{}</span><div class="contrib-bar-track"><div class="contrib-bar{}" style="width:{}%"></div></div><span class="contrib-count" data-bump="{}" title="{} / {} possible pairs = {} coverage"><span class="contrib-num">{}</span> votes on <span class="contrib-num">{}</span> films</span></div>"#,
                    crate::api::html_escape(&u.label),
                    you_class,
                    bar_pct,
                    changed,
                    u.votes, pairs, pct,
                    u.votes,
                    u.films_voted,
                )
            }).collect();

            container.set_inner_html(&format!(r#"<div class="contrib-bars">{}</div>"#, rows));

            // Trigger bump animations on changed elements
            trigger_bump_animations(&container);

            drop(old_votes);
            *prev_votes.borrow_mut() = contribs
                .iter()
                .map(|c| (c.label.clone(), c.votes))
                .collect();
        }
    });

    view! {
        <div class="stats-section">
            <h3>"Voter Contributions"</h3>
            <div id="contributions-content"></div>
        </div>
    }
}

#[component]
fn SuggestSection() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let suggest_film = RwSignal::new(String::new());
    let suggest_title = RwSignal::new(String::new());
    let suggest_team = RwSignal::new(String::new());
    let suggest_city = RwSignal::new(String::new());
    let suggest_poster = RwSignal::new(String::new());
    let suggest_video = RwSignal::new(String::new());

    view! {
        <div class="suggest-section">
            <h3>"Suggest a Correction"</h3>
            <p class="suggest-hint">
                "Wrong title, missing video link, bad poster? Pick the film and fill in what needs changing. You can also suggest a new film."
            </p>
            <div class="suggest-form">
                <select
                    id="suggest-film"
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        suggest_film.set(val.clone());
                        if let Ok(id) = val.parse::<usize>() {
                            let films = state.films.get_untracked();
                            if let Some(film) = films.iter().find(|f| f.id == id) {
                                suggest_title.set(film.title.clone());
                                suggest_team.set(film.team.clone());
                                suggest_city.set(film.city.clone());
                                suggest_poster.set(film.poster_url.clone());
                                suggest_video.set(film.video_url.clone());
                            }
                        } else {
                            suggest_title.set(String::new());
                            suggest_team.set(String::new());
                            suggest_city.set(String::new());
                            suggest_poster.set(String::new());
                            suggest_video.set(String::new());
                        }
                    }
                >
                    <option value="">"Pick a film..."</option>
                    <For
                        each=move || state.films.get()
                        key=|f| f.id
                        let:film
                    >
                        <option value={film.id.to_string()}>
                            {format!("{} \u{2014} {}", film.title, film.team)}
                        </option>
                    </For>
                </select>
                <input type="text" id="suggest-title" placeholder="Title"
                    prop:value=move || suggest_title.get()
                    on:input=move |ev| suggest_title.set(event_target_value(&ev)) />
                <input type="text" id="suggest-team" placeholder="Team"
                    prop:value=move || suggest_team.get()
                    on:input=move |ev| suggest_team.set(event_target_value(&ev)) />
                <input type="text" id="suggest-city" placeholder="City"
                    prop:value=move || suggest_city.get()
                    on:input=move |ev| suggest_city.set(event_target_value(&ev)) />
                <input type="url" id="suggest-poster" placeholder="Poster URL"
                    prop:value=move || suggest_poster.get()
                    on:input=move |ev| suggest_poster.set(event_target_value(&ev)) />
                <input type="url" id="suggest-video" placeholder="Video URL (YouTube, Vimeo...)"
                    prop:value=move || suggest_video.get()
                    on:input=move |ev| suggest_video.set(event_target_value(&ev)) />
                <button on:click=move |_| {
                    let title = suggest_title.get_untracked();
                    let team = suggest_team.get_untracked();
                    let city = suggest_city.get_untracked();
                    let poster = suggest_poster.get_untracked();
                    let video = suggest_video.get_untracked();
                    let film_val = suggest_film.get_untracked();

                    if title.trim().is_empty() && team.trim().is_empty() && video.trim().is_empty() {
                        return;
                    }

                    let is_new = film_val.is_empty();
                    let film_name = if is_new {
                        if !title.trim().is_empty() { title.clone() } else { team.clone() }
                    } else {
                        // Find film name from the select
                        let films = state.films.get_untracked();
                        films.iter()
                            .find(|f| f.id.to_string() == film_val)
                            .map(|f| f.title.clone())
                            .unwrap_or_default()
                    };
                    let issue_title = if is_new {
                        format!("New film: {}", film_name)
                    } else {
                        format!("Correction: {}", film_name)
                    };
                    let mut lines = Vec::new();
                    if !title.trim().is_empty() { lines.push(format!("- **Title:** {}", title.trim())); }
                    if !team.trim().is_empty() { lines.push(format!("- **Team:** {}", team.trim())); }
                    if !city.trim().is_empty() { lines.push(format!("- **City:** {}", city.trim())); }
                    if !poster.trim().is_empty() { lines.push(format!("- **Poster URL:** {}", poster.trim())); }
                    if !video.trim().is_empty() { lines.push(format!("- **Video URL:** {}", video.trim())); }

                    let heading = if is_new { "## New film suggestion" } else { "## Suggested correction" };
                    let body = format!("{}\n\n{}\n", heading, lines.join("\n"));

                    let url = format!(
                        "https://github.com/mariogeiger/48hfp-arena/issues/new?title={}&body={}",
                        js_sys::encode_uri_component(&issue_title),
                        js_sys::encode_uri_component(&body),
                    );
                    let _ = web_sys::window().unwrap().open_with_url_and_target(&url, "_blank");
                }>
                    "Open GitHub Issue"
                </button>
            </div>
        </div>
    }
}

#[component]
fn MatrixSections() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Re-render user matrix when data or board changes
    Effect::new({
        let state = state.clone();
        move || {
            let data = state.user_matrix.get();
            let board = state.board.get();
            let Some(data) = data else { return };
            if data.films.is_empty() {
                let doc = web_sys::window().unwrap().document().unwrap();
                if let Some(el) = doc.get_element_by_id("user-matrix") {
                    el.set_inner_html(
                        r#"<p class="matrix-empty">No votes yet. Start comparing!</p>"#,
                    );
                }
                return;
            }

            let films = sort_films_by_board(&data.films, &board);
            let vote_map: std::collections::HashMap<String, usize> = data
                .votes
                .iter()
                .filter_map(|v| {
                    let a = v.film_a.min(v.film_b);
                    let b = v.film_a.max(v.film_b);
                    Some((format!("{},{}", a, b), v.winner?))
                })
                .collect();
            let legacy_set: std::collections::HashSet<String> = data
                .legacy_votes
                .iter()
                .map(|v| {
                    let a = v.film_a.min(v.film_b);
                    let b = v.film_a.max(v.film_b);
                    format!("{},{}", a, b)
                })
                .collect();

            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let vote_map_ci = vote_map.clone();
            let legacy_set_ci = legacy_set.clone();
            let films_tt = films.clone();
            let films_oc = films.clone();
            let vote_map_oc = vote_map.clone();
            let state_oc = state.clone();

            render_matrix_canvas(
                "user-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let row = &films_ci[ri];
                        let col = &films_ci[ci];
                        let a = row.id.min(col.id);
                        let b = row.id.max(col.id);
                        let key = format!("{},{}", a, b);
                        if let Some(&winner) = vote_map_ci.get(&key) {
                            let won = winner == row.id;
                            CellInfo {
                                bg: if won { CellBg::Win } else { CellBg::Loss },
                                text: if won { "W" } else { "L" }.to_string(),
                            }
                        } else if legacy_set_ci.contains(&key) {
                            CellInfo {
                                bg: CellBg::Legacy,
                                text: "?".to_string(),
                            }
                        } else {
                            CellInfo {
                                bg: CellBg::Empty,
                                text: String::new(),
                            }
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        format!("{} vs {}", films_tt[ri].title, films_tt[ci].title)
                    }),
                    on_click: Some(Box::new(move |ri, ci| {
                        let row = &films_oc[ri];
                        let col = &films_oc[ci];
                        let a = row.id.min(col.id);
                        let b = row.id.max(col.id);
                        let key = format!("{},{}", a, b);
                        let s = state_oc.clone();
                        if let Some(&winner) = vote_map_oc.get(&key) {
                            let loser = if winner == row.id { col.id } else { row.id };
                            wasm_bindgen_futures::spawn_local(async move {
                                crate::api::matrix_action(&s, "/api/unvote", winner, loser).await;
                            });
                        } else {
                            let (w, l) = (row.id, col.id);
                            wasm_bindgen_futures::spawn_local(async move {
                                crate::api::matrix_action(&s, "/api/vote", w, l).await;
                            });
                        }
                    })),
                },
            );
        }
    });

    // Re-render global matrix when data or board changes
    Effect::new({
        let state = state.clone();
        move || {
            let data = state.global_matrix.get();
            let board = state.board.get();
            let Some(data) = data else { return };
            if data.films.is_empty() {
                let doc = web_sys::window().unwrap().document().unwrap();
                if let Some(el) = doc.get_element_by_id("global-matrix") {
                    el.set_inner_html(r#"<p class="matrix-empty">No data yet.</p>"#);
                }
                return;
            }

            let films = sort_films_by_board(&data.films, &board);
            let win_map: std::collections::HashMap<String, u32> = data
                .wins
                .iter()
                .map(|w| (format!("{},{}", w.winner, w.loser), w.count))
                .collect();
            let score_map: std::collections::HashMap<usize, f64> = data
                .films
                .iter()
                .map(|f| (f.id, f.score.unwrap_or(1.0)))
                .collect();

            let matrix_films: Vec<MatrixFilmInfo> = films
                .iter()
                .map(|f| MatrixFilmInfo {
                    title: f.title.clone(),
                })
                .collect();

            let films_ci = films.clone();
            let win_map_ci = win_map.clone();
            let score_map_ci = score_map.clone();
            let films_tt = films.clone();
            let win_map_tt = win_map.clone();
            let score_map_tt = score_map.clone();

            render_matrix_canvas(
                "global-matrix",
                MatrixConfig {
                    films: matrix_films,
                    cell_info: Box::new(move |ri, ci| {
                        let row = &films_ci[ri];
                        let col = &films_ci[ci];
                        let w_rc = win_map_ci
                            .get(&format!("{},{}", row.id, col.id))
                            .copied()
                            .unwrap_or(0);
                        let w_cr = win_map_ci
                            .get(&format!("{},{}", col.id, row.id))
                            .copied()
                            .unwrap_or(0);
                        let total = w_rc + w_cr;
                        if total == 0 {
                            return CellInfo {
                                bg: CellBg::Empty,
                                text: String::new(),
                            };
                        }
                        let observed = w_rc as f64 / total as f64;
                        let bi = score_map_ci.get(&row.id).copied().unwrap_or(1.0);
                        let bj = score_map_ci.get(&col.id).copied().unwrap_or(1.0);
                        let predicted = bi / (bi + bj);
                        let residual = observed - predicted;
                        CellInfo {
                            bg: CellBg::Residual(residual),
                            text: w_rc.to_string(),
                        }
                    }),
                    tooltip: Box::new(move |ri, ci| {
                        let row = &films_tt[ri];
                        let col = &films_tt[ci];
                        let w_rc = win_map_tt
                            .get(&format!("{},{}", row.id, col.id))
                            .copied()
                            .unwrap_or(0);
                        let w_cr = win_map_tt
                            .get(&format!("{},{}", col.id, row.id))
                            .copied()
                            .unwrap_or(0);
                        let total = w_rc + w_cr;
                        if total == 0 {
                            return format!("{} vs {}: no votes", row.title, col.title);
                        }
                        let observed = w_rc as f64 / total as f64;
                        let bi = score_map_tt.get(&row.id).copied().unwrap_or(1.0);
                        let bj = score_map_tt.get(&col.id).copied().unwrap_or(1.0);
                        let predicted = bi / (bi + bj);
                        let residual = observed - predicted;
                        format!(
                            "{} vs {}\nobserved {}% / model {}% ({}{}%)",
                            row.title,
                            col.title,
                            (observed * 100.0) as i32,
                            (predicted * 100.0) as i32,
                            if residual > 0.0 { "+" } else { "" },
                            (residual * 100.0) as i32,
                        )
                    }),
                    on_click: None,
                },
            );
        }
    });

    view! {
        <div class="matrix-section">
            <h3>"Your Vote Matrix"</h3>
            <p class="matrix-hint">"Tap empty cell to vote (row wins). Tap filled cell to remove vote."</p>
            <div class="matrix-scroll-wrapper">
                <div id="user-matrix">
                    <p class="matrix-empty">"No votes yet. Start comparing!"</p>
                </div>
            </div>
        </div>
        <div class="matrix-section">
            <h3>"Global Win Matrix"</h3>
            <p class="matrix-hint">"Color = surprise: green = wins more than model expects, red = wins less. Hover for details."</p>
            <div class="matrix-scroll-wrapper">
                <div id="global-matrix">
                    <p class="matrix-empty">"No data yet."</p>
                </div>
            </div>
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

fn sort_films_by_board(
    films: &[crate::state::MatrixFilm],
    board: &[crate::state::LeaderboardEntry],
) -> Vec<crate::state::MatrixFilm> {
    let rank: std::collections::HashMap<usize, usize> = board
        .iter()
        .enumerate()
        .map(|(i, b)| (b.film_id, i))
        .collect();
    let mut sorted = films.to_vec();
    sorted.sort_by_key(|f| rank.get(&f.id).copied().unwrap_or(usize::MAX));
    sorted
}

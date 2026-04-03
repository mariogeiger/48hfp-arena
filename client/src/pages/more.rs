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

        <div class="about-section">
            <h3>"How It Works"</h3>
            <div class="about-content">

                <h4>"The Bradley\u{2013}Terry Model"</h4>
                <p>
                    "Each head-to-head vote feeds a "
                    <a href="https://en.wikipedia.org/wiki/Bradley%E2%80%93Terry_model" target="_blank">"Bradley\u{2013}Terry model"</a>
                    ", a statistical method for ranking items from pairwise comparisons. Every film "
                    <var>"i"</var>
                    " gets a positive strength parameter \u{03B2}"
                    <sub><var>"i"</var></sub>
                    ". The probability that film\u{00A0}"
                    <var>"i"</var>
                    " beats film\u{00A0}"
                    <var>"j"</var>
                    " is:"
                </p>
                <p class="math">
                    "P(" <var>"i"</var> " beats " <var>"j"</var>
                    ")\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
                    "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
                    "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
                </p>
                <p>
                    <b>"Intuition:"</b>
                    " think of \u{03B2} as the \u{201C}mass\u{201D} of a film. When two films are placed on a balance, the heavier one tips the scale in its favor. A film with twice the strength of another wins roughly two-thirds of the time. When two films have equal strength, the probability is \u{00BD}\u{00A0}\u{2014} a fair coin flip."
                </p>
                <BtSimulation />

                <h4>"Estimating Strengths: the MM Algorithm"</h4>
                <p>
                    "We observe wins and losses but don\u{2019}t know the true \u{03B2} values. The "
                    <a href="https://en.wikipedia.org/wiki/MM_algorithm" target="_blank">"MM\u{00A0}algorithm"</a>
                    " (minorization\u{2013}maximization) finds them by iterating a simple update rule until convergence:"
                </p>
                <p class="math">
                    "\u{03B2}" <sub><var>"i"</var></sub> <sup>"(new)"</sup>
                    "\u{00A0}=\u{00A0}"
                    <var>"w" <sub>"i"</sub></var>
                    "\u{2009}/\u{2009}\u{2211}" <sub><var>"j"</var> "\u{2260}" <var>"i"</var></sub>
                    "\u{00A0}"
                    <var>"n" <sub>"ij"</sub></var>
                    "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
                    "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
                </p>
                <p>
                    "where " <var>"w" <sub>"i"</sub></var>
                    " is the total number of wins for film\u{00A0}"
                    <var>"i"</var>
                    " across all opponents and "
                    <var>"n" <sub>"ij"</sub></var>
                    " is the total number of comparisons between "
                    <var>"i"</var> " and\u{00A0}" <var>"j"</var> "."
                </p>
                <p>
                    <b>"Intuition:"</b>
                    " the numerator says \u{201C}how often does this film actually win?\u{201D} The denominator says \u{201C}how often "
                    <em>"would"</em>
                    " it win under the current model?\u{201D} If a film wins more than the model expects, its strength goes up; if it wins less, its strength goes down. The algorithm keeps adjusting until these two quantities balance out for every film simultaneously."
                </p>
                <p>
                    "After each iteration the strengths are renormalized so their geometric mean equals\u{00A0}1 (i.e.\u{00A0}we subtract the mean of log\u{00A0}\u{03B2} values). This prevents the numbers from drifting to infinity while preserving all the ratios that matter. Films with zero wins are pinned to a near-zero score\u{00A0}(10"
                    <sup>"\u{2212}6"</sup>
                    "). Convergence is declared when the maximum relative change across all \u{03B2} values drops below\u{00A0}10"
                    <sup>"\u{2212}8"</sup> "."
                </p>

                <h4>"Display Score"</h4>
                <p class="math">
                    "score\u{00A0}=\u{00A0}round(500\u{00A0}\u{00D7}\u{00A0}log"
                    <sub>"2"</sub> "(1\u{00A0}+\u{00A0}\u{03B2}))"
                </p>
                <p>
                    <b>"Intuition:"</b>
                    " raw \u{03B2} values can span many orders of magnitude, making them hard to compare at a glance. The logarithm compresses this range into a human-friendly scale. The +1 ensures a film with \u{03B2}\u{00A0}=\u{00A0}0 maps to score\u{00A0}0, and the 500 multiplier spreads the values into a comfortable range (roughly 0\u{2013}5000+). Every doubling of a film\u{2019}s strength adds 500\u{00A0}points."
                </p>

                <h4>"Smart Pair Selection: D-Optimal Design"</h4>
                <p>
                    "Pairs are not presented randomly. The system uses "
                    <a href="https://en.wikipedia.org/wiki/Optimal_design#D-optimality" target="_blank">"D-optimal experimental design"</a>
                    " to pick the most informative matchup for each vote."
                </p>
                <p>
                    "First, the server builds the "
                    <a href="https://en.wikipedia.org/wiki/Fisher_information" target="_blank">"Fisher information matrix"</a>
                    "\u{00A0}" <b>"F"</b>
                    ". For each pair (" <var>"i"</var> ",\u{00A0}" <var>"j"</var>
                    ") that has been compared, the information contributed is:"
                </p>
                <p class="math">
                    "I" <sub><var>"ij"</var></sub>
                    "\u{00A0}=\u{00A0}"
                    <var>"n" <sub>"ij"</sub></var>
                    "\u{00A0}\u{22C5}\u{00A0}"
                    <var>"p" <sub>"ij"</sub></var>
                    "\u{00A0}\u{22C5}\u{00A0}(1\u{00A0}\u{2212}\u{00A0}"
                    <var>"p" <sub>"ij"</sub></var>
                    ")\u{2003}where\u{2003}"
                    <var>"p" <sub>"ij"</sub></var>
                    "\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
                    "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
                    "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
                </p>
                <p>
                    <b>"Intuition:"</b>
                    " " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
                    ") is the variance of a Bernoulli trial. It peaks at "
                    <var>"p"</var> "\u{00A0}=\u{00A0}\u{00BD} (evenly matched films) and vanishes when one film always wins. A matchup between closely-ranked films tells you more than a blowout. Multiplying by the number of comparisons "
                    <var>"n" <sub>"ij"</sub></var>
                    " accounts for how much data we already have."
                </p>
                <p>
                    "These contributions fill a symmetric matrix "
                    <b>"F"</b>
                    " (with I" <sub><var>"ij"</var></sub>
                    " added to the diagonal entries " <b>"F"</b> "[" <var>"i"</var> "," <var>"i"</var>
                    "] and " <b>"F"</b> "[" <var>"j"</var> "," <var>"j"</var>
                    "], and subtracted from the off-diagonal " <b>"F"</b> "[" <var>"i"</var> "," <var>"j"</var>
                    "]). The diagonal is regularized by adding a small prior\u{00A0}(0.25) for numerical stability. Then the matrix is inverted via Gauss\u{2013}Jordan elimination with partial pivoting to obtain "
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "."
                </p>
                <p>"For each candidate pair (a, b) the D-optimal score is:"</p>
                <p class="math">
                    "d" <sub><var>"ab"</var></sub>
                    "\u{00A0}=\u{00A0}"
                    <var>"p" <sub>"ab"</sub></var>
                    "(1\u{00A0}\u{2212}\u{00A0}"
                    <var>"p" <sub>"ab"</sub></var>
                    ")\u{00A0}\u{22C5}\u{00A0}("
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
                    "]\u{00A0}+\u{00A0}"
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
                    "]\u{00A0}\u{2212}\u{00A0}2"
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var> "])"
                </p>
                <p>
                    <b>"Intuition:"</b> " "
                    <b>"F"</b> <sup>"\u{2212}1"</sup>
                    " encodes how uncertain we are about each film\u{2019}s strength. The expression "
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
                    "]\u{00A0}+\u{00A0}"
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
                    "]\u{00A0}\u{2212}\u{00A0}2"
                    <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var>
                    "] is the variance of the difference \u{03B2}" <sub><var>"a"</var></sub>
                    "\u{00A0}\u{2212}\u{00A0}\u{03B2}" <sub><var>"b"</var></sub>
                    ". Multiplying by " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
                    ") weights this by how much a single new comparison can actually reduce that uncertainty. So the system picks the matchup where one more vote would shrink the overall uncertainty the most."
                </p>
                <p>
                    "These D-optimal scores are converted to sampling probabilities with a softmax (inverse temperature\u{00A0}=\u{00A0}5), so the best pair is strongly favored but not deterministically chosen\u{00A0}\u{2014} preserving some exploration."
                </p>

                <h4>"Leaderboard Eligibility"</h4>
                <p>"A film appears on the leaderboard once it has at least 10 comparisons from at least 2 different voters. All votes from all users are aggregated into one global ranking."</p>
            </div>
        </div>

        <StatsSection />

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

        <ContributionsSection />
        <SuggestSection />
        <MatrixSections />
    }
}

/// Mini Bradley-Terry simulation: shows votes arriving, filling a win matrix,
/// and β scores converging via the MM algorithm.
#[component]
fn BtSimulation() -> impl IntoView {
    use wasm_bindgen::JsCast;

    const N: usize = 4;
    const LABELS: [&str; N] = ["A", "B", "C", "D"];
    // Hidden "true" strengths used to generate votes probabilistically
    const TRUE_BETA: [f64; N] = [4.0, 2.0, 1.0, 0.5];

    // wins[i][j] = number of times i beat j
    let wins: RwSignal<[[u32; N]; N]> = RwSignal::new([[0; N]; N]);
    // Current estimated β values
    let betas: RwSignal<[f64; N]> = RwSignal::new([1.0; N]);
    // Step counter for display
    let step: RwSignal<u32> = RwSignal::new(0);
    // Last vote highlight (winner, loser)
    let last_vote: RwSignal<Option<(usize, usize)>> = RwSignal::new(None);
    // Timer handle
    let timer_handle: RwSignal<Option<i32>> = RwSignal::new(None);
    let playing = RwSignal::new(true);

    // Run MM algorithm on current wins, return new betas
    let run_mm = move |w: [[u32; N]; N]| -> [f64; N] {
        let mut scores = [1.0_f64; N];
        // Check which films have wins
        let has_wins: Vec<bool> = (0..N).map(|i| (0..N).any(|j| w[i][j] > 0)).collect();
        let has_comparisons: Vec<bool> = (0..N)
            .map(|i| (0..N).any(|j| w[i][j] + w[j][i] > 0))
            .collect();

        for i in 0..N {
            if has_comparisons[i] && !has_wins[i] {
                scores[i] = 1e-6;
            }
        }

        for _ in 0..200 {
            let old = scores;
            let mut max_rel = 0.0_f64;

            for i in 0..N {
                if !has_wins[i] {
                    continue;
                }
                let w_i: f64 = (0..N).map(|j| w[i][j] as f64).sum();
                let denom: f64 = (0..N)
                    .filter(|&j| j != i)
                    .filter_map(|j| {
                        let n_ij = (w[i][j] + w[j][i]) as f64;
                        if n_ij > 0.0 {
                            Some(n_ij / (old[i] + old[j]))
                        } else {
                            None
                        }
                    })
                    .sum();
                if denom > 0.0 {
                    scores[i] = w_i / denom;
                    let rel = (scores[i] - old[i]).abs() / old[i];
                    max_rel = max_rel.max(rel);
                }
            }

            // Normalize: geometric mean of ranked films = 1
            let ranked: Vec<usize> = (0..N).filter(|&i| has_wins[i]).collect();
            if !ranked.is_empty() {
                let log_mean =
                    ranked.iter().map(|&i| scores[i].ln()).sum::<f64>() / ranked.len() as f64;
                let scale = (-log_mean).exp();
                for &i in &ranked {
                    scores[i] *= scale;
                }
            }

            if max_rel < 1e-8 {
                break;
            }
        }
        scores
    };

    // Add one random vote based on true strengths
    let add_vote = move || {
        // Pick a random pair
        let window = web_sys::window().unwrap();
        let crypto = window.crypto().unwrap();
        let mut buf = [0u8; 4];
        crypto.get_random_values_with_u8_array(&mut buf).ok();
        let r = u32::from_le_bytes(buf);

        // Pick pair (i, j) where i < j
        let pair_count = N * (N - 1) / 2;
        let pair_idx = (r as usize) % pair_count;
        let mut idx = 0;
        let mut pi = 0;
        let mut pj = 1;
        for i in 0..N {
            for j in (i + 1)..N {
                if idx == pair_idx {
                    pi = i;
                    pj = j;
                }
                idx += 1;
            }
        }

        // Determine winner based on true strengths
        let p_i = TRUE_BETA[pi] / (TRUE_BETA[pi] + TRUE_BETA[pj]);
        let mut buf2 = [0u8; 4];
        crypto.get_random_values_with_u8_array(&mut buf2).ok();
        let r2 = u32::from_le_bytes(buf2) as f64 / u32::MAX as f64;
        let (winner, loser) = if r2 < p_i { (pi, pj) } else { (pj, pi) };

        wins.update(|w| w[winner][loser] += 1);
        let new_betas = run_mm(wins.get_untracked());
        betas.set(new_betas);
        step.update(|s| *s += 1);
        last_vote.set(Some((winner, loser)));
    };

    let start_timer = move || {
        let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            add_vote();
        });
        let window = web_sys::window().unwrap();
        let id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                600,
            )
            .unwrap();
        cb.forget();
        timer_handle.set(Some(id));
    };

    let stop_timer = move || {
        if let Some(id) = timer_handle.get_untracked() {
            web_sys::window().unwrap().clear_interval_with_handle(id);
            timer_handle.set(None);
        }
    };

    let reset = move |_| {
        stop_timer();
        wins.set([[0; N]; N]);
        betas.set([1.0; N]);
        step.set(0);
        last_vote.set(None);
        if playing.get_untracked() {
            start_timer();
        }
    };

    let toggle_play = move |_| {
        if playing.get_untracked() {
            stop_timer();
            playing.set(false);
        } else {
            playing.set(true);
            start_timer();
        }
    };

    let step_one = move |_| {
        add_vote();
    };

    // Auto-start
    start_timer();

    // Draw on canvas whenever state changes
    Effect::new(move || {
        let w = wins.get();
        let b = betas.get();
        let _s = step.get();
        let lv = last_vote.get();

        let doc = web_sys::window().unwrap().document().unwrap();
        let Some(canvas) = doc.get_element_by_id("bt-sim-canvas") else {
            return;
        };
        let canvas: web_sys::HtmlCanvasElement = canvas.unchecked_into();
        let dpr = web_sys::window().unwrap().device_pixel_ratio();
        let css_w = 340.0;
        let css_h = 170.0;
        canvas.set_width((css_w * dpr) as u32);
        canvas.set_height((css_h * dpr) as u32);

        let ctx: web_sys::CanvasRenderingContext2d =
            canvas.get_context("2d").unwrap().unwrap().unchecked_into();
        ctx.scale(dpr, dpr).ok();
        ctx.clear_rect(0.0, 0.0, css_w, css_h);

        // === Left half: Win matrix ===
        let mat_x = 20.0;
        let mat_y = 22.0;
        let cell = 26.0;
        let hdr = 16.0; // space for header labels

        // Header label
        ctx.set_font("bold 11px system-ui, sans-serif");
        ctx.set_fill_style_str("#2d1f3d");
        ctx.set_text_align("center");

        // Column headers
        for j in 0..N {
            let x = mat_x + hdr + j as f64 * cell + cell / 2.0;
            ctx.fill_text(LABELS[j], x, mat_y - 4.0).ok();
        }
        // Row headers
        ctx.set_text_align("right");
        for i in 0..N {
            let y = mat_y + i as f64 * cell + cell / 2.0 + 4.0;
            ctx.fill_text(LABELS[i], mat_x + hdr - 4.0, y).ok();
        }

        // Cells
        for i in 0..N {
            for j in 0..N {
                let x = mat_x + hdr + j as f64 * cell;
                let y = mat_y + i as f64 * cell;

                if i == j {
                    // Diagonal: dark
                    ctx.set_fill_style_str("#d4c4dd");
                    ctx.fill_rect(x, y, cell, cell);
                } else {
                    // Highlight last vote cell
                    let is_last = lv.map_or(false, |(wi, lo)| wi == i && lo == j);
                    if is_last {
                        ctx.set_fill_style_str("#f5b43666");
                    } else {
                        ctx.set_fill_style_str("#f5f0f7");
                    }
                    ctx.fill_rect(x, y, cell, cell);

                    // Win count
                    let count = w[i][j];
                    if count > 0 {
                        ctx.set_font("bold 11px system-ui, sans-serif");
                        ctx.set_text_align("center");
                        ctx.set_fill_style_str("#742a85");
                        ctx.fill_text(&count.to_string(), x + cell / 2.0, y + cell / 2.0 + 4.0)
                            .ok();
                    }
                }

                // Cell border
                ctx.set_stroke_style_str("#d4c4dd");
                ctx.set_line_width(0.5);
                ctx.stroke_rect(x, y, cell, cell);
            }
        }

        // "wins >" label
        ctx.set_font("9px system-ui, sans-serif");
        ctx.set_fill_style_str("#7a6888");
        ctx.set_text_align("left");
        ctx.fill_text(
            "row beats col \u{2192}",
            mat_x,
            mat_y + N as f64 * cell + 12.0,
        )
        .ok();

        // === Right half: β bars ===
        let bar_x = mat_x + hdr + N as f64 * cell + 30.0;
        let bar_area_w = css_w - bar_x - 10.0;
        let bar_h = 16.0;
        let bar_gap = 6.0;
        let bars_top = mat_y;

        // Find max display score for scaling
        let display_scores: Vec<f64> = b.iter().map(|&s| (1.0 + s).log2() * 500.0).collect();
        let max_score = display_scores.iter().cloned().fold(1.0_f64, f64::max);

        ctx.set_font("bold 11px system-ui, sans-serif");
        for i in 0..N {
            let y = bars_top + i as f64 * (bar_h + bar_gap);
            let score = display_scores[i];
            let bar_w = (score / max_score) * (bar_area_w - 32.0);
            let bar_w = bar_w.max(0.0);

            // Label
            ctx.set_text_align("right");
            ctx.set_fill_style_str("#2d1f3d");
            ctx.fill_text(LABELS[i], bar_x + 10.0, y + bar_h / 2.0 + 4.0)
                .ok();

            // Bar
            ctx.set_fill_style_str("#742a85");
            ctx.fill_rect(bar_x + 14.0, y, bar_w, bar_h);

            // Score value
            ctx.set_text_align("left");
            ctx.set_font("10px system-ui, sans-serif");
            ctx.set_fill_style_str("#7a6888");
            ctx.fill_text(
                &format!("{:.0}", score),
                bar_x + 14.0 + bar_w + 3.0,
                y + bar_h / 2.0 + 3.5,
            )
            .ok();
            ctx.set_font("bold 11px system-ui, sans-serif");
        }

        // β header
        ctx.set_font("bold 11px system-ui, sans-serif");
        ctx.set_text_align("left");
        ctx.set_fill_style_str("#2d1f3d");
        ctx.fill_text("Score", bar_x, bars_top - 6.0).ok();
    });

    view! {
        <div class="bt-demo">
            <div class="bt-sim-header">
                {move || format!("{} votes", step.get())}
            </div>
            <canvas id="bt-sim-canvas"
                style="width:340px;height:170px;display:block;margin:0 auto;" />
            <div class="bt-sim-controls">
                <button on:click=toggle_play class="bt-sim-btn">
                    {move || if playing.get() { "\u{23F8} Pause" } else { "\u{25B6} Play" }}
                </button>
                <button on:click=step_one class="bt-sim-btn">
                    "+1 vote"
                </button>
                <button on:click=reset class="bt-sim-btn">
                    "Reset"
                </button>
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

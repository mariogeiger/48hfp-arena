//! Behind the ranking: how many votes and voters, who voted most, how every
//! pair fared against the model, and the way to how it is computed.

use crate::components::icons::{Icon, icon};
use crate::i18n::{fill, say, t};
use crate::pages::matrices::GlobalMatrix;
use crate::state::{AppState, Page, SiteConfig};
use leptos::prelude::*;

#[component]
pub fn Backstage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let source_url = SiteConfig::get().source_url;
    let stats = state.stats;
    let tile = move |value: fn(&crate::state::Stats) -> u64, label: String| {
        view! {
            <StatTile value=Signal::derive(move || stats.with(|s| s.as_ref().map(value).unwrap_or(0))) label=label />
        }
    };
    view! {
        <section class="backstage">
            <h2 class="section-title">{t().backstage_title}</h2>
            <div class="stat-tiles">
                {tile(|s| s.total_votes, t().stats_votes.to_string())}
                {tile(|s| s.active_users, t().stats_voters.to_string())}
                {tile(|s| s.films_with_votes, say(t().stats_voted_on, &[]))}
            </div>
            <Contributions />
            <details class="fold">
                <summary>
                    {icon(Icon::Grid)}
                    <span>{t().matrix_global}</span>
                    {icon(Icon::ChevronDown)}
                </summary>
                <p class="fine-print">{t().matrix_global_hint}</p>
                <GlobalMatrix />
            </details>
            <div class="backstage-links">
                <button class="link-btn" on:click=move |_| state.navigate(Page::About)>
                    {icon(Icon::Help)}
                    <span>{t().how_it_works_link}</span>
                </button>
                <a class="link-btn" href="/api/leaderboard.csv" target="_blank">
                    {icon(Icon::Download)}
                    <span>"CSV"</span>
                </a>
                {(!source_url.is_empty()).then(|| view! {
                    <a class="link-btn" href=source_url target="_blank" rel="noopener">
                        {icon(Icon::GitHub)}
                        <span>{t().source_code}</span>
                    </a>
                })}
            </div>
        </section>
    }
}

/// A number that jumps when it changes.
#[component]
fn StatTile(value: Signal<u64>, label: String) -> impl IntoView {
    let bump = RwSignal::new(false);
    let last = StoredValue::new(None::<u64>);
    Effect::new(move || {
        let now = value.get();
        if last.get_value().is_some_and(|before| before != now) {
            bump.set(true);
            crate::timeout::Timeout::new(450, move || bump.set(false)).forget();
        }
        last.set_value(Some(now));
    });
    view! {
        <div class="stat-tile">
            <b class:changed=move || bump.get()>{move || value.get()}</b>
            <span>{label}</span>
        </div>
    }
}

/// The voters who voted most, the viewer among them.
#[component]
fn Contributions() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let contributions = state.contributions;
    move || {
        let all = contributions.get();
        let max = all.first().map(|c| c.votes).unwrap_or(1).max(1);
        // The first ten, and the viewer wherever they stand.
        let shown: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(i, c)| *i < 10 || c.is_you)
            .map(|(_, c)| c.clone())
            .collect();
        (!shown.is_empty()).then(|| {
            view! {
                <h3 class="sub-title">{t().contrib_heading}</h3>
                <div class="contrib">
                    {shown
                        .into_iter()
                        .map(|c| {
                            let label = if c.is_you {
                                t().contrib_you.to_string()
                            } else {
                                fill(t().contrib_voter, &[("n", &c.rank)])
                            };
                            let width = format!("{:.1}%", 100.0 * c.votes as f64 / max as f64);
                            view! {
                                <div class="contrib-row" class:you=c.is_you>
                                    <span class="contrib-label">{label}</span>
                                    <span class="contrib-track"><i style:width=width></i></span>
                                    <span class="contrib-votes">{c.votes}</span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
        })
    }
}

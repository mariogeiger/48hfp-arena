//! An item's history: each revision with what it changed, its author and
//! date, and the way back to any earlier version.

use crate::components::form::field_label;
use crate::i18n::{fill, lang, t};
use crate::state::{AppState, SiteConfig};
use crate::wiki::History;
use leptos::prelude::*;
use versus_shared::{Change, FilmFields, HistoryEntry};

/// One row of the history: a revision, or the original version (`rev` 0).
struct Row {
    rev: u64,
    heading: String,
    byline: Option<String>,
    changes: Vec<(String, String, String)>,
    /// The fields this row leaves the item with, which it can restore.
    fields: Option<FilmFields>,
}

/// Each field `after` changed from `before`, as (name, before, after).
fn changes(before: Option<&FilmFields>, after: &FilmFields) -> Vec<(String, String, String)> {
    let names = SiteConfig::get().fields;
    let empty = FilmFields::default();
    let before = before.unwrap_or(&empty);
    let slots = [
        (&names.title, t().field_title, &before.title, &after.title),
        (&names.team, t().field_team, &before.team, &after.team),
        (&names.city, t().field_city, &before.city, &after.city),
        (
            &names.poster_url,
            t().field_poster,
            &before.poster_url,
            &after.poster_url,
        ),
        (
            &names.video_url,
            t().field_video,
            &before.video_url,
            &after.video_url,
        ),
        (
            &names.synopsis,
            t().field_synopsis,
            &before.synopsis,
            &after.synopsis,
        ),
    ];
    slots
        .into_iter()
        .filter(|(_, _, b, a)| b != a)
        .filter_map(|(name, words, b, a)| Some((field_label(name, words)?, b.clone(), a.clone())))
        .collect()
}

fn date(time: u64) -> String {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(time as f64 * 1000.0))
        .to_locale_string(lang().tag(), &wasm_bindgen::JsValue::UNDEFINED)
        .into()
}

fn byline(entry: &HistoryEntry) -> String {
    let who = if entry.is_you {
        t().author_you.to_string()
    } else {
        fill(t().author_anon, &[("tag", &entry.author)])
    };
    format!("{who} \u{00B7} {}", date(entry.time))
}

/// The rows newest first, and the item's current fields.
fn rows(history: History) -> (Vec<Row>, Option<FilmFields>) {
    let mut current = history.original.clone();
    let mut rows: Vec<Row> = history
        .original
        .map(|original| Row {
            rev: 0,
            heading: t().history_original.into(),
            byline: None,
            changes: Vec::new(),
            fields: Some(original),
        })
        .into_iter()
        .collect();
    for entry in history.revisions.into_iter().rev() {
        let (heading, fields) = match &entry.change {
            Change::Edit { fields, revert_of } => {
                let heading = match revert_of {
                    None => t().history_edit.to_string(),
                    Some(0) => t().history_revert_original.to_string(),
                    Some(n) => fill(t().history_revert, &[("n", n)]),
                };
                (heading, Some(fields.clone()))
            }
            Change::Propose { fields } => (t().history_propose.to_string(), Some(fields.clone())),
            Change::Review { approve: true } => (t().history_approve.to_string(), None),
            Change::Review { approve: false } => (t().history_reject.to_string(), None),
        };
        let changed = fields
            .as_ref()
            .map(|f| changes(current.as_ref(), f))
            .unwrap_or_default();
        if fields.is_some() {
            current = fields.clone();
        }
        rows.push(Row {
            rev: entry.rev,
            heading: format!("{heading} \u{00B7} #{}", entry.rev),
            byline: Some(byline(&entry)),
            changes: changed,
            fields,
        });
    }
    rows.reverse();
    (rows, current)
}

#[component]
pub fn ItemHistory(film: usize) -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let editable = SiteConfig::get().editable;
    let history = RwSignal::new(None::<History>);
    let error = RwSignal::new(None::<String>);

    let load = {
        let state = state.clone();
        move || {
            let state = state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                history.try_set(crate::wiki::history(&state, film).await);
            });
        }
    };
    load();

    let restore = {
        let load = load.clone();
        move |rev: u64| {
            let state = state.clone();
            let load = load.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::wiki::revert(&state, film, rev).await {
                    Ok(()) => {
                        state.add_toast(t().saved.into());
                        error.try_set(None);
                        load();
                    }
                    Err(code) => {
                        error.try_set(Some(t().error(&code).into()));
                    }
                }
            });
        }
    };

    move || {
        let history = history.get()?;
        let (rows, current) = rows(history);
        if rows.is_empty() {
            return Some(view! { <p class="sheet-hint">{t().history_empty}</p> }.into_any());
        }
        let restore = restore.clone();
        Some(
            view! {
                {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
                <ol class="history">
                    {rows.into_iter().scan(false, |marked, row| {
                        // The newest row with the current fields is the current
                        // version; older rows with the same fields restore nothing.
                        let same = row.fields.is_some() && row.fields == current;
                        let is_current = same && !std::mem::replace(marked, *marked || same);
                        let restorable = editable && row.fields.is_some() && !same;
                        Some((row, is_current, restorable))
                    }).map(|(row, is_current, restorable)| {
                        let rev = row.rev;
                        let restore = restore.clone();
                        view! {
                            <li class="history-row" class:history-current=is_current>
                                <div class="history-heading">
                                    <strong>{row.heading}</strong>
                                    {is_current.then(|| view! { <span class="badge">{t().history_current}</span> })}
                                </div>
                                {row.byline.map(|b| view! { <div class="history-byline">{b}</div> })}
                                <ul class="history-changes">
                                    {row.changes.into_iter().map(|(name, before, after)| view! {
                                        <li>
                                            <span class="history-field">{name}</span>
                                            {(!before.is_empty()).then(|| view! { <del>{before}</del> })}
                                            <ins>{after}</ins>
                                        </li>
                                    }).collect_view()}
                                </ul>
                                {restorable.then(|| view! {
                                    <button class="history-restore" on:click=move |_| restore(rev)>
                                        {t().history_restore}
                                    </button>
                                })}
                            </li>
                        }
                    }).collect_view()}
                </ol>
            }
            .into_any(),
        )
    }
}

//! The form that edits an item, or proposes a new one: one input per field
//! the site has, for the poster an image to upload or a link, and for the
//! video one row per version, in order.

use crate::components::poster::Poster;
use crate::i18n::{say, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use versus_shared::{Film, FilmFields, Video};
use wasm_bindgen::JsCast;

/// What the form calls a field: the site's name for it, the language's when
/// the site gives none, or `None` when the site lacks the field.
pub fn field_label(site: &Option<String>, words: &'static str) -> Option<String> {
    site.as_ref().map(|name| {
        if name.is_empty() {
            words.to_string()
        } else {
            name.clone()
        }
    })
}

#[component]
fn TextField(label: String, value: RwSignal<String>, #[prop(optional)] url: bool) -> impl IntoView {
    view! {
        <label class="form-field">
            <span>{label}</span>
            <input
                type="text"
                inputmode=if url { "url" } else { "text" }
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </label>
    }
}

/// One version as the form edits it; `key` tells rows apart as they move.
#[derive(Clone, Copy)]
struct VersionRow {
    key: usize,
    label: RwSignal<String>,
    url: RwSignal<String>,
}

impl VersionRow {
    fn new(video: Video) -> Self {
        static NEXT_KEY: AtomicUsize = AtomicUsize::new(0);
        VersionRow {
            key: NEXT_KEY.fetch_add(1, Ordering::Relaxed),
            label: RwSignal::new(video.label),
            url: RwSignal::new(video.url),
        }
    }

    /// The rows of `videos`; with no version yet, one empty row stands
    /// ready for a link.
    fn all(videos: Vec<Video>) -> Vec<Self> {
        if videos.is_empty() {
            vec![Self::new(Video::default())]
        } else {
            videos.into_iter().map(Self::new).collect()
        }
    }
}

/// The versions in order, each a name and a link that may move up or go.
#[component]
fn VersionsField(label: String, rows: RwSignal<Vec<VersionRow>>) -> impl IntoView {
    let position = move |key: usize| rows.with(|rows| rows.iter().position(|r| r.key == key));
    view! {
        <div class="form-field">
            <span>{label}</span>
            {move || (rows.with(Vec::len) > 1).then(|| view! { <small class="form-hint">{t().form_versions_hint}</small> })}
            <For each=move || rows.get() key=|r| r.key let:r>
                <div class="version-row">
                    <input
                        type="text"
                        class="version-name"
                        placeholder=t().form_version_name
                        prop:value=move || r.label.get()
                        on:input=move |ev| r.label.set(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        inputmode="url"
                        class="version-url"
                        placeholder="https://\u{2026}"
                        prop:value=move || r.url.get()
                        on:input=move |ev| r.url.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        class="version-act"
                        title=t().form_move_up
                        aria-label=t().form_move_up
                        disabled=move || position(r.key).is_none_or(|i| i == 0)
                        on:click=move |_| {
                            if let Some(i) = position(r.key).filter(|&i| i > 0) {
                                rows.update(|rows| rows.swap(i - 1, i));
                            }
                        }
                    >"\u{2191}"</button>
                    <button
                        type="button"
                        class="version-act"
                        title=t().form_remove_version
                        aria-label=t().form_remove_version
                        on:click=move |_| rows.update(|rows| rows.retain(|x| x.key != r.key))
                    >"\u{00D7}"</button>
                </div>
            </For>
            <button
                type="button"
                class="version-add"
                on:click=move |_| rows.update(|rows| rows.push(VersionRow::new(Video::default())))
            >{t().form_add_version}</button>
        </div>
    }
}

/// The versions the rows hold, those left entirely blank aside.
fn versions_of(rows: &[VersionRow]) -> Vec<Video> {
    rows.iter()
        .map(|r| Video {
            label: r.label.get_untracked(),
            url: r.url.get_untracked(),
        })
        .filter(|v| !(v.label.trim().is_empty() && v.url.trim().is_empty()))
        .collect()
}

/// Edits `film`, or proposes a new item when it is `None`. `on_done` gets the
/// item's id once saved, or `None` when cancelled. With `focus_video`, the
/// first version's link awaits one, in view.
#[component]
pub fn ItemForm(
    film: Option<Film>,
    on_done: Callback<Option<usize>>,
    #[prop(optional)] focus_video: bool,
) -> impl IntoView {
    let state = use_context::<AppState>().unwrap();
    let cfg = SiteConfig::get();
    let names = cfg.fields.clone();
    let initial = film.as_ref().map(Film::fields).unwrap_or_default();
    let id = film.map(|f| f.id);
    let preview_id = id.unwrap_or(0);

    let title = RwSignal::new(initial.title);
    let team = RwSignal::new(initial.team);
    let city = RwSignal::new(initial.city);
    let poster = RwSignal::new(initial.poster_url);
    let version_rows = RwSignal::new(VersionRow::all(initial.videos));
    let synopsis = RwSignal::new(initial.synopsis);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let uploading = RwSignal::new(false);

    let state_upload = state.clone();
    let on_file = move |ev: web_sys::Event| {
        let Some(file) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .and_then(|input| input.files())
            .and_then(|files| files.get(0))
        else {
            return;
        };
        let state = state_upload.clone();
        uploading.set(true);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match crate::wiki::upload_poster(&state, file).await {
                Ok(url) => {
                    poster.try_set(url);
                }
                Err(code) => {
                    error.try_set(Some(t().error(&code).into()));
                }
            }
            uploading.try_set(false);
        });
    };

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() || uploading.get_untracked() {
            return;
        }
        let fields = FilmFields {
            title: title.get_untracked(),
            team: team.get_untracked(),
            city: city.get_untracked(),
            poster_url: poster.get_untracked(),
            videos: version_rows.with_untracked(|rows| versions_of(rows)),
            synopsis: synopsis.get_untracked(),
        };
        if fields.title.trim().is_empty() {
            error.set(Some(t().error_title_required.into()));
            return;
        }
        let state = state.clone();
        busy.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let result = match id {
                Some(id) => crate::wiki::edit(&state, id, fields).await.map(|()| id),
                None => crate::wiki::propose(&state, fields).await,
            };
            busy.try_set(false);
            match result {
                Ok(saved) => {
                    state.add_toast(
                        if id.is_some() {
                            t().saved
                        } else {
                            t().proposed
                        }
                        .into(),
                    );
                    on_done.try_run(Some(saved));
                }
                Err(code) => error.set(Some(t().error(&code).into())),
            }
        });
    };

    let text = |name: &Option<String>, words: &'static str, value: RwSignal<String>, url: bool| {
        field_label(name, words)
            .map(|label| view! { <TextField label=label value=value url=url /> })
    };

    let form = NodeRef::<leptos::html::Form>::new();
    if focus_video {
        form.on_load(|form| {
            if let Ok(Some(link)) = form.query_selector(".version-url")
                && let Ok(link) = link.dyn_into::<web_sys::HtmlElement>()
            {
                link.scroll_into_view_with_bool(false);
                let _ = link.focus();
            }
        });
    }

    view! {
        <form class="item-form" node_ref=form on:submit=submit>
            {text(&names.title, t().field_title, title, false)}
            {text(&names.team, t().field_team, team, false)}
            {text(&names.city, t().field_city, city, false)}
            {field_label(&names.poster_url, t().field_poster).map(|label| view! {
                <div class="form-field">
                    <span>{label}</span>
                    <div class="form-poster">
                        {move || view! { <div class="form-poster-preview"><Poster url=poster.get() title=title.get() id=preview_id /></div> }}
                        <div class="form-poster-inputs">
                            <label class="upload-button">
                                {move || if uploading.get() { t().form_uploading } else { t().form_poster_upload }}
                                <input type="file" accept="image/*" on:change=on_file />
                            </label>
                            <input
                                type="text"
                                inputmode="url"
                                placeholder=t().form_poster_link
                                prop:value=move || poster.get()
                                on:input=move |ev| poster.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                </div>
            })}
            {field_label(&names.videos, t().field_video).map(|label| view! {
                <VersionsField label=label rows=version_rows />
            })}
            {field_label(&names.synopsis, t().field_synopsis).map(|label| view! {
                <label class="form-field">
                    <span>{label}</span>
                    <textarea
                        rows="5"
                        maxlength="2000"
                        prop:value=move || synopsis.get()
                        on:input=move |ev| synopsis.set(event_target_value(&ev))
                    ></textarea>
                </label>
            })}
            {move || error.get().map(|e| view! { <p class="form-error">{e}</p> })}
            <div class="sheet-actions">
                <button type="submit" class="primary" disabled=move || busy.get() || uploading.get()>
                    {if id.is_some() { t().form_save.to_string() } else { say(t().form_propose, &[]) }}
                </button>
                <button type="button" on:click=move |_| on_done.run(id)>{t().form_cancel}</button>
            </div>
        </form>
    }
}

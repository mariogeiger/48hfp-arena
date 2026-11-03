//! The form that edits an item, or proposes a new one: one input per field
//! the site has, and for the poster an image to upload or a link.

use crate::components::poster::Poster;
use crate::i18n::{say, t};
use crate::state::{AppState, SiteConfig};
use leptos::prelude::*;
use versus_shared::{Film, FilmFields};
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
                type=if url { "url" } else { "text" }
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </label>
    }
}

/// Edits `film`, or proposes a new item when it is `None`. `on_done` gets the
/// item's id once saved, or `None` when cancelled.
#[component]
pub fn ItemForm(film: Option<Film>, on_done: Callback<Option<usize>>) -> impl IntoView {
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
    let video = RwSignal::new(initial.video_url);
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
            video_url: video.get_untracked(),
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

    view! {
        <form class="item-form" on:submit=submit>
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
                                type="url"
                                placeholder=t().form_poster_link
                                prop:value=move || poster.get()
                                on:input=move |ev| poster.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                </div>
            })}
            {text(&names.video_url, t().field_video, video, true)}
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

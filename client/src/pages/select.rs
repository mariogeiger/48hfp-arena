use crate::api;
use crate::components::poster::Poster;
use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn SelectPage() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    let state_ds = state.clone();
    let pending_save: StoredValue<
        Option<send_wrapper::SendWrapper<gloo_timers::callback::Timeout>>,
    > = StoredValue::new(None);
    let debounce_save = move || {
        // Cancel any pending save before scheduling a new one
        pending_save.set_value(None); // dropping the old Timeout cancels it
        let s = state_ds.clone();
        pending_save.set_value(Some(send_wrapper::SendWrapper::new(
            gloo_timers::callback::Timeout::new(500, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    api::save_selection(&s).await;
                });
            }),
        )));
    };

    let visible_films = Memo::new(move |_| {
        let films = state.films.get();
        let q = state.search_query.get().to_lowercase();
        if q.is_empty() {
            films
        } else {
            films
                .into_iter()
                .filter(|f| {
                    f.title.to_lowercase().contains(&q)
                        || f.team.to_lowercase().contains(&q)
                        || f.city.to_lowercase().contains(&q)
                })
                .collect()
        }
    });

    let selected_count = Memo::new(move |_| state.selected_ids.get().len());

    let state_sa = state.clone();
    let state_da = state.clone();

    view! {
        <div class="select-header">
            <p>"Select the films you remember, then compare them head-to-head."</p>
            <div class="search-wrapper">
                <input
                    class="search-box"
                    type="text"
                    placeholder="Search films, teams, cities..."
                    on:input=move |ev| {
                        state.search_query.set(event_target_value(&ev));
                    }
                    prop:value=move || state.search_query.get()
                />
                <button
                    class="search-clear"
                    on:click=move |_| state.search_query.set(String::new())
                >
                    "\u{00D7}"
                </button>
            </div>
            <div class="select-actions">
                <button on:click={
                    let debounce_save = debounce_save.clone();
                    move |_| {
                        let q = state_sa.search_query.get().to_lowercase();
                        let films = state_sa.films.get();
                        state_sa.selected_ids.update(|ids| {
                            for f in &films {
                                if q.is_empty()
                                    || f.title.to_lowercase().contains(&q)
                                    || f.team.to_lowercase().contains(&q)
                                    || f.city.to_lowercase().contains(&q)
                                {
                                    ids.insert(f.id);
                                }
                            }
                        });
                        debounce_save();
                    }
                }>"Select All"</button>
                <button on:click={
                    let debounce_save = debounce_save.clone();
                    move |_| {
                        state_da.selected_ids.set(Default::default());
                        debounce_save();
                    }
                }>"Deselect All"</button>
            </div>
        </div>
        <div class="film-list" id="film-list">
            {move || {
                let films = visible_films.get();
                films.into_iter().map(|film| {
                    let film_id = film.id;
                    let poster_url = film.poster_url.clone();
                    let title = film.title.clone();
                    let meta = format!(
                        "{}{}",
                        film.team,
                        if film.city.is_empty() {
                            String::new()
                        } else {
                            format!(" \u{00B7} {}", film.city)
                        }
                    );
                    let debounce_save = debounce_save.clone();
                    view! {
                        <div
                            class=move || {
                                if state.selected_ids.get().contains(&film_id) {
                                    "film-item selected"
                                } else {
                                    "film-item"
                                }
                            }
                            on:click=move |_| {
                                state.selected_ids.update(|ids| {
                                    if ids.contains(&film_id) {
                                        ids.remove(&film_id);
                                    } else {
                                        ids.insert(film_id);
                                    }
                                });
                                debounce_save();
                            }
                        >
                            <div class="film-check"></div>
                            <Poster url=poster_url />
                            <div class="film-info">
                                <div class="film-title">{title}</div>
                                <div class="film-meta">{meta}</div>
                            </div>
                        </div>
                    }
                }).collect_view()
            }}
        </div>
        <div
            class=move || {
                if selected_count.get() >= 2 {
                    "selection-status has-enough"
                } else {
                    "selection-status"
                }
            }
        >
            {move || {
                let n = selected_count.get();
                if n < 2 {
                    "Select at least 2 films to compare".to_string()
                } else {
                    format!("{} films selected", n)
                }
            }}
        </div>
    }
}

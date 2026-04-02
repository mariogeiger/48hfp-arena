use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    static BROKEN: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn is_broken(url: &str) -> bool {
    BROKEN.with(|b| b.borrow().contains(url))
}

fn mark_broken(url: &str) {
    BROKEN.with(|b| b.borrow_mut().insert(url.to_string()));
}

#[component]
pub fn Poster(url: String) -> impl IntoView {
    if url.is_empty() || is_broken(&url) {
        view! { <div class="poster-ph" inner_html="&#127916;" /> }.into_any()
    } else {
        let url_for_err = url.clone();
        let show_placeholder = RwSignal::new(false);
        view! {
            <img
                class="poster"
                src=url
                style:display=move || if show_placeholder.get() { "none" } else { "" }
                on:error=move |_| {
                    mark_broken(&url_for_err);
                    show_placeholder.set(true);
                }
            />
            {move || {
                if show_placeholder.get() {
                    Some(view! { <div class="poster-ph" inner_html="&#127916;" /> })
                } else {
                    None
                }
            }}
        }
        .into_any()
    }
}

/// Helper to generate poster HTML string (for innerHTML contexts like canvas tooltips)
pub fn poster_html(url: &str) -> String {
    if url.is_empty() || is_broken(url) {
        r#"<div class="poster-ph">&#127916;</div>"#.to_string()
    } else {
        format!(
            r#"<img class="poster" src="{}">"#,
            crate::api::html_escape(url)
        )
    }
}

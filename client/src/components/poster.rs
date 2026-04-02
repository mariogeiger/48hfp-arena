use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn Poster(url: String) -> impl IntoView {
    if url.is_empty() {
        view! { <div class="poster-ph" inner_html="&#127916;" /> }.into_any()
    } else {
        view! {
            <img
                class="poster"
                src=url
                on:error=move |ev| {
                    // Replace broken img with emoji placeholder
                    if let Some(target) = ev.target()
                        && let Ok(img) = target.dyn_into::<web_sys::Element>() {
                            let doc = web_sys::window().unwrap().document().unwrap();
                            let ph = doc.create_element("div").unwrap();
                            ph.set_class_name("poster-ph");
                            ph.set_inner_html("&#127916;");
                            if let Some(parent) = img.parent_node() {
                                let _ = parent.replace_child(&ph, &img);
                            }
                        }
                }
            />
        }
        .into_any()
    }
}

/// Helper to generate poster HTML string (for innerHTML contexts like canvas tooltips)
pub fn poster_html(url: &str) -> String {
    if url.is_empty() {
        r#"<div class="poster-ph">&#127916;</div>"#.to_string()
    } else {
        format!(
            r#"<img class="poster" src="{}">"#,
            crate::api::html_escape(url)
        )
    }
}

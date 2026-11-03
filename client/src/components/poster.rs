//! An item's poster, or in its place a tile of its own colour bearing its
//! initials, so that items without a poster still look apart.

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

/// The hue of the item `id`'s tile: the golden angle apart from the
/// previous item's, so that neighbours never look alike.
fn hue(id: usize) -> u32 {
    ((id as f64 * 137.508) % 360.0) as u32
}

/// The first letters of the first two words of `title`.
fn initials(title: &str) -> String {
    let letters: String = title
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    if letters.is_empty() {
        "\u{2605}".into()
    } else {
        letters
    }
}

#[component]
pub fn Poster(url: String, title: String, id: usize) -> impl IntoView {
    let tile = move || {
        view! {
            <div class="poster-ph" style=format!("--hue:{}", hue(id))>
                <span>{initials(&title)}</span>
            </div>
        }
    };
    if url.is_empty() || is_broken(&url) {
        tile().into_any()
    } else {
        let failed = RwSignal::new(false);
        let url_for_err = url.clone();
        view! {
            {move || {
                if failed.get() {
                    tile().into_any()
                } else {
                    let url_for_err = url_for_err.clone();
                    view! {
                        <img
                            class="poster"
                            src=url.clone()
                            alt=""
                            loading="lazy"
                            on:error=move |_| {
                                mark_broken(&url_for_err);
                                failed.set(true);
                            }
                        />
                    }
                    .into_any()
                }
            }}
        }
        .into_any()
    }
}

/// The same, as HTML, for the lists drawn as HTML (see the board): the tile
/// waits hidden behind the poster, which `give_way_to_tile` takes away if it
/// fails to load.
pub fn poster_html(url: &str, title: &str, id: usize) -> String {
    let tile = format!(
        r#"<div class="poster-ph" style="--hue:{}"><span>{}</span></div>"#,
        hue(id),
        crate::api::html_escape(&initials(title))
    );
    if url.is_empty() || is_broken(url) {
        tile
    } else {
        format!(
            r#"<div class="poster-stack">{tile}<img class="poster" src="{}" alt="" loading="lazy"></div>"#,
            crate::api::html_escape(url)
        )
    }
}

/// An `error` caught on its way to a poster drawn by `poster_html`: the
/// poster goes, its tile shows, and its address is not tried again.
pub fn give_way_to_tile(ev: &web_sys::Event) {
    use wasm_bindgen::JsCast;
    let Some(img) = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    if img.tag_name() == "IMG" && img.class_list().contains("poster") {
        if let Some(src) = img.get_attribute("src") {
            mark_broken(&src);
        }
        img.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_the_first_two_words() {
        assert_eq!(initials("La Dernière Demeure"), "LD");
        assert_eq!(initials("ça capote !"), "ÇC");
        assert_eq!(initials("RENATUS"), "R");
        assert_eq!(initials("…"), "\u{2605}");
    }
}

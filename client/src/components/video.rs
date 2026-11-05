//! An item's videos: pills choosing a version, over its player. A video plays
//! in place when the link is YouTube, YouCraft, Vimeo or a video file, else
//! it is offered as a link. Where an editable site's item has none, a button
//! of the player's size asks for one.

use crate::components::icons::{Icon, icon};
use crate::i18n::t;
use crate::state::SiteConfig;
use leptos::prelude::*;
use versus_shared::{Film, Video};

#[derive(Debug, PartialEq, Eq)]
pub enum Embed {
    /// A player page to show in a frame.
    Frame(String),
    /// A video file for the browser's own player.
    File(String),
    /// Anything else, opened in a new tab.
    Link(String),
}

/// How `url` is best shown.
pub fn embed(url: &str) -> Embed {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return Embed::Link(url.to_string());
    };
    let (location, query) = rest.split_once('?').unwrap_or((rest, ""));
    let location = location.split('#').next().unwrap_or(location);
    let (host, path) = location.split_once('/').unwrap_or((location, ""));
    let host = host.trim_start_matches("www.").trim_start_matches("m.");
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let param = |name: &str| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
    };

    let youtube = match (host, segments.as_slice()) {
        ("youtu.be", [id, ..]) => Some(*id),
        ("youtube.com" | "youtube-nocookie.com", ["watch"]) => param("v"),
        ("youtube.com" | "youtube-nocookie.com", ["shorts" | "embed" | "live", id, ..]) => {
            Some(*id)
        }
        _ => None,
    };
    if let Some(id) = youtube.filter(|id| is_video_id(id)) {
        return Embed::Frame(format!("https://www.youtube-nocookie.com/embed/{id}"));
    }

    let youcraft = match (host, segments.as_slice()) {
        ("youcraft.tube", ["watch"]) => param("v"),
        ("youcraft.tube", ["embed", id]) => Some(*id),
        _ => None,
    };
    if let Some(id) = youcraft.filter(|id| is_video_id(id)) {
        return Embed::Frame(format!("https://youcraft.tube/embed/{id}"));
    }

    let vimeo = match (host, segments.as_slice()) {
        ("vimeo.com", [id]) => Some((*id, None)),
        ("vimeo.com", [id, hash]) => Some((*id, Some(*hash))),
        ("player.vimeo.com", ["video", id, ..]) => Some((*id, param("h"))),
        _ => None,
    };
    if let Some((id, hash)) = vimeo.filter(|(id, _)| id.bytes().all(|b| b.is_ascii_digit())) {
        let hash = hash
            .filter(|h| h.bytes().all(|b| b.is_ascii_alphanumeric()))
            .map_or(String::new(), |h| format!("?h={h}"));
        return Embed::Frame(format!("https://player.vimeo.com/video/{id}{hash}"));
    }

    let file = path.to_ascii_lowercase();
    if [".mp4", ".m4v", ".mov", ".webm", ".ogv"]
        .iter()
        .any(|ext| file.ends_with(ext))
    {
        return Embed::File(url.to_string());
    }
    Embed::Link(url.to_string())
}

/// Whether `id` is a video's id on YouTube or YouCraft, which share its form:
/// eleven characters of the URL-safe base64 alphabet.
fn is_video_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The versions of an item, the first one chosen at first, each starting at
/// once with `autoplay`. A single version shows no pill: its name, if any,
/// has nothing to tell apart.
#[component]
pub fn Versions(
    videos: Vec<Video>,
    link_label: String,
    #[prop(optional)] autoplay: bool,
) -> impl IntoView {
    let chosen = RwSignal::new(0usize);
    let pills = (videos.len() > 1).then(|| {
        view! {
            <div class="version-pills">
                {videos.iter().enumerate().map(|(i, video)| view! {
                    <button
                        type="button"
                        class="version-pill"
                        class:version-on=move || chosen.get() == i
                        aria-pressed=move || (chosen.get() == i).to_string()
                        on:click=move |_| chosen.set(i)
                    >
                        {video.label.clone()}
                    </button>
                }).collect_view()}
            </div>
        }
    });
    let player = move || {
        let url = videos.get(chosen.get())?.url.clone();
        Some(view! { <Player url=url link_label=link_label.clone() autoplay=autoplay /> })
    };
    view! { {pills} {player} }
}

/// `url`'s player, which starts at once with `autoplay`, as when a tap on
/// the item asked for it.
#[component]
pub fn Player(url: String, link_label: String, #[prop(optional)] autoplay: bool) -> impl IntoView {
    match embed(&url) {
        Embed::Frame(src) => {
            let src = if autoplay { with_autoplay(&src) } else { src };
            view! {
                <div class="video-frame">
                    <iframe
                        src=src
                        allow="autoplay; fullscreen; picture-in-picture; encrypted-media"
                        allowfullscreen=true
                        referrerpolicy="strict-origin-when-cross-origin"
                    ></iframe>
                </div>
            }
            .into_any()
        }
        Embed::File(src) => view! {
            <video
                class="video-file"
                src=src
                controls=true
                autoplay=autoplay
                preload="metadata"
                playsinline=true
            ></video>
        }
        .into_any(),
        Embed::Link(href) => view! {
            <a class="video-link" href=href target="_blank" rel="noopener">{link_label}</a>
        }
        .into_any(),
    }
}

/// A player page's `src`, asking it to start playing: the players `embed`
/// knows all take `autoplay=1`.
fn with_autoplay(src: &str) -> String {
    let sep = if src.contains('?') { '&' } else { '?' };
    format!("{src}{sep}autoplay=1")
}

/// Whether `film` lacks the video anyone could give it: the site has videos
/// and lets its items be edited, and this one has none.
pub fn misses_video(film: &Film) -> bool {
    let cfg = SiteConfig::get();
    cfg.editable && cfg.fields.videos.is_some() && film.videos.is_empty()
}

/// Where the video of an item that has none would play: a button that
/// opens its form.
#[component]
pub fn MissingVideo(on_add: Callback<()>) -> impl IntoView {
    view! {
        <button
            type="button"
            class="video-missing"
            on:click=move |ev| {
                ev.stop_propagation();
                on_add.run(());
            }
        >
            {icon(Icon::Clapperboard)}
            <b>{t().video_missing}</b>
            <span>{t().video_add}</span>
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_files_and_links() {
        let yt = Embed::Frame("https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ".into());
        assert_eq!(embed("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3"), yt);
        assert_eq!(embed("https://youtu.be/dQw4w9WgXcQ?si=x"), yt);
        assert_eq!(embed("https://m.youtube.com/shorts/dQw4w9WgXcQ"), yt);
        let yc = Embed::Frame("https://youcraft.tube/embed/I5g2TEn0UtU".into());
        assert_eq!(embed("https://youcraft.tube/watch?v=I5g2TEn0UtU"), yc);
        assert_eq!(embed("https://www.youcraft.tube/embed/I5g2TEn0UtU"), yc);
        assert!(matches!(
            embed("https://youcraft.tube/watch?v=I5g2TEn0UtU%22"),
            Embed::Link(_)
        ));
        assert_eq!(
            embed("https://vimeo.com/123456/abcdef"),
            Embed::Frame("https://player.vimeo.com/video/123456?h=abcdef".into())
        );
        assert_eq!(
            embed("https://cheapmo.ch/48hfp-2026-wax/anim/rene-renait.mov"),
            Embed::File("https://cheapmo.ch/48hfp-2026-wax/anim/rene-renait.mov".into())
        );
        assert!(matches!(
            embed("https://drive.google.com/file/d/x/view"),
            Embed::Link(_)
        ));
        assert!(matches!(
            embed("https://www.youtube.com/watch?v=short"),
            Embed::Link(_)
        ));
    }
}

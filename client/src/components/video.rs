//! An item's video: played in place when the link is YouTube, Vimeo or a
//! video file, else offered as a link.

use leptos::prelude::*;

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
    if let Some(id) = youtube.filter(|id| is_youtube_id(id)) {
        return Embed::Frame(format!("https://www.youtube-nocookie.com/embed/{id}"));
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

fn is_youtube_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[component]
pub fn Video(url: String, link_label: String) -> impl IntoView {
    match embed(&url) {
        Embed::Frame(src) => view! {
            <div class="video-frame">
                <iframe
                    src=src
                    allow="autoplay; fullscreen; picture-in-picture; encrypted-media"
                    allowfullscreen=true
                    referrerpolicy="strict-origin-when-cross-origin"
                ></iframe>
            </div>
        }
        .into_any(),
        Embed::File(src) => view! {
            <video class="video-file" src=src controls=true preload="metadata" playsinline=true></video>
        }
        .into_any(),
        Embed::Link(href) => view! {
            <a class="video-link" href=href target="_blank" rel="noopener">{link_label}</a>
        }
        .into_any(),
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

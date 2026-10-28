use leptos::prelude::*;
use std::collections::HashSet;
use versus_shared::Film;

/// Per-site configuration fetched once from `/api/config` before the app mounts
/// and provided through Leptos context. It never changes at runtime, so it is
/// stored as a plain value rather than a signal.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SiteConfig {
    pub display_name: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub has_image: bool,
    #[serde(default = "default_true")]
    pub require_selection: bool,
    #[serde(default)]
    pub duel_question: String,
    #[serde(default)]
    pub noun: String,
    #[serde(default)]
    pub noun_plural: String,
    #[serde(default)]
    pub welcome_text: String,
    #[serde(default)]
    pub unknown_label: String,
    #[serde(default)]
    pub link_label: String,
    #[serde(default)]
    pub suggest_repo: String,
    #[serde(default)]
    pub source_url: String,
}

fn default_true() -> bool {
    true
}

impl Default for SiteConfig {
    fn default() -> Self {
        // Film defaults, so the UI stays sensible even if /api/config fails.
        Self {
            display_name: "FilmRank".into(),
            tagline: String::new(),
            has_image: true,
            require_selection: true,
            duel_question: String::new(),
            noun: "film".into(),
            noun_plural: "films".into(),
            welcome_text: "Please only vote on pairs you have real opinions about. \
                           Your honest comparisons make the ranking meaningful for everyone."
                .into(),
            unknown_label: "Skip this one".into(),
            link_label: "Open".into(),
            suggest_repo: String::new(),
            source_url: String::new(),
        }
    }
}

impl SiteConfig {
    /// Read the config from Leptos context (falls back to defaults if missing).
    pub fn get() -> Self {
        use_context::<SiteConfig>().unwrap_or_default()
    }
}

/// Capitalize the first character — for turning `noun_plural` into a heading.
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Select,
    Swipe,
    Board,
    More,
}

impl Page {
    pub fn from_str(s: &str) -> Option<Page> {
        match s {
            "select" => Some(Page::Select),
            "swipe" => Some(Page::Swipe),
            "board" => Some(Page::Board),
            "more" | "stats" => Some(Page::More),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Page::Select => "select",
            Page::Swipe => "swipe",
            Page::Board => "board",
            Page::More => "more",
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FilmPair {
    pub a: Film,
    pub b: Film,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteRecord {
    pub winner_id: usize,
    pub loser_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairDoneReason {
    NotEnough,
    FocusDone,
    AllDone,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LeaderboardEntry {
    pub film_id: usize,
    pub title: String,
    pub team: String,
    pub city: String,
    pub poster_url: String,
    #[serde(default)]
    pub video_url: String,
    pub rating: f64,
    pub wins: u32,
    pub losses: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Stats {
    pub total_votes: u64,
    pub active_users: u64,
    pub total_films: u64,
    pub films_with_votes: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Contribution {
    pub label: String,
    pub is_you: bool,
    pub votes: u64,
    pub films_voted: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixFilm {
    pub id: usize,
    pub title: String,
    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixVote {
    pub film_a: usize,
    pub film_b: usize,
    pub winner: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixWin {
    pub winner: usize,
    pub loser: usize,
    pub count: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UserMatrixData {
    pub films: Vec<MatrixFilm>,
    pub votes: Vec<MatrixVote>,
    #[serde(default)]
    pub legacy_votes: Vec<MatrixVote>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GlobalMatrixData {
    pub films: Vec<MatrixFilm>,
    pub wins: Vec<MatrixWin>,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: f64,
    pub html: String,
}

#[derive(Clone)]
pub struct AppState {
    pub page: RwSignal<Page>,
    pub films: RwSignal<Vec<Film>>,
    pub selected_ids: RwSignal<HashSet<usize>>,
    pub search_query: RwSignal<String>,

    pub pair: RwSignal<Option<FilmPair>>,
    pub pair_status: RwSignal<Option<PairDoneReason>>,
    pub vote_count: RwSignal<usize>,
    pub vote_history: RwSignal<Vec<VoteRecord>>,
    pub focus_film_id: RwSignal<Option<usize>>,

    pub board: RwSignal<Vec<LeaderboardEntry>>,
    pub stats: RwSignal<Option<Stats>>,
    pub contributions: RwSignal<Vec<Contribution>>,
    pub user_matrix: RwSignal<Option<UserMatrixData>>,
    pub global_matrix: RwSignal<Option<GlobalMatrixData>>,

    pub toasts: RwSignal<Vec<Toast>>,
    pub banned: RwSignal<bool>,

    pub user_id: String,
}

impl AppState {
    pub fn new() -> Self {
        let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let user_id = storage
            .get_item("filmrank_uid")
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                let id = gen_uuid_v4();
                storage.set_item("filmrank_uid", &id).unwrap();
                id
            });

        let selected_ids: HashSet<usize> = storage
            .get_item("filmrank_selected")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Self {
            page: RwSignal::new(Page::Swipe),
            films: RwSignal::new(Vec::new()),
            selected_ids: RwSignal::new(selected_ids),
            search_query: RwSignal::new(String::new()),

            pair: RwSignal::new(None),
            pair_status: RwSignal::new(None),
            vote_count: RwSignal::new(0),
            vote_history: RwSignal::new(Vec::new()),
            focus_film_id: RwSignal::new(None),

            board: RwSignal::new(Vec::new()),
            stats: RwSignal::new(None),
            contributions: RwSignal::new(Vec::new()),
            user_matrix: RwSignal::new(None),
            global_matrix: RwSignal::new(None),

            toasts: RwSignal::new(Vec::new()),
            banned: RwSignal::new(false),

            user_id,
        }
    }

    pub fn add_toast(&self, html: String) {
        let id = js_sys::Math::random() + js_sys::Date::now();
        self.toasts.update(|t| {
            t.push(Toast {
                id,
                html: html.clone(),
            })
        });
        let toasts = self.toasts;
        crate::timeout::Timeout::new(4_000, move || {
            toasts.update(|t| t.retain(|toast| toast.id != id));
        })
        .forget();
    }

    pub fn navigate(&self, page: Page) {
        self.page.set(page);

        // Update body class for swipe
        let doc = web_sys::window().unwrap().document().unwrap();
        let body = doc.body().unwrap();
        if page == Page::Swipe {
            body.class_list().add_1("swipe-active").unwrap();
        } else {
            body.class_list().remove_1("swipe-active").unwrap();
        }

        // Update hash
        let window = web_sys::window().unwrap();
        let _ = window.location().set_hash(page.as_str());

        // Trigger data loading for the page
        let state = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match page {
                Page::Swipe => crate::api::load_pair(&state).await,
                Page::Board => crate::api::load_board(&state).await,
                Page::More => crate::api::load_more(&state).await,
                Page::Select => {}
            }
        });
    }
}

fn gen_uuid_v4() -> String {
    let mut buf = [0u8; 16];
    let array = js_sys::Uint8Array::new_with_length(16);
    web_sys::window()
        .unwrap()
        .crypto()
        .unwrap()
        .get_random_values_with_u8_array(&mut buf)
        .unwrap();
    array.copy_from(&buf);
    buf[6] = (buf[6] & 0x0f) | 0x40; // version 4
    buf[8] = (buf[8] & 0x3f) | 0x80; // variant 1
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        buf[0],
        buf[1],
        buf[2],
        buf[3],
        buf[4],
        buf[5],
        buf[6],
        buf[7],
        buf[8],
        buf[9],
        buf[10],
        buf[11],
        buf[12],
        buf[13],
        buf[14],
        buf[15],
    )
}

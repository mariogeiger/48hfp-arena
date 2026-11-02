use crate::i18n::Lang;
use leptos::prelude::*;
use std::collections::HashSet;
use versus_shared::{Film, Proposal};

/// Per-site configuration fetched once from `/api/config` before the app mounts.
/// It never changes at runtime, so it is installed once as a plain global value:
/// readable anywhere, including async code that runs outside any reactive owner.
/// Empty copy means the site leaves the wording to its language (see
/// `i18n::or_default`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SiteConfig {
    pub display_name: String,
    #[serde(default)]
    pub has_image: bool,
    #[serde(default = "default_true")]
    pub require_selection: bool,
    #[serde(default)]
    pub duel_question: String,
    #[serde(default = "default_noun")]
    pub noun: String,
    #[serde(default = "default_noun_plural")]
    pub noun_plural: String,
    #[serde(default)]
    pub welcome_text: String,
    #[serde(default)]
    pub unknown_label: String,
    #[serde(default)]
    pub link_label: String,
    #[serde(default)]
    pub unranked_banner: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub editable: bool,
    #[serde(default)]
    pub lang: Lang,
    #[serde(default)]
    pub fields: FieldNames,
    #[serde(default)]
    pub min_votes: u32,
    #[serde(default)]
    pub min_voters: u32,
}

/// What the site calls each field of an item; `None` for the fields it lacks.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FieldNames {
    pub title: Option<String>,
    pub team: Option<String>,
    pub city: Option<String>,
    pub poster_url: Option<String>,
    pub video_url: Option<String>,
}

impl Default for FieldNames {
    fn default() -> Self {
        Self {
            title: Some(String::new()),
            team: Some(String::new()),
            city: Some(String::new()),
            poster_url: Some(String::new()),
            video_url: Some(String::new()),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_noun() -> String {
    "item".into()
}
fn default_noun_plural() -> String {
    "items".into()
}

impl Default for SiteConfig {
    fn default() -> Self {
        // So the UI stays sensible even if /api/config fails.
        Self {
            display_name: "Versus".into(),
            has_image: true,
            require_selection: true,
            duel_question: String::new(),
            noun: default_noun(),
            noun_plural: default_noun_plural(),
            welcome_text: String::new(),
            unknown_label: String::new(),
            link_label: String::new(),
            unranked_banner: String::new(),
            source_url: String::new(),
            editable: false,
            lang: Lang::En,
            fields: FieldNames::default(),
            min_votes: 10,
            min_voters: 2,
        }
    }
}

thread_local! {
    static SITE_CONFIG: std::cell::OnceCell<SiteConfig> = const { std::cell::OnceCell::new() };
}

impl SiteConfig {
    /// Install the config, once, before the app mounts.
    pub fn install(self) {
        SITE_CONFIG.with(|c| {
            let _ = c.set(self);
        });
    }

    /// The installed config, or the defaults before installation.
    pub fn get() -> Self {
        SITE_CONFIG.with(|c| c.get().cloned().unwrap_or_default())
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
    pub rank: u64,
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

    /// Proposed items awaiting review (only on editable sites).
    pub proposals: RwSignal<Vec<Proposal>>,
    /// The item sheet open over the page, if any.
    pub sheet: RwSignal<Option<Sheet>>,

    pub user_id: String,
}

/// What the item sheet shows: an item, published or proposed, or the form
/// for proposing a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    Item(usize),
    New,
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

            proposals: RwSignal::new(Vec::new()),
            sheet: RwSignal::new(None),

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

    /// Open the sheet of an item over the leaderboard; its link is `#film/<id>`.
    pub fn open_sheet(&self, sheet: Sheet) {
        if self.page.get_untracked() != Page::Board {
            self.navigate(Page::Board);
        }
        if let Sheet::Item(id) = sheet {
            let _ = web_sys::window()
                .unwrap()
                .location()
                .set_hash(&format!("film/{id}"));
        }
        self.sheet.set(Some(sheet));
    }

    /// Show what the address names, when the browser changes it (the back
    /// button, a link): an item's sheet for `#film/<id>`, else a page.
    pub fn follow_hash(&self) {
        let hash = web_sys::window()
            .unwrap()
            .location()
            .hash()
            .unwrap_or_default();
        let hash = hash.trim_start_matches('#');
        if let Some(id) = hash.strip_prefix("film/").and_then(|id| id.parse().ok()) {
            if self.sheet.get_untracked() != Some(Sheet::Item(id)) {
                self.open_sheet(Sheet::Item(id));
            }
        } else if let Some(page) = Page::from_str(hash) {
            self.sheet.set(None);
            if self.page.get_untracked() != page {
                self.navigate(page);
            }
        }
    }

    pub fn close_sheet(&self) {
        self.sheet.set(None);
        let _ = web_sys::window()
            .unwrap()
            .location()
            .set_hash(self.page.get_untracked().as_str());
    }

    pub fn navigate(&self, page: Page) {
        self.sheet.set(None);
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
                Page::Board => {
                    crate::api::load_board(&state).await;
                    crate::wiki::load_proposals(&state).await;
                }
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

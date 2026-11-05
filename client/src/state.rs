//! What the app knows and shows: the site's configuration, the page, the
//! sheet or panel open over it, the viewer's identity and selection, and the
//! data each page draws, as signals the components read.

use crate::i18n::Lang;
use leptos::prelude::*;
use std::collections::HashSet;
use versus_shared::{Film, Outcome, Proposal, canonical_pair};

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
    pub start_page: String,
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
    /// The site's Google OAuth client ID; empty offers no Google sign-in.
    #[serde(default)]
    pub google_client_id: String,
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
    pub videos: Option<String>,
    #[serde(default)]
    pub synopsis: Option<String>,
}

impl Default for FieldNames {
    fn default() -> Self {
        Self {
            title: Some(String::new()),
            team: Some(String::new()),
            city: Some(String::new()),
            poster_url: Some(String::new()),
            videos: Some(String::new()),
            synopsis: None,
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
            start_page: String::new(),
            noun: default_noun(),
            noun_plural: default_noun_plural(),
            welcome_text: String::new(),
            unknown_label: String::new(),
            link_label: String::new(),
            unranked_banner: String::new(),
            source_url: String::new(),
            google_client_id: String::new(),
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

/// The pages, one per tab, and the page explaining the ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Duels,
    Board,
    Me,
    About,
}

impl Page {
    /// The page an address names; older addresses still lead somewhere.
    pub fn from_str(s: &str) -> Option<Page> {
        match s {
            "duels" | "swipe" | "select" => Some(Page::Duels),
            "board" | "stats" => Some(Page::Board),
            "me" | "more" => Some(Page::Me),
            "about" => Some(Page::About),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Page::Duels => "duels",
            Page::Board => "board",
            Page::Me => "me",
            Page::About => "about",
        }
    }
}

/// What may slide over the page besides an item's sheet or a duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Signing in, the link to another device, signing out.
    Account,
    /// The items the viewer knows.
    Selection,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FilmPair {
    pub a: Film,
    pub b: Film,
}

/// A pair the viewer passed on in the duels, which comes back only once
/// they voted on `votes_left` other duels, or when no other pair is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassedPair {
    pub pair: (usize, usize),
    pub votes_left: usize,
}

/// A vote cast in the duels: the pair as it was shown, and what it came to.
#[derive(Debug, Clone, PartialEq)]
pub struct VoteRecord {
    pub a: usize,
    pub b: usize,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairDoneReason {
    NotEnough,
    AllDone,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LeaderboardEntry {
    pub film_id: usize,
    pub title: String,
    pub rating: f64,
    pub wins: u32,
    #[serde(default)]
    pub ties: u32,
    pub losses: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Stats {
    pub total_votes: u64,
    pub active_users: u64,
    pub films_with_votes: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Contribution {
    pub rank: u64,
    pub is_you: bool,
    pub votes: u64,
    /// The voter's public nickname.
    #[serde(default)]
    pub nickname: Option<String>,
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
    pub outcome: Outcome,
}

/// A pair compared before outcomes were recorded.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixPair {
    pub film_a: usize,
    pub film_b: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixWin {
    pub winner: usize,
    pub loser: usize,
    pub count: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MatrixTie {
    pub a: usize,
    pub b: usize,
    pub count: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UserMatrixData {
    pub films: Vec<MatrixFilm>,
    pub votes: Vec<MatrixVote>,
    #[serde(default)]
    pub legacy_votes: Vec<MatrixPair>,
}

impl UserMatrixData {
    /// What the viewer's duel of `a` and `b` came to, if they voted on it.
    pub fn outcome(&self, a: usize, b: usize) -> Option<Outcome> {
        self.votes
            .iter()
            .find(|v| canonical_pair(v.film_a, v.film_b) == canonical_pair(a, b))
            .map(|v| v.outcome)
    }

    /// Whether the viewer compared `a` and `b` before outcomes were recorded.
    pub fn compared_before(&self, a: usize, b: usize) -> bool {
        self.legacy_votes
            .iter()
            .any(|v| canonical_pair(v.film_a, v.film_b) == canonical_pair(a, b))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GlobalMatrixData {
    pub films: Vec<MatrixFilm>,
    pub wins: Vec<MatrixWin>,
    #[serde(default)]
    pub ties: Vec<MatrixTie>,
}

impl GlobalMatrixData {
    /// Everyone's duels, indexed by pair.
    pub fn tally(&self) -> GlobalTally {
        GlobalTally {
            wins: self
                .wins
                .iter()
                .map(|w| ((w.winner, w.loser), w.count))
                .collect(),
            ties: self
                .ties
                .iter()
                .map(|t| (canonical_pair(t.a, t.b), t.count))
                .collect(),
            scores: self
                .films
                .iter()
                .map(|f| (f.id, f.score.unwrap_or(1.0)))
                .collect(),
        }
    }
}

/// Everyone's duels: what each item did against each other, and the
/// strength the model gives it.
pub struct GlobalTally {
    wins: std::collections::HashMap<(usize, usize), u32>,
    ties: std::collections::HashMap<(usize, usize), u32>,
    scores: std::collections::HashMap<usize, f64>,
}

impl GlobalTally {
    /// `a`'s wins over `b`, their ties, and `a`'s losses to `b`.
    pub fn record(&self, a: usize, b: usize) -> (u32, u32, u32) {
        let wins = |w, l| self.wins.get(&(w, l)).copied().unwrap_or(0);
        let ties = self.ties.get(&canonical_pair(a, b)).copied().unwrap_or(0);
        (wins(a, b), ties, wins(b, a))
    }

    /// What `a` scored against `b`, a tie counting half: none if they never
    /// met.
    pub fn share(&self, a: usize, b: usize) -> Option<f64> {
        let (w, t, l) = self.record(a, b);
        let n = w + t + l;
        (n > 0).then(|| (w as f64 + 0.5 * t as f64) / n as f64)
    }

    /// The chance the model gives `a` against `b`.
    pub fn chance(&self, a: usize, b: usize) -> f64 {
        let score = |id| self.scores.get(&id).copied().unwrap_or(1.0);
        score(a) / (score(a) + score(b))
    }
}

/// The Google account this identity signs in with, as the viewer sees it.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct GoogleProfile {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub picture: String,
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

    pub pair: RwSignal<Option<FilmPair>>,
    pub pair_status: RwSignal<Option<PairDoneReason>>,
    pub vote_count: RwSignal<usize>,
    /// The duels left among the viewer's items, as of the last pair drawn.
    pub pairs_left: RwSignal<Option<usize>>,
    pub vote_history: RwSignal<Vec<VoteRecord>>,
    /// The pairs passed on lately, oldest first.
    pub passed: StoredValue<Vec<PassedPair>>,

    pub board: RwSignal<Vec<LeaderboardEntry>>,
    pub stats: RwSignal<Option<Stats>>,
    pub contributions: RwSignal<Vec<Contribution>>,
    pub user_matrix: RwSignal<Option<UserMatrixData>>,
    pub global_matrix: RwSignal<Option<GlobalMatrixData>>,
    /// Counts the votes others cast, so the ranking's tab can pulse.
    pub live_pulse: RwSignal<u32>,

    pub toasts: RwSignal<Vec<Toast>>,
    pub banned: RwSignal<bool>,

    /// Proposed items awaiting review (only on editable sites).
    pub proposals: RwSignal<Vec<Proposal>>,
    /// The item sheet open over the page, if any.
    pub sheet: RwSignal<Option<Sheet>>,
    /// The duel open over the page, if any: the left item, then the right one.
    pub duel: RwSignal<Option<(usize, usize)>>,
    /// The panel open over the page, if any.
    pub panel: RwSignal<Option<Panel>>,
    /// The item whose video plays large over everything, if any.
    pub screening: RwSignal<Option<usize>>,

    /// The private name of this identity, once known.
    pub user_name: RwSignal<Option<String>>,
    /// The Google account that signs in as this identity, if any.
    pub google: RwSignal<Option<GoogleProfile>>,
    /// Another identity a `#continue/<uuid>` link offers, awaiting the
    /// viewer's answer.
    pub handoff: RwSignal<Option<Handoff>>,

    pub user_id: String,
}

/// An identity a `#continue/<uuid>` link offers, as the server knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Handoff {
    pub user_id: String,
    pub name: Option<String>,
    pub votes: usize,
    /// The votes of this device's identity, which would fold into it.
    pub own_votes: usize,
}

/// What the item sheet shows: an item, published or proposed, its edit form
/// straight away, or the form for proposing a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    Item(usize),
    Edit(usize),
    /// The edit form, its video field ready for a link.
    AddVideo(usize),
    New,
}

impl Sheet {
    /// The item shown, if any.
    pub fn item(self) -> Option<usize> {
        match self {
            Sheet::Item(id) | Sheet::Edit(id) | Sheet::AddVideo(id) => Some(id),
            Sheet::New => None,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let user_id = storage
            .get_item(USER_ID_KEY)
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                let id = gen_uuid_v4();
                storage.set_item(USER_ID_KEY, &id).unwrap();
                id
            });

        Self {
            page: RwSignal::new(Page::Duels),
            films: RwSignal::new(Vec::new()),
            selected_ids: RwSignal::new(HashSet::new()),

            pair: RwSignal::new(None),
            pair_status: RwSignal::new(None),
            vote_count: RwSignal::new(0),
            pairs_left: RwSignal::new(None),
            vote_history: RwSignal::new(Vec::new()),
            passed: StoredValue::new(Vec::new()),

            board: RwSignal::new(Vec::new()),
            stats: RwSignal::new(None),
            contributions: RwSignal::new(Vec::new()),
            user_matrix: RwSignal::new(None),
            global_matrix: RwSignal::new(None),
            live_pulse: RwSignal::new(0),

            toasts: RwSignal::new(Vec::new()),
            banned: RwSignal::new(false),

            proposals: RwSignal::new(Vec::new()),
            sheet: RwSignal::new(None),
            duel: RwSignal::new(None),
            panel: RwSignal::new(None),
            screening: RwSignal::new(None),

            user_name: RwSignal::new(None),
            google: RwSignal::new(None),
            handoff: RwSignal::new(None),

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

    /// Open the sheet of an item; its link is `#film/<id>`.
    pub fn open_sheet(&self, sheet: Sheet) {
        if let Some(id) = sheet.item() {
            let _ = web_sys::window()
                .unwrap()
                .location()
                .set_hash(&format!("film/{id}"));
        }
        self.sheet.set(Some(sheet));
    }

    /// Show what the address names, when the browser changes it (the back
    /// button, a link): an item's sheet for `#film/<id>`, the offer of
    /// another identity for `#continue/<uuid>`, else a page.
    pub fn follow_hash(&self) {
        let hash = web_sys::window()
            .unwrap()
            .location()
            .hash()
            .unwrap_or_default();
        let hash = hash.trim_start_matches('#');
        if let Some(other) = hash.strip_prefix("continue/") {
            let (state, other) = (self.clone(), other.to_string());
            self.navigate(self.page.get_untracked());
            wasm_bindgen_futures::spawn_local(async move {
                crate::identity::offer_handoff(&state, &other).await;
            });
        } else if let Some(id) = hash.strip_prefix("film/").and_then(|id| id.parse().ok()) {
            if self.sheet.get_untracked().and_then(Sheet::item) != Some(id) {
                self.open_sheet(Sheet::Item(id));
            }
        } else if let Some(page) = Page::from_str(hash) {
            self.sheet.set(None);
            if self.page.get_untracked() != page {
                self.navigate(page);
            }
            if hash == "select" {
                self.panel.set(Some(Panel::Selection));
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
        self.duel.set(None);
        self.panel.set(None);
        self.page.set(page);

        // The duels fill the screen and do not scroll.
        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();
        let _ = body
            .class_list()
            .toggle_with_force("duels-active", page == Page::Duels);

        let window = web_sys::window().unwrap();
        let _ = window.location().set_hash(page.as_str());
        window.scroll_to_with_x_and_y(0.0, 0.0);

        let state = self.clone();
        wasm_bindgen_futures::spawn_local(async move { state.load_page(page).await });
    }

    /// Fetch what `page` shows.
    pub async fn load_page(&self, page: Page) {
        match page {
            Page::Duels => crate::api::load_pair(self).await,
            Page::Board => {
                crate::api::load_board(self).await;
                crate::wiki::load_proposals(self).await;
                crate::api::load_backstage(self).await;
            }
            Page::Me => crate::api::load_mine(self).await,
            Page::About => {}
        }
    }
}

/// The key under which the browser keeps its identity.
pub const USER_ID_KEY: &str = "filmrank_uid";

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

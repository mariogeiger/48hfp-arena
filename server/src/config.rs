//! Per-site configuration. One `versus-server` binary serves any dataset; the
//! behaviour for a given site is described entirely by a `site.toml` file.
//!
//! The config is loaded once at startup from the path in `SITE_CONFIG`
//! (default `./site.toml`, relative to the process working directory). Data
//! paths (`data_csv`, `db_path`, …) are likewise relative to the working
//! directory, so each launchd instance just points its `WorkingDirectory` at
//! its own instance folder.

use serde::{Deserialize, Serialize};

/// Maps the columns of the site's `data.csv` onto the five generic display
/// slots the UI knows how to render. Only `label` is required.
#[derive(Debug, Clone, Deserialize)]
pub struct Columns {
    /// Primary text (→ `Film.title`). Required.
    pub label: String,
    /// Secondary line (→ `Film.team`).
    #[serde(default)]
    pub sublabel: Option<String>,
    /// Tertiary line / badge (→ `Film.city`).
    #[serde(default)]
    pub detail: Option<String>,
    /// Image URL column (→ `Film.poster_url`).
    #[serde(default)]
    pub image: Option<String>,
    /// Video column, one version per line as `label url` (→ `Film.videos`).
    #[serde(default)]
    pub link: Option<String>,
    /// Long text column, several lines (→ `Film.synopsis`). The column may be
    /// absent from `data.csv` and filled in by editing.
    #[serde(default)]
    pub description: Option<String>,
}

/// Display names for the columns, by the same slots as `Columns`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FieldLabels {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sublabel: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Fr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Brand name shown in the title bar and header.
    pub display_name: String,
    #[serde(default)]
    pub tagline: String,

    // ---- runtime / deployment ----
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_data_csv")]
    pub data_csv: String,
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_banned_path")]
    pub banned_path: String,
    /// Directory of the built WASM client to serve. Instances point this at the
    /// shared repo build so a single `trunk build` serves every site.
    #[serde(default = "default_client_dist")]
    pub client_dist: String,
    /// Journal of the films' revisions (see `catalog`).
    #[serde(default = "default_edits_path")]
    pub edits_path: String,
    /// Directory of the posters people upload, served at `/uploads/`.
    #[serde(default = "default_uploads_dir")]
    pub uploads_dir: String,

    // ---- dataset shape ----
    pub columns: Columns,
    /// Whether items carry images. When false the client hides the image slot
    /// and renders text-only cards.
    #[serde(default)]
    pub has_image: bool,
    /// Whether users pick a subset to compare. When false every item is in play
    /// for everyone — the select step is skipped and pairs are drawn from the
    /// full set. Defaults to true (the 48hfp "films I've seen" behaviour).
    #[serde(default = "default_true")]
    pub require_selection: bool,
    /// The page a visit opens on when its address names none: `duels`,
    /// `board` or `me`; the older `select`, `swipe`, `stats` and `more` still
    /// lead to the page that took their place. Empty opens the duels.
    #[serde(default)]
    pub start_page: String,
    /// Whether every item starts selected, the viewer taking out those they
    /// don't know; else none does. Items added later join the selection too.
    #[serde(default)]
    pub select_by_default: bool,
    /// Leaderboard eligibility: an item appears once it has at least this many
    /// comparisons from at least `min_voters` distinct voters. Defaults match
    /// the original 48hfp thresholds (10 / 2). Set both to 1 to populate the
    /// board from the very first vote.
    #[serde(default = "default_min_votes")]
    pub min_votes: u32,
    #[serde(default = "default_min_voters")]
    pub min_voters: usize,
    /// Optional template to synthesize an image URL when the `image` column is
    /// empty. `{Header}` tokens are replaced with that row's column values.
    #[serde(default)]
    pub image_fallback: Option<String>,
    /// Whether anyone may edit the items and propose new ones, wiki-style.
    #[serde(default)]
    pub editable: bool,
    /// What the edit form calls each column; a column without a name here is
    /// called by its header in `data.csv`.
    #[serde(default)]
    pub field_labels: FieldLabels,

    /// Language of the interface.
    #[serde(default)]
    pub lang: Lang,

    // ---- user-facing copy (served to the client via /api/config) ----
    /// Question shown above each duel, e.g. "Which matters most?".
    #[serde(default)]
    pub duel_question: String,
    /// Singular noun for an item, e.g. "film" / "principle".
    #[serde(default = "default_noun")]
    pub noun: String,
    /// Plural noun, e.g. "films" / "principles".
    #[serde(default = "default_noun_plural")]
    pub noun_plural: String,
    /// Welcome-overlay body text.
    #[serde(default)]
    pub welcome_text: String,
    /// Label for the per-card "I don't know this one" button. Empty lets the
    /// client say it in the site's language, as for the other copy below.
    #[serde(default)]
    pub unknown_label: String,
    /// Label for the board's external link (the `link` column), e.g. "Watch".
    #[serde(default)]
    pub link_label: String,
    /// Banner above the board's items that lack the votes to be ranked, which
    /// it shows in random order.
    #[serde(default)]
    pub unranked_banner: String,
    /// URL behind the GitHub icon in the nav.
    #[serde(default)]
    pub source_url: String,

    // ---- signing in with Google ----
    /// The site's OAuth client ID; empty offers no Google sign-in.
    #[serde(default)]
    pub google_client_id: String,
    /// Google's public keys (a JWK set), kept fresh by the host.
    #[serde(default = "default_google_keys")]
    pub google_keys: String,
}

fn default_true() -> bool {
    true
}
fn default_min_votes() -> u32 {
    10
}
fn default_min_voters() -> usize {
    2
}
fn default_port() -> u16 {
    4848
}
fn default_data_csv() -> String {
    "data.csv".into()
}
fn default_db_path() -> String {
    "db.json".into()
}
fn default_backup_dir() -> String {
    "backups".into()
}
fn default_banned_path() -> String {
    "banned.txt".into()
}
fn default_client_dist() -> String {
    "./client/dist".into()
}
fn default_edits_path() -> String {
    "edits.jsonl".into()
}
fn default_uploads_dir() -> String {
    "uploads".into()
}
fn default_google_keys() -> String {
    "google-keys.json".into()
}
fn default_noun() -> String {
    "item".into()
}
fn default_noun_plural() -> String {
    "items".into()
}

impl SiteConfig {
    /// Load from `SITE_CONFIG` (default `./site.toml`). Panics with a clear
    /// message on missing/invalid config — the server cannot run without it.
    pub fn load() -> Self {
        let path = std::env::var("SITE_CONFIG").unwrap_or_else(|_| "site.toml".into());
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Cannot read site config {path}: {e}"));
        toml::from_str(&content).unwrap_or_else(|e| panic!("Invalid site config {path}: {e}"))
    }

    /// The subset of config the browser client needs, as JSON for `/api/config`.
    pub fn client_json(&self) -> serde_json::Value {
        serde_json::json!({
            "display_name": self.display_name,
            "tagline": self.tagline,
            "has_image": self.has_image,
            "require_selection": self.require_selection,
            "start_page": self.start_page,
            "select_by_default": self.select_by_default,
            "duel_question": self.duel_question,
            "noun": self.noun,
            "noun_plural": self.noun_plural,
            "welcome_text": self.welcome_text,
            "unknown_label": self.unknown_label,
            "link_label": self.link_label,
            "unranked_banner": self.unranked_banner,
            "source_url": self.source_url,
            "google_client_id": self.google_client_id,
            "editable": self.editable,
            "min_votes": self.min_votes,
            "min_voters": self.min_voters,
            "lang": self.lang,
            "fields": self.field_names(),
        })
    }

    /// The display name of each column the site has; absent ones are null.
    fn field_names(&self) -> serde_json::Value {
        let c = &self.columns;
        let l = &self.field_labels;
        let name = |label: &Option<String>, header: &Option<String>| {
            header
                .as_ref()
                .map(|h| label.clone().unwrap_or_else(|| h.clone()))
        };
        serde_json::json!({
            "title": name(&l.label, &Some(c.label.clone())),
            "team": name(&l.sublabel, &c.sublabel),
            "city": name(&l.detail, &c.detail),
            "poster_url": name(&l.image, &c.image),
            "videos": name(&l.link, &c.link),
            "synopsis": name(&l.description, &c.description),
        })
    }
}

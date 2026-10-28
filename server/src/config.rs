//! Per-site configuration. One `versus-server` binary serves any dataset; the
//! behaviour for a given site is described entirely by a `site.toml` file.
//!
//! The config is loaded once at startup from the path in `SITE_CONFIG`
//! (default `./site.toml`, relative to the process working directory). Data
//! paths (`data_csv`, `db_path`, …) are likewise relative to the working
//! directory, so each launchd instance just points its `WorkingDirectory` at
//! its own instance folder.

use serde::Deserialize;

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
    /// External link column (→ `Film.video_url`).
    #[serde(default)]
    pub link: Option<String>,
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
    /// Label for the per-card "I don't know this one" button.
    #[serde(default = "default_unknown_label")]
    pub unknown_label: String,
    /// Label for the board's external link (the `link` column), e.g. "Watch".
    #[serde(default = "default_link_label")]
    pub link_label: String,
    /// GitHub `owner/repo` for the "suggest a correction" form. Empty hides it.
    #[serde(default)]
    pub suggest_repo: String,
    /// URL behind the GitHub icon in the nav.
    #[serde(default)]
    pub source_url: String,
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
fn default_noun() -> String {
    "item".into()
}
fn default_noun_plural() -> String {
    "items".into()
}
fn default_unknown_label() -> String {
    "Skip this one".into()
}
fn default_link_label() -> String {
    "Open".into()
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
            "duel_question": self.duel_question,
            "noun": self.noun,
            "noun_plural": self.noun_plural,
            "welcome_text": self.welcome_text,
            "unknown_label": self.unknown_label,
            "link_label": self.link_label,
            "suggest_repo": self.suggest_repo,
            "source_url": self.source_url,
        })
    }
}

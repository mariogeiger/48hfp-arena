//! The interface's words in each language the site may choose (`lang` in its
//! site.toml). Both tables are one struct, so a missing translation does not
//! compile. Placeholders such as `{n}` are filled by `say`, which also puts
//! the site's nouns in `{item}`, `{items}` and `{Items}`.

use std::cell::RefCell;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Fr,
}

impl Lang {
    /// BCP 47 tag, for dates.
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr-CH",
        }
    }
}

struct Speech {
    lang: Lang,
    noun: String,
    noun_plural: String,
}

thread_local! {
    static SPEECH: RefCell<Speech> = RefCell::new(Speech {
        lang: Lang::En,
        noun: "item".into(),
        noun_plural: "items".into(),
    });
}

/// Choose the language and the site's nouns once, before the app mounts.
pub fn set_speech(lang: Lang, noun: &str, noun_plural: &str) {
    SPEECH.with(|s| {
        *s.borrow_mut() = Speech {
            lang,
            noun: noun.to_string(),
            noun_plural: noun_plural.to_string(),
        }
    });
}

pub fn lang() -> Lang {
    SPEECH.with(|s| s.borrow().lang)
}

/// The words of the site's language.
pub fn t() -> &'static Words {
    match lang() {
        Lang::En => &EN,
        Lang::Fr => &FR,
    }
}

/// `template` with each `{name}` replaced by its value.
pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    args.iter().fold(template.to_string(), |s, (name, value)| {
        s.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

/// `template` filled with `args` and the site's nouns.
pub fn say(template: &str, args: &[(&str, &dyn Display)]) -> String {
    SPEECH.with(|s| {
        let s = s.borrow();
        let filled = fill(template, args);
        fill(
            &filled,
            &[
                ("item", &s.noun),
                ("items", &s.noun_plural),
                ("Items", &capitalize(&s.noun_plural)),
            ],
        )
    })
}

/// `s` with its first letter in upper case.
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// An item's wins, ties and losses, the ties told only when there are some.
pub fn record(wins: u32, ties: u32, losses: u32) -> String {
    let template = if ties == 0 {
        t().board_record
    } else {
        t().board_record_ties
    };
    fill(template, &[("w", &wins), ("t", &ties), ("l", &losses)])
}

/// The site's own wording when it has one, else the language's.
pub fn or_default(site: &str, words: &'static str) -> String {
    if site.is_empty() {
        say(words, &[])
    } else {
        site.to_string()
    }
}

pub struct Words {
    // Navigation
    pub tab_duels: &'static str,
    pub tab_board: &'static str,
    pub tab_me: &'static str,
    pub sign_in: &'static str,

    // Duels
    pub vote_one: &'static str,
    pub vote_many: &'static str,
    pub undo: &'static str,
    pub skip: &'static str,
    pub tie: &'static str,
    pub skip_unknown: &'static str,
    pub open_link: &'static str,
    pub done_none_heading: &'static str,
    pub done_none_text: &'static str,
    pub done_none_action: &'static str,
    pub done_all_heading: &'static str,
    pub done_all_text: &'static str,
    pub done_all_action: &'static str,
    pub milestone: &'static str,

    // Selection
    pub selection_chip: &'static str,
    pub selection_title: &'static str,
    pub selection_hint: &'static str,
    pub select_search: &'static str,
    pub select_all: &'static str,
    pub deselect_all: &'static str,
    pub select_count: &'static str,
    pub select_too_few: &'static str,

    // Ranking
    pub board_unranked_title: &'static str,
    pub board_unranked: &'static str,
    pub board_record: &'static str,
    pub board_record_ties: &'static str,
    pub overtook: &'static str,

    // Live events
    pub live_edited: &'static str,
    pub live_proposed: &'static str,
    pub live_approved: &'static str,
    pub live_rejected: &'static str,

    // Behind the scenes
    pub backstage_title: &'static str,
    pub stats_votes: &'static str,
    pub stats_voters: &'static str,
    pub stats_voted_on: &'static str,
    pub contrib_heading: &'static str,
    pub contrib_you: &'static str,
    pub contrib_voter: &'static str,
    pub contrib_you_named: &'static str,
    pub nickname_title: &'static str,
    pub nickname_hint: &'static str,
    pub nickname_draw: &'static str,
    pub nickname_placeholder: &'static str,
    pub matrix_global: &'static str,
    pub matrix_global_hint: &'static str,
    pub matrix_global_empty: &'static str,
    pub matrix_corner: &'static str,
    pub matrix_versus: &'static str,
    pub matrix_no_votes: &'static str,
    pub matrix_residual: &'static str,
    pub matrix_win: &'static str,
    pub matrix_loss: &'static str,
    pub matrix_tie: &'static str,
    pub how_it_works_link: &'static str,
    pub source_code: &'static str,

    // How it works
    pub how_it_works: &'static str,
    pub sim_votes: &'static str,
    pub sim_pause: &'static str,
    pub sim_play: &'static str,
    pub sim_step: &'static str,
    pub sim_reset: &'static str,
    pub sim_row_beats_col: &'static str,
    pub sim_score: &'static str,

    // My votes
    pub me_rank: &'static str,
    pub me_pairs: &'static str,
    pub keep_votes: &'static str,
    pub keep_votes_text: &'static str,
    pub matrix_yours: &'static str,
    pub matrix_yours_hint: &'static str,
    pub matrix_yours_empty: &'static str,
    pub reset_confirm: &'static str,
    pub reset_button: &'static str,

    // Duel sheet
    pub duel_hint: &'static str,
    pub duel_your_pick: &'static str,
    pub duel_none: &'static str,
    pub duel_legacy: &'static str,
    pub duel_remove: &'static str,
    pub duel_everyone: &'static str,
    pub duel_model: &'static str,
    pub percent: &'static str,

    // Account
    pub or: &'static str,
    pub other_device: &'static str,
    pub session_hint: &'static str,
    pub session_name: &'static str,
    pub session_name_placeholder: &'static str,
    pub session_name_needed: &'static str,
    pub session_share: &'static str,
    pub session_copied: &'static str,
    pub google_sign_out: &'static str,
    pub google_sign_out_confirm: &'static str,
    pub google_failed: &'static str,
    pub handoff_heading: &'static str,
    pub handoff_named: &'static str,
    pub handoff_unnamed: &'static str,
    pub handoff_fold: &'static str,
    pub handoff_warning: &'static str,
    pub handoff_accept: &'static str,
    pub handoff_refuse: &'static str,
    pub handoff_already: &'static str,
    pub handoff_unknown: &'static str,
    pub handoff_invalid: &'static str,

    // Overlays
    pub welcome_heading: &'static str,
    pub welcome_text: &'static str,
    pub welcome_promise: &'static str,
    pub welcome_refuse: &'static str,
    pub banned_heading: &'static str,
    pub banned_text: &'static str,

    // Wiki
    pub sheet_close: &'static str,
    pub sheet_edit: &'static str,
    pub video_missing: &'static str,
    pub video_add: &'static str,
    pub sheet_history: &'static str,
    pub sheet_back: &'static str,
    pub form_save: &'static str,
    pub form_cancel: &'static str,
    pub form_poster_upload: &'static str,
    pub form_poster_link: &'static str,
    pub form_uploading: &'static str,
    pub form_new_hint: &'static str,
    pub form_propose: &'static str,
    pub form_version_name: &'static str,
    pub form_versions_hint: &'static str,
    pub form_add_version: &'static str,
    pub form_move_up: &'static str,
    pub form_remove_version: &'static str,
    pub saved: &'static str,
    pub proposed: &'static str,
    pub add_item: &'static str,
    pub proposals_heading: &'static str,
    pub proposals_hint: &'static str,
    pub proposal_badge: &'static str,
    pub proposal_yours: &'static str,
    pub approve: &'static str,
    pub reject: &'static str,
    pub approved: &'static str,
    pub rejected: &'static str,
    pub author_anon: &'static str,
    pub author_you: &'static str,
    pub history_edit: &'static str,
    pub history_revert: &'static str,
    pub history_revert_original: &'static str,
    pub history_propose: &'static str,
    pub history_approve: &'static str,
    pub history_reject: &'static str,
    pub history_original: &'static str,
    pub history_restore: &'static str,
    pub history_current: &'static str,
    pub history_empty: &'static str,
    pub field_title: &'static str,
    pub field_team: &'static str,
    pub field_city: &'static str,
    pub field_poster: &'static str,
    pub field_video: &'static str,
    pub field_synopsis: &'static str,

    // Errors
    pub error_title_required: &'static str,
    pub error_bad_text: &'static str,
    pub error_bad_synopsis: &'static str,
    pub error_bad_url: &'static str,
    pub error_video_url_required: &'static str,
    pub error_video_label_required: &'static str,
    pub error_too_many_videos: &'static str,
    pub error_unchanged: &'static str,
    pub error_unknown_film: &'static str,
    pub error_not_pending: &'static str,
    pub error_own_proposal: &'static str,
    pub error_rate_limited: &'static str,
    pub error_read_only: &'static str,
    pub error_banned: &'static str,
    pub error_too_large: &'static str,
    pub error_not_image: &'static str,
    pub error_other: &'static str,
}

impl Words {
    /// The message for an error code from the server.
    pub fn error(&self, code: &str) -> &'static str {
        match code {
            "title_required" => self.error_title_required,
            "bad_text" => self.error_bad_text,
            "bad_synopsis" => self.error_bad_synopsis,
            "bad_url" => self.error_bad_url,
            "video_url_required" => self.error_video_url_required,
            "video_label_required" => self.error_video_label_required,
            "too_many_videos" => self.error_too_many_videos,
            "unchanged" => self.error_unchanged,
            "unknown_film" | "unknown_revision" => self.error_unknown_film,
            "not_pending" => self.error_not_pending,
            "own_proposal" => self.error_own_proposal,
            "rate_limited" => self.error_rate_limited,
            "read_only" => self.error_read_only,
            "banned" => self.error_banned,
            "too_large" => self.error_too_large,
            "not_jpeg" => self.error_not_image,
            _ => self.error_other,
        }
    }
}

pub static EN: Words = Words {
    tab_duels: "Duels",
    tab_board: "Ranking",
    tab_me: "My votes",
    sign_in: "Sign in",

    vote_one: "vote",
    vote_many: "votes",
    undo: "Undo",
    skip: "Skip",
    tie: "Tie",
    skip_unknown: "Don\u{2019}t know",
    open_link: "Open",
    done_none_heading: "Pick your {items}",
    done_none_text: "At least two, for a duel.",
    done_none_action: "Pick",
    done_all_heading: "Every duel is done!",
    done_all_text: "{n} votes. Bravo!",
    done_all_action: "See the ranking",
    milestone: "\u{1F389} {n} votes!",

    selection_chip: "{n} {items}",
    selection_title: "My {items}",
    selection_hint: "Keep only the {items} you know.",
    select_search: "Search\u{2026}",
    select_all: "All",
    deselect_all: "None",
    select_count: "Done \u{00B7} {n} {items}",
    select_too_few: "Pick at least 2",

    board_unranked_title: "Not ranked yet",
    board_unranked: "They need more votes: shown in random order.",
    board_record: "{w}W \u{00B7} {l}L",
    board_record_ties: "{w}W \u{00B7} {t}T \u{00B7} {l}L",
    overtook: "<strong>{a}</strong> overtook <strong>{b}</strong> \u{2192} #{n}",

    live_edited: "{title} was edited",
    live_proposed: "New proposal: {title}",
    live_approved: "{title} joins the ranking",
    live_rejected: "The proposal {title} was rejected",

    backstage_title: "Behind the scenes",
    stats_votes: "votes",
    stats_voters: "voters",
    stats_voted_on: "{items} with votes",
    contrib_heading: "Top voters",
    contrib_you: "You",
    contrib_voter: "Voter {n}",
    contrib_you_named: "You \u{00B7} {name}",
    nickname_title: "Your nickname",
    nickname_hint: "The name the top voters show for you. Draw another, or write your own.",
    nickname_draw: "Draw another nickname",
    nickname_placeholder: "Write your own",
    matrix_global: "Every duel",
    matrix_global_hint: "Green: wins more than the model expects; red: less. Tap a cell.",
    matrix_global_empty: "No data yet.",
    matrix_corner: "\u{2193} beats \u{2192}",
    matrix_versus: "{a} vs {b}",
    matrix_no_votes: "{a} vs {b}: no votes",
    matrix_residual: "{a} vs {b}\nobserved {o}% / model {m}% ({r}%)",
    matrix_win: "W",
    matrix_loss: "L",
    matrix_tie: "0",
    how_it_works_link: "How is it computed?",
    source_code: "Source code",

    how_it_works: "How It Works",
    sim_votes: "{n} votes",
    sim_pause: "\u{23F8} Pause",
    sim_play: "\u{25B6} Play",
    sim_step: "+1 vote",
    sim_reset: "Reset",
    sim_row_beats_col: "row beats col \u{2192}",
    sim_score: "Score",

    me_rank: "#{n} of {m} voters",
    me_pairs: "{done} of {total} duels",
    keep_votes: "Keep your votes",
    keep_votes_text: "Find them on all your devices.",
    matrix_yours: "My duels",
    matrix_yours_hint: "Tap a cell to revisit its duel.",
    matrix_yours_empty: "No duels yet.",
    reset_confirm: "This will permanently delete all your votes. Are you sure?",
    reset_button: "Erase all my votes",

    duel_hint: "Tap the one you prefer.",
    duel_your_pick: "Your pick",
    duel_none: "You haven\u{2019}t voted on this pair yet.",
    duel_legacy: "You compared them before winners were recorded: pick again.",
    duel_remove: "Remove my vote",
    duel_everyone: "Everyone",
    duel_model: "The model",
    percent: "{p}%",

    or: "or",
    other_device: "On another device",
    session_hint: "Whoever opens this link votes as you: keep it to yourself.",
    session_name: "Your name",
    session_name_placeholder: "e.g. Tom",
    session_name_needed: "Your name first: the link asks whoever opens it whether they are you.",
    session_share: "Share the link",
    session_copied: "Link copied",
    google_sign_out: "Sign out",
    google_sign_out_confirm: "This device will start afresh; your votes stay with your Google account, which signing in finds again.",
    google_failed: "Signing in with Google failed.",
    handoff_heading: "Continue a session?",
    handoff_named: "This link continues the session of {name} ({n} votes). Are you {name}?",
    handoff_unnamed: "This link continues an unnamed session ({n} votes). Is it yours?",
    handoff_fold: "The {n} votes of this device will join it.",
    handoff_warning: "If someone sent you this link by mistake, decline: you would be voting in their name.",
    handoff_accept: "Yes, continue",
    handoff_refuse: "No",
    handoff_already: "This link is your own session.",
    handoff_unknown: "This link names no session.",
    handoff_invalid: "This link is broken.",

    welcome_heading: "Welcome!",
    welcome_text: "Please only vote on pairs you have real opinions about. Your honest comparisons make the ranking meaningful for everyone.",
    welcome_promise: "I promise",
    welcome_refuse: "I can't promise that",
    banned_heading: "Account Suspended",
    banned_text: "Your account has been suspended for violating the voting rules. If you believe this is a mistake, please contact the site administrator.",

    sheet_close: "Close",
    sheet_edit: "Edit",
    video_missing: "No video yet",
    video_add: "Add the video",
    sheet_history: "History",
    sheet_back: "Back",
    form_save: "Save",
    form_cancel: "Cancel",
    form_poster_upload: "Upload an image",
    form_poster_link: "or paste a link to an image",
    form_uploading: "Uploading\u{2026}",
    form_new_hint: "Someone else will have to approve it before it enters the ranking.",
    form_propose: "Propose",
    form_version_name: "Name",
    form_versions_hint: "The first one opens by default.",
    form_add_version: "+ Add a version",
    form_move_up: "Move up",
    form_remove_version: "Remove this version",
    saved: "Saved",
    proposed: "Proposed: someone else now has to approve it",
    add_item: "Add a {item}",
    proposals_heading: "Proposed {items}",
    proposals_hint: "Each one enters the ranking once someone other than its author approves it.",
    proposal_badge: "Proposed",
    proposal_yours: "You proposed this one: someone else has to review it.",
    approve: "Approve",
    reject: "Reject",
    approved: "Approved",
    rejected: "Rejected",
    author_anon: "anonymous \u{00B7} {tag}",
    author_you: "you",
    history_edit: "Edited",
    history_revert: "Restored the version of revision {n}",
    history_revert_original: "Restored the original version",
    history_propose: "Proposed",
    history_approve: "Approved",
    history_reject: "Rejected",
    history_original: "Original version",
    history_restore: "Restore this version",
    history_current: "Current version",
    history_empty: "No changes yet.",
    field_title: "Title",
    field_team: "Team",
    field_city: "City",
    field_poster: "Poster",
    field_video: "Video",
    field_synopsis: "Synopsis",

    error_title_required: "The title is required.",
    error_bad_text: "Text must be one line of at most 200 characters.",
    error_bad_synopsis: "The synopsis can be at most 2000 characters.",
    error_bad_url: "Links must start with https:// or http://.",
    error_video_url_required: "Each version needs a link.",
    error_video_label_required: "With several versions, each one needs a name.",
    error_too_many_videos: "At most 10 versions.",
    error_unchanged: "Nothing changed.",
    error_unknown_film: "This item no longer exists.",
    error_not_pending: "This proposal was already reviewed.",
    error_own_proposal: "You can't review your own proposal.",
    error_rate_limited: "Too many changes for now: try again in an hour.",
    error_read_only: "This site doesn't take edits.",
    error_banned: "Your account is suspended.",
    error_too_large: "The image is too large.",
    error_not_image: "This file isn't an image the browser can read.",
    error_other: "Something went wrong: try again.",
};

pub static FR: Words = Words {
    tab_duels: "Duels",
    tab_board: "Classement",
    tab_me: "Mes votes",
    sign_in: "Connexion",

    vote_one: "vote",
    vote_many: "votes",
    undo: "Annuler",
    skip: "Passer",
    tie: "\u{00C9}galit\u{00E9}",
    skip_unknown: "Connais pas",
    open_link: "Ouvrir",
    done_none_heading: "Choisis tes {items}",
    done_none_text: "Au moins deux, pour un duel.",
    done_none_action: "Choisir",
    done_all_heading: "Tous les duels sont faits\u{00A0}!",
    done_all_text: "{n} votes. Bravo\u{00A0}!",
    done_all_action: "Voir le classement",
    milestone: "\u{1F389} {n} votes\u{00A0}!",

    selection_chip: "{n} {items}",
    selection_title: "Mes {items}",
    selection_hint: "Garde seulement les {items} que tu connais.",
    select_search: "Chercher\u{2026}",
    select_all: "Tous",
    deselect_all: "Aucun",
    select_count: "OK \u{00B7} {n} {items}",
    select_too_few: "Choisis-en au moins 2",

    board_unranked_title: "Pas encore class\u{00E9}s",
    board_unranked: "Il leur faut plus de votes\u{00A0}: ordre al\u{00E9}atoire.",
    board_record: "{w}V \u{00B7} {l}D",
    board_record_ties: "{w}V \u{00B7} {t}N \u{00B7} {l}D",
    overtook: "<strong>{a}</strong> d\u{00E9}passe <strong>{b}</strong> \u{2192} n\u{00B0}{n}",

    live_edited: "{title} a \u{00E9}t\u{00E9} modifi\u{00E9}",
    live_proposed: "Nouvelle proposition\u{00A0}: {title}",
    live_approved: "{title} entre dans le classement",
    live_rejected: "La proposition {title} a \u{00E9}t\u{00E9} rejet\u{00E9}e",

    backstage_title: "En coulisses",
    stats_votes: "votes",
    stats_voters: "votants",
    stats_voted_on: "{items} vot\u{00E9}s",
    contrib_heading: "Top votants",
    contrib_you: "Toi",
    contrib_voter: "Votant {n}",
    contrib_you_named: "Toi \u{00B7} {name}",
    nickname_title: "Ton surnom",
    nickname_hint: "Le nom que montre pour toi la liste des top votants. Tires-en un autre, ou \u{00E9}cris le tien.",
    nickname_draw: "Tirer un autre surnom",
    nickname_placeholder: "\u{00C9}cris le tien",
    matrix_global: "Tous les duels",
    matrix_global_hint: "Vert\u{00A0}: gagne plus que pr\u{00E9}vu\u{00A0}; rouge\u{00A0}: moins. Touche une case.",
    matrix_global_empty: "Pas encore de donn\u{00E9}es.",
    matrix_corner: "\u{2193} bat \u{2192}",
    matrix_versus: "{a} contre {b}",
    matrix_no_votes: "{a} contre {b}\u{00A0}: aucun vote",
    matrix_residual: "{a} contre {b}\nobserv\u{00E9} {o}\u{00A0}% / mod\u{00E8}le {m}\u{00A0}% ({r}\u{00A0}%)",
    matrix_win: "V",
    matrix_loss: "D",
    matrix_tie: "0",
    how_it_works_link: "Comment c\u{2019}est calcul\u{00E9}\u{00A0}?",
    source_code: "Code source",

    how_it_works: "Comment \u{00E7}a marche",
    sim_votes: "{n} votes",
    sim_pause: "\u{23F8} Pause",
    sim_play: "\u{25B6} Lecture",
    sim_step: "+1 vote",
    sim_reset: "Recommencer",
    sim_row_beats_col: "la ligne bat la colonne \u{2192}",
    sim_score: "Score",

    me_rank: "n\u{00B0}\u{00A0}{n} sur {m} votants",
    me_pairs: "{done} duels sur {total}",
    keep_votes: "Garde tes votes",
    keep_votes_text: "Retrouve-les sur tous tes appareils.",
    matrix_yours: "Mes duels",
    matrix_yours_hint: "Touche une case pour revoir son duel.",
    matrix_yours_empty: "Pas encore de duel.",
    reset_confirm: "Tous tes votes seront effac\u{00E9}s d\u{00E9}finitivement. Tu es s\u{00FB}r\u{00B7}e\u{00A0}?",
    reset_button: "Effacer tous mes votes",

    duel_hint: "Touche celui que tu pr\u{00E9}f\u{00E8}res.",
    duel_your_pick: "Ton choix",
    duel_none: "Tu n\u{2019}as pas encore vot\u{00E9} sur cette paire.",
    duel_legacy: "Tu les as compar\u{00E9}s avant que le gagnant soit enregistr\u{00E9}\u{00A0}: choisis \u{00E0} nouveau.",
    duel_remove: "Retirer mon vote",
    duel_everyone: "Tout le monde",
    duel_model: "Le mod\u{00E8}le",
    percent: "{p}\u{00A0}%",

    or: "ou",
    other_device: "Sur un autre appareil",
    session_hint: "Qui ouvre ce lien vote en ton nom\u{00A0}: garde-le pour toi.",
    session_name: "Ton nom",
    session_name_placeholder: "p. ex. Tom",
    session_name_needed: "D\u{2019}abord ton nom\u{00A0}: le lien demande \u{00E0} qui l\u{2019}ouvre s\u{2019}il est bien toi.",
    session_share: "Partager le lien",
    session_copied: "Lien copi\u{00E9}",
    google_sign_out: "Se d\u{00E9}connecter",
    google_sign_out_confirm: "Cet appareil repartira de z\u{00E9}ro\u{00A0}; tes votes restent li\u{00E9}s \u{00E0} ton compte Google, que tu retrouves en te reconnectant.",
    google_failed: "La connexion avec Google a \u{00E9}chou\u{00E9}.",
    handoff_heading: "Continuer une session\u{00A0}?",
    handoff_named: "Ce lien continue la session de {name} ({n} votes). Tu es bien {name}\u{00A0}?",
    handoff_unnamed: "Ce lien continue une session sans nom ({n} votes). C\u{2019}est la tienne\u{00A0}?",
    handoff_fold: "Les {n} votes faits sur cet appareil la rejoindront.",
    handoff_warning: "Si quelqu\u{2019}un t\u{2019}a envoy\u{00E9} ce lien par erreur, refuse\u{00A0}: tu voterais \u{00E0} sa place.",
    handoff_accept: "Oui, continuer",
    handoff_refuse: "Non",
    handoff_already: "Ce lien est ta propre session.",
    handoff_unknown: "Ce lien ne m\u{00E8}ne \u{00E0} aucune session.",
    handoff_invalid: "Ce lien est cass\u{00E9}.",

    welcome_heading: "Bienvenue\u{00A0}!",
    welcome_text: "Ne vote que sur des paires que tu connais vraiment. Des comparaisons honn\u{00EA}tes rendent le classement juste pour tout le monde.",
    welcome_promise: "Promis",
    welcome_refuse: "Je ne peux pas promettre",
    banned_heading: "Compte suspendu",
    banned_text: "Ton compte a \u{00E9}t\u{00E9} suspendu pour non-respect des r\u{00E8}gles de vote. Si c\u{2019}est une erreur, contacte l\u{2019}administrateur du site.",

    sheet_close: "Fermer",
    sheet_edit: "Modifier",
    video_missing: "Vid\u{00E9}o manquante",
    video_add: "Ajouter la vid\u{00E9}o",
    sheet_history: "Historique",
    sheet_back: "Retour",
    form_save: "Enregistrer",
    form_cancel: "Annuler",
    form_poster_upload: "Envoyer une image",
    form_poster_link: "ou coller le lien d\u{2019}une image",
    form_uploading: "Envoi\u{2026}",
    form_new_hint: "Quelqu\u{2019}un d\u{2019}autre devra l\u{2019}approuver avant qu\u{2019}il entre dans le classement.",
    form_propose: "Proposer",
    form_version_name: "Nom",
    form_versions_hint: "La premi\u{00E8}re s\u{2019}ouvre par d\u{00E9}faut.",
    form_add_version: "+ Ajouter une version",
    form_move_up: "Monter",
    form_remove_version: "Enlever cette version",
    saved: "Enregistr\u{00E9}",
    proposed: "Propos\u{00E9}\u{00A0}: quelqu\u{2019}un d\u{2019}autre doit maintenant l\u{2019}approuver",
    add_item: "Ajouter un {item}",
    proposals_heading: "{Items} propos\u{00E9}s",
    proposals_hint: "Chacun entre dans le classement d\u{00E8}s qu\u{2019}une autre personne que son auteur l\u{2019}approuve.",
    proposal_badge: "Propos\u{00E9}",
    proposal_yours: "C\u{2019}est ta proposition\u{00A0}: quelqu\u{2019}un d\u{2019}autre doit la relire.",
    approve: "Approuver",
    reject: "Rejeter",
    approved: "Approuv\u{00E9}",
    rejected: "Rejet\u{00E9}",
    author_anon: "anonyme \u{00B7} {tag}",
    author_you: "toi",
    history_edit: "Modifi\u{00E9}",
    history_revert: "Version de la r\u{00E9}vision {n} r\u{00E9}tablie",
    history_revert_original: "Version d\u{2019}origine r\u{00E9}tablie",
    history_propose: "Propos\u{00E9}",
    history_approve: "Approuv\u{00E9}",
    history_reject: "Rejet\u{00E9}",
    history_original: "Version d\u{2019}origine",
    history_restore: "Revenir \u{00E0} cette version",
    history_current: "Version actuelle",
    history_empty: "Aucune modification pour l\u{2019}instant.",
    field_title: "Titre",
    field_team: "\u{00C9}quipe",
    field_city: "Ville",
    field_poster: "Affiche",
    field_video: "Vid\u{00E9}o",
    field_synopsis: "Synopsis",

    error_title_required: "Le titre est obligatoire.",
    error_bad_text: "Le texte doit tenir sur une ligne de 200 caract\u{00E8}res au plus.",
    error_bad_synopsis: "Le synopsis peut faire 2000 caract\u{00E8}res au plus.",
    error_bad_url: "Les liens doivent commencer par https:// ou http://.",
    error_video_url_required: "Chaque version a besoin d\u{2019}un lien.",
    error_video_label_required: "Avec plusieurs versions, chacune a besoin d\u{2019}un nom.",
    error_too_many_videos: "10 versions au plus.",
    error_unchanged: "Rien n\u{2019}a chang\u{00E9}.",
    error_unknown_film: "Cet \u{00E9}l\u{00E9}ment n\u{2019}existe plus.",
    error_not_pending: "Cette proposition a d\u{00E9}j\u{00E0} \u{00E9}t\u{00E9} relue.",
    error_own_proposal: "Tu ne peux pas relire ta propre proposition.",
    error_rate_limited: "Trop de modifications pour l\u{2019}instant\u{00A0}: r\u{00E9}essaie dans une heure.",
    error_read_only: "Ce site ne prend pas de modifications.",
    error_banned: "Ton compte est suspendu.",
    error_too_large: "L\u{2019}image est trop grande.",
    error_not_image: "Ce fichier n\u{2019}est pas une image lisible par le navigateur.",
    error_other: "Quelque chose n\u{2019}a pas march\u{00E9}\u{00A0}: r\u{00E9}essaie.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn say_fills_arguments_and_nouns() {
        set_speech(Lang::En, "film", "films");
        assert_eq!(say(EN.select_count, &[("n", &3)]), "Done \u{00B7} 3 films");
        assert_eq!(say(EN.stats_voted_on, &[]), "films with votes");
    }
}

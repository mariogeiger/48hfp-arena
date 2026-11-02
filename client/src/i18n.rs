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
    pub nav_select: &'static str,
    pub nav_compare: &'static str,
    pub nav_board: &'static str,
    pub nav_more: &'static str,

    // Select
    pub select_prompt: &'static str,
    pub select_search: &'static str,
    pub select_all: &'static str,
    pub deselect_all: &'static str,
    pub select_too_few: &'static str,
    pub select_count: &'static str,

    // Compare
    pub done_none_heading: &'static str,
    pub done_none_text: &'static str,
    pub done_heading: &'static str,
    pub done_focus_text: &'static str,
    pub done_focus_action: &'static str,
    pub done_all_text: &'static str,
    pub done_all_action: &'static str,
    pub pick_left: &'static str,
    pub pick_right: &'static str,
    pub comparisons_made: &'static str,
    pub undo: &'static str,
    pub skip: &'static str,
    pub compare_against: &'static str,
    pub random_pairs: &'static str,
    pub skip_unknown: &'static str,
    pub open_link: &'static str,

    // Leaderboard
    pub board_title: &'static str,
    pub board_subtitle: &'static str,
    pub board_export: &'static str,
    pub board_unranked: &'static str,
    pub board_record: &'static str,
    pub overtook: &'static str,

    // Live events
    pub live_vote: &'static str,
    pub live_unvote: &'static str,
    pub live_reset: &'static str,
    pub live_edited: &'static str,
    pub live_proposed: &'static str,
    pub live_approved: &'static str,
    pub live_rejected: &'static str,

    // Overlays
    pub welcome_heading: &'static str,
    pub welcome_text: &'static str,
    pub welcome_promise: &'static str,
    pub welcome_refuse: &'static str,
    pub banned_heading: &'static str,
    pub banned_text: &'static str,

    // More
    pub more_title: &'static str,
    pub more_subtitle: &'static str,
    pub how_it_works: &'static str,
    pub sim_votes: &'static str,
    pub sim_pause: &'static str,
    pub sim_play: &'static str,
    pub sim_step: &'static str,
    pub sim_reset: &'static str,
    pub sim_row_beats_col: &'static str,
    pub sim_score: &'static str,
    pub stats_heading: &'static str,
    pub stats_votes: &'static str,
    pub stats_voters: &'static str,
    pub stats_voted_on: &'static str,
    pub reset_confirm: &'static str,
    pub reset_button: &'static str,
    pub contrib_heading: &'static str,
    pub contrib_none: &'static str,
    pub contrib_you: &'static str,
    pub contrib_voter: &'static str,
    pub contrib_count: &'static str,
    pub contrib_coverage: &'static str,
    pub matrix_yours: &'static str,
    pub matrix_yours_hint: &'static str,
    pub matrix_yours_empty: &'static str,
    pub matrix_global: &'static str,
    pub matrix_global_hint: &'static str,
    pub matrix_global_empty: &'static str,
    pub matrix_versus: &'static str,
    pub matrix_no_votes: &'static str,
    pub matrix_residual: &'static str,
    pub matrix_win: &'static str,
    pub matrix_loss: &'static str,

    // Wiki
    pub sheet_close: &'static str,
    pub sheet_edit: &'static str,
    pub sheet_history: &'static str,
    pub sheet_back: &'static str,
    pub form_save: &'static str,
    pub form_cancel: &'static str,
    pub form_poster_upload: &'static str,
    pub form_poster_link: &'static str,
    pub form_uploading: &'static str,
    pub form_new_heading: &'static str,
    pub form_new_hint: &'static str,
    pub form_propose: &'static str,
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
    pub error_title_required: &'static str,
    pub error_bad_text: &'static str,
    pub error_bad_url: &'static str,
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
            "bad_url" => self.error_bad_url,
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
    nav_select: "Select {Items}",
    nav_compare: "Compare",
    nav_board: "Leaderboard",
    nav_more: "More",

    select_prompt: "Select the {items} you know, then compare them head-to-head.",
    select_search: "Search {items}...",
    select_all: "Select All",
    deselect_all: "Deselect All",
    select_too_few: "Select at least 2 {items} to compare",
    select_count: "{n} {items} selected",

    done_none_heading: "Select {items} first",
    done_none_text: "Go back and pick at least 2 {items} you know.",
    done_heading: "All done!",
    done_focus_text: "You've compared this {item} against all others.",
    done_focus_action: "Compare All",
    done_all_text: "You've compared all possible pairs. ({n} votes cast)",
    done_all_action: "View Leaderboard",
    pick_left: "Pick left",
    pick_right: "Pick right",
    comparisons_made: "{n} comparisons made",
    undo: "Undo",
    skip: "Skip",
    compare_against: "Compare against:",
    random_pairs: "Random pairs",
    skip_unknown: "Skip this one",
    open_link: "Open",

    board_title: "Leaderboard",
    board_subtitle: "Bradley\u{2013}Terry rankings based on all voter comparisons",
    board_export: "Export as CSV",
    board_unranked: "Not enough votes yet to rank these {items}: they are shown in random order.",
    board_record: "{w}W - {l}L",
    overtook: "<strong>{a}</strong> overtook <strong>{b}</strong> \u{2192} #{n}",

    live_vote: "Voted: {a} over {b}",
    live_unvote: "Unvoted: {a} vs {b}",
    live_reset: "Someone reset their votes",
    live_edited: "{title} was edited",
    live_proposed: "New proposal: {title}",
    live_approved: "{title} joins the ranking",
    live_rejected: "The proposal {title} was rejected",

    welcome_heading: "Welcome!",
    welcome_text: "Please only vote on pairs you have real opinions about. Your honest comparisons make the ranking meaningful for everyone.",
    welcome_promise: "I promise",
    welcome_refuse: "I can't promise that",
    banned_heading: "Account Suspended",
    banned_text: "Your account has been suspended for violating the voting rules. If you believe this is a mistake, please contact the site administrator.",

    more_title: "More",
    more_subtitle: "Voting stats, your votes matrix, and global results",
    how_it_works: "How It Works",
    sim_votes: "{n} votes",
    sim_pause: "\u{23F8} Pause",
    sim_play: "\u{25B6} Play",
    sim_step: "+1 vote",
    sim_reset: "Reset",
    sim_row_beats_col: "row beats col \u{2192}",
    sim_score: "Score",
    stats_heading: "Voting Stats",
    stats_votes: "Total Votes",
    stats_voters: "Voters",
    stats_voted_on: "{Items} Voted On",
    reset_confirm: "This will permanently delete all your votes. Are you sure?",
    reset_button: "Reset All My Votes",
    contrib_heading: "Voter Contributions",
    contrib_none: "No contributions yet.",
    contrib_you: "You",
    contrib_voter: "Voter {n}",
    contrib_count: "<span class=\"contrib-num\">{v}</span> votes on <span class=\"contrib-num\">{f}</span> {items}",
    contrib_coverage: "{v} / {p} possible pairs = {pct} coverage",
    matrix_yours: "Your Vote Matrix",
    matrix_yours_hint: "Tap empty cell to vote (row wins). Tap filled cell to remove vote.",
    matrix_yours_empty: "No votes yet. Start comparing!",
    matrix_global: "Global Win Matrix",
    matrix_global_hint: "Color = surprise: green = wins more than model expects, red = wins less. Hover for details.",
    matrix_global_empty: "No data yet.",
    matrix_versus: "{a} vs {b}",
    matrix_no_votes: "{a} vs {b}: no votes",
    matrix_residual: "{a} vs {b}\nobserved {o}% / model {m}% ({r}%)",
    matrix_win: "W",
    matrix_loss: "L",

    sheet_close: "Close",
    sheet_edit: "Edit",
    sheet_history: "History",
    sheet_back: "Back",
    form_save: "Save",
    form_cancel: "Cancel",
    form_poster_upload: "Upload an image",
    form_poster_link: "or paste a link to an image",
    form_uploading: "Uploading\u{2026}",
    form_new_heading: "Add a {item}",
    form_new_hint: "Someone else will have to approve it before it enters the ranking.",
    form_propose: "Propose",
    saved: "Saved",
    proposed: "Proposed: someone else now has to approve it",
    add_item: "+ Add a {item}",
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
    error_title_required: "The title is required.",
    error_bad_text: "Text must be one line of at most 200 characters.",
    error_bad_url: "Links must start with https:// or http://.",
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
    nav_select: "Choisir",
    nav_compare: "Comparer",
    nav_board: "Classement",
    nav_more: "Plus",

    select_prompt: "Choisis les {items} que tu connais, puis compare-les deux par deux.",
    select_search: "Chercher\u{2026}",
    select_all: "Tout choisir",
    deselect_all: "Tout enlever",
    select_too_few: "Choisis au moins 2 {items} pour comparer",
    select_count: "{n} {items} choisis",

    done_none_heading: "Choisis d\u{2019}abord des {items}",
    done_none_text: "Reviens en arri\u{00E8}re et choisis au moins 2 {items} que tu connais.",
    done_heading: "C\u{2019}est fini\u{00A0}!",
    done_focus_text: "Tu as compar\u{00E9} ce {item} \u{00E0} tous les autres.",
    done_focus_action: "Tout comparer",
    done_all_text: "Tu as compar\u{00E9} toutes les paires possibles. ({n} votes)",
    done_all_action: "Voir le classement",
    pick_left: "Choisir \u{00E0} gauche",
    pick_right: "Choisir \u{00E0} droite",
    comparisons_made: "{n} comparaisons faites",
    undo: "Annuler",
    skip: "Passer",
    compare_against: "Comparer \u{00E0}\u{00A0}:",
    random_pairs: "Paires au hasard",
    skip_unknown: "Pas vu",
    open_link: "Ouvrir",

    board_title: "Classement",
    board_subtitle: "Classement de Bradley\u{2013}Terry, tir\u{00E9} de toutes les comparaisons",
    board_export: "Exporter en CSV",
    board_unranked: "Pas encore assez de votes pour classer ces {items}\u{00A0}: ils sont affich\u{00E9}s dans un ordre al\u{00E9}atoire.",
    board_record: "{w}V - {l}D",
    overtook: "<strong>{a}</strong> d\u{00E9}passe <strong>{b}</strong> \u{2192} n\u{00B0}{n}",

    live_vote: "Vote\u{00A0}: {a} plut\u{00F4}t que {b}",
    live_unvote: "Vote retir\u{00E9}\u{00A0}: {a} contre {b}",
    live_reset: "Quelqu\u{2019}un a effac\u{00E9} ses votes",
    live_edited: "{title} a \u{00E9}t\u{00E9} modifi\u{00E9}",
    live_proposed: "Nouvelle proposition\u{00A0}: {title}",
    live_approved: "{title} entre dans le classement",
    live_rejected: "La proposition {title} a \u{00E9}t\u{00E9} rejet\u{00E9}e",

    welcome_heading: "Bienvenue\u{00A0}!",
    welcome_text: "Ne vote que sur des paires que tu connais vraiment. Des comparaisons honn\u{00EA}tes rendent le classement juste pour tout le monde.",
    welcome_promise: "Promis",
    welcome_refuse: "Je ne peux pas promettre",
    banned_heading: "Compte suspendu",
    banned_text: "Ton compte a \u{00E9}t\u{00E9} suspendu pour non-respect des r\u{00E8}gles de vote. Si c\u{2019}est une erreur, contacte l\u{2019}administrateur du site.",

    more_title: "Plus",
    more_subtitle: "Statistiques, ta matrice de votes et les r\u{00E9}sultats globaux",
    how_it_works: "Comment \u{00E7}a marche",
    sim_votes: "{n} votes",
    sim_pause: "\u{23F8} Pause",
    sim_play: "\u{25B6} Lecture",
    sim_step: "+1 vote",
    sim_reset: "Recommencer",
    sim_row_beats_col: "la ligne bat la colonne \u{2192}",
    sim_score: "Score",
    stats_heading: "Statistiques",
    stats_votes: "Votes",
    stats_voters: "Votants",
    stats_voted_on: "{Items} avec des votes",
    reset_confirm: "Tous tes votes seront effac\u{00E9}s d\u{00E9}finitivement. Tu es s\u{00FB}r\u{00B7}e\u{00A0}?",
    reset_button: "Effacer tous mes votes",
    contrib_heading: "Contributions",
    contrib_none: "Aucune contribution pour l\u{2019}instant.",
    contrib_you: "Toi",
    contrib_voter: "Votant {n}",
    contrib_count: "<span class=\"contrib-num\">{v}</span> votes sur <span class=\"contrib-num\">{f}</span> {items}",
    contrib_coverage: "{v} / {p} paires possibles = {pct} de couverture",
    matrix_yours: "Ta matrice de votes",
    matrix_yours_hint: "Touche une case vide pour voter (la ligne gagne). Touche une case remplie pour retirer le vote.",
    matrix_yours_empty: "Pas encore de vote. Commence \u{00E0} comparer\u{00A0}!",
    matrix_global: "Matrice globale des victoires",
    matrix_global_hint: "Couleur = surprise\u{00A0}: vert = gagne plus que le mod\u{00E8}le ne pr\u{00E9}voit, rouge = moins. Survole pour le d\u{00E9}tail.",
    matrix_global_empty: "Pas encore de donn\u{00E9}es.",
    matrix_versus: "{a} contre {b}",
    matrix_no_votes: "{a} contre {b}\u{00A0}: aucun vote",
    matrix_residual: "{a} contre {b}\nobserv\u{00E9} {o}\u{00A0}% / mod\u{00E8}le {m}\u{00A0}% ({r}\u{00A0}%)",
    matrix_win: "V",
    matrix_loss: "D",

    sheet_close: "Fermer",
    sheet_edit: "Modifier",
    sheet_history: "Historique",
    sheet_back: "Retour",
    form_save: "Enregistrer",
    form_cancel: "Annuler",
    form_poster_upload: "Envoyer une image",
    form_poster_link: "ou coller le lien d\u{2019}une image",
    form_uploading: "Envoi\u{2026}",
    form_new_heading: "Ajouter un {item}",
    form_new_hint: "Quelqu\u{2019}un d\u{2019}autre devra l\u{2019}approuver avant qu\u{2019}il entre dans le classement.",
    form_propose: "Proposer",
    saved: "Enregistr\u{00E9}",
    proposed: "Propos\u{00E9}\u{00A0}: quelqu\u{2019}un d\u{2019}autre doit maintenant l\u{2019}approuver",
    add_item: "+ Ajouter un {item}",
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
    error_title_required: "Le titre est obligatoire.",
    error_bad_text: "Le texte doit tenir sur une ligne de 200 caract\u{00E8}res au plus.",
    error_bad_url: "Les liens doivent commencer par https:// ou http://.",
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
        assert_eq!(say(EN.select_count, &[("n", &3)]), "3 films selected");
        assert_eq!(say(EN.stats_voted_on, &[]), "Films Voted On");
    }
}

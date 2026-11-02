//! "How it works": the ranking's mathematics, in the site's language, with a
//! live simulation of the Bradley–Terry model.

use crate::i18n::{Lang, fill, lang, say, t};
use crate::state::SiteConfig;
use leptos::prelude::*;

#[component]
pub fn HowItWorks() -> impl IntoView {
    let cfg = SiteConfig::get();
    let eligibility = match lang() {
        Lang::En => {
            "A {item} appears on the leaderboard once it has at least {votes} comparisons from at least {voters} different voters. All votes from all users are aggregated into one global ranking."
        }
        Lang::Fr => {
            "Un {item} entre dans le classement d\u{00E8}s qu\u{2019}il compte au moins {votes} comparaisons venant d\u{2019}au moins {voters} votants diff\u{00E9}rents. Tous les votes de tout le monde forment un seul classement global."
        }
    };
    let eligibility = say(
        eligibility,
        &[("votes", &cfg.min_votes), ("voters", &cfg.min_voters)],
    );
    view! {
        <div class="about-section">
            <h3>{t().how_it_works}</h3>
            <div class="about-content">
                {match lang() {
                    Lang::En => english().into_any(),
                    Lang::Fr => french().into_any(),
                }}
                <h4>{match lang() { Lang::En => "Leaderboard Eligibility", Lang::Fr => "Entr\u{00E9}e dans le classement" }}</h4>
                <p>{eligibility}</p>
            </div>
        </div>
    }
}

fn english() -> impl IntoView {
    view! {
        <h4>"The Bradley\u{2013}Terry Model"</h4>
        <p>
            "Each head-to-head vote feeds a "
            <a href="https://en.wikipedia.org/wiki/Bradley%E2%80%93Terry_model" target="_blank">"Bradley\u{2013}Terry model"</a>
            ", a statistical method for ranking items from pairwise comparisons. Every film "
            <var>"i"</var>
            " gets a positive strength parameter \u{03B2}"
            <sub><var>"i"</var></sub>
            ". The probability that film\u{00A0}"
            <var>"i"</var>
            " beats film\u{00A0}"
            <var>"j"</var>
            " is:"
        </p>
        <p class="math">
            "P(" <var>"i"</var> " beats " <var>"j"</var>
            ")\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            <b>"Intuition:"</b>
            " think of \u{03B2} as the \u{201C}mass\u{201D} of a film. When two films are placed on a balance, the heavier one tips the scale in its favor. A film with twice the strength of another wins roughly two-thirds of the time. When two films have equal strength, the probability is \u{00BD}\u{00A0}\u{2014} a fair coin flip."
        </p>
        <BtSimulation />

        <h4>"Estimating Strengths: the MM Algorithm"</h4>
        <p>
            "We observe wins and losses but don\u{2019}t know the true \u{03B2} values. The "
            <a href="https://en.wikipedia.org/wiki/MM_algorithm" target="_blank">"MM\u{00A0}algorithm"</a>
            " (minorization\u{2013}maximization) finds them by iterating a simple update rule until convergence:"
        </p>
        <p class="math">
            "\u{03B2}" <sub><var>"i"</var></sub> <sup>"(new)"</sup>
            "\u{00A0}=\u{00A0}"
            <var>"w" <sub>"i"</sub></var>
            "\u{2009}/\u{2009}\u{2211}" <sub><var>"j"</var> "\u{2260}" <var>"i"</var></sub>
            "\u{00A0}"
            <var>"n" <sub>"ij"</sub></var>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            "where " <var>"w" <sub>"i"</sub></var>
            " is the total number of wins for film\u{00A0}"
            <var>"i"</var>
            " across all opponents and "
            <var>"n" <sub>"ij"</sub></var>
            " is the total number of comparisons between "
            <var>"i"</var> " and\u{00A0}" <var>"j"</var> "."
        </p>
        <p>
            <b>"Intuition:"</b>
            " the numerator says \u{201C}how often does this film actually win?\u{201D} The denominator says \u{201C}how often "
            <em>"would"</em>
            " it win under the current model?\u{201D} If a film wins more than the model expects, its strength goes up; if it wins less, its strength goes down. The algorithm keeps adjusting until these two quantities balance out for every film simultaneously."
        </p>
        <p>
            "After each iteration the strengths are renormalized so their geometric mean equals\u{00A0}1 (i.e.\u{00A0}we subtract the mean of log\u{00A0}\u{03B2} values). This prevents the numbers from drifting to infinity while preserving all the ratios that matter. Films with zero wins are pinned to a near-zero score\u{00A0}(10"
            <sup>"\u{2212}6"</sup>
            "). Convergence is declared when the maximum relative change across all \u{03B2} values drops below\u{00A0}10"
            <sup>"\u{2212}8"</sup> "."
        </p>

        <h4>"Display Score"</h4>
        <p class="math">
            "score\u{00A0}=\u{00A0}round(500\u{00A0}\u{00D7}\u{00A0}log"
            <sub>"2"</sub> "(1\u{00A0}+\u{00A0}\u{03B2}))"
        </p>
        <p>
            <b>"Intuition:"</b>
            " raw \u{03B2} values can span many orders of magnitude, making them hard to compare at a glance. The logarithm compresses this range into a human-friendly scale. The +1 ensures a film with \u{03B2}\u{00A0}=\u{00A0}0 maps to score\u{00A0}0, and the 500 multiplier spreads the values into a comfortable range (roughly 0\u{2013}5000+). Every doubling of a film\u{2019}s strength adds 500\u{00A0}points."
        </p>

        <h4>"Smart Pair Selection: D-Optimal Design"</h4>
        <p>
            "Pairs are not presented randomly. The system uses "
            <a href="https://en.wikipedia.org/wiki/Optimal_design#D-optimality" target="_blank">"D-optimal experimental design"</a>
            " to pick the most informative matchup for each vote."
        </p>
        <p>
            "First, the server builds the "
            <a href="https://en.wikipedia.org/wiki/Fisher_information" target="_blank">"Fisher information matrix"</a>
            "\u{00A0}" <b>"F"</b>
            ". For each pair (" <var>"i"</var> ",\u{00A0}" <var>"j"</var>
            ") that has been compared, the information contributed is:"
        </p>
        <p class="math">
            "I" <sub><var>"ij"</var></sub>
            "\u{00A0}=\u{00A0}"
            <var>"n" <sub>"ij"</sub></var>
            "\u{00A0}\u{22C5}\u{00A0}"
            <var>"p" <sub>"ij"</sub></var>
            "\u{00A0}\u{22C5}\u{00A0}(1\u{00A0}\u{2212}\u{00A0}"
            <var>"p" <sub>"ij"</sub></var>
            ")\u{2003}where\u{2003}"
            <var>"p" <sub>"ij"</sub></var>
            "\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            <b>"Intuition:"</b>
            " " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
            ") is the variance of a Bernoulli trial. It peaks at "
            <var>"p"</var> "\u{00A0}=\u{00A0}\u{00BD} (evenly matched films) and vanishes when one film always wins. A matchup between closely-ranked films tells you more than a blowout. Multiplying by the number of comparisons "
            <var>"n" <sub>"ij"</sub></var>
            " accounts for how much data we already have."
        </p>
        <p>
            "These contributions fill a symmetric matrix "
            <b>"F"</b>
            " (with I" <sub><var>"ij"</var></sub>
            " added to the diagonal entries " <b>"F"</b> "[" <var>"i"</var> "," <var>"i"</var>
            "] and " <b>"F"</b> "[" <var>"j"</var> "," <var>"j"</var>
            "], and subtracted from the off-diagonal " <b>"F"</b> "[" <var>"i"</var> "," <var>"j"</var>
            "]). The diagonal is regularized by adding a small prior\u{00A0}(0.25) for numerical stability. Then the matrix is inverted via Gauss\u{2013}Jordan elimination with partial pivoting to obtain "
            <b>"F"</b> <sup>"\u{2212}1"</sup> "."
        </p>
        <p>"For each candidate pair (a, b) the D-optimal score is:"</p>
        <p class="math">
            "d" <sub><var>"ab"</var></sub>
            "\u{00A0}=\u{00A0}"
            <var>"p" <sub>"ab"</sub></var>
            "(1\u{00A0}\u{2212}\u{00A0}"
            <var>"p" <sub>"ab"</sub></var>
            ")\u{00A0}\u{22C5}\u{00A0}("
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
            "]\u{00A0}+\u{00A0}"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
            "]\u{00A0}\u{2212}\u{00A0}2"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var> "])"
        </p>
        <p>
            <b>"Intuition:"</b> " "
            <b>"F"</b> <sup>"\u{2212}1"</sup>
            " encodes how uncertain we are about each film\u{2019}s strength. The expression "
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
            "]\u{00A0}+\u{00A0}"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
            "]\u{00A0}\u{2212}\u{00A0}2"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var>
            "] is the variance of the difference \u{03B2}" <sub><var>"a"</var></sub>
            "\u{00A0}\u{2212}\u{00A0}\u{03B2}" <sub><var>"b"</var></sub>
            ". Multiplying by " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
            ") weights this by how much a single new comparison can actually reduce that uncertainty. So the system picks the matchup where one more vote would shrink the overall uncertainty the most."
        </p>
        <p>
            "These D-optimal scores are converted to sampling probabilities with a softmax (inverse temperature\u{00A0}=\u{00A0}5), so the best pair is strongly favored but not deterministically chosen\u{00A0}\u{2014} preserving some exploration."
        </p>
    }
}

fn french() -> impl IntoView {
    view! {
        <h4>"Le mod\u{00E8}le de Bradley\u{2013}Terry"</h4>
        <p>
            "Chaque duel alimente un "
            <a href="https://fr.wikipedia.org/wiki/Mod%C3%A8le_de_Bradley-Terry" target="_blank">"mod\u{00E8}le de Bradley\u{2013}Terry"</a>
            ", une m\u{00E9}thode statistique qui classe des \u{00E9}l\u{00E9}ments \u{00E0} partir de comparaisons par paires. Chaque film "
            <var>"i"</var>
            " re\u{00E7}oit une force positive \u{03B2}"
            <sub><var>"i"</var></sub>
            ". La probabilit\u{00E9} que le film\u{00A0}"
            <var>"i"</var>
            " batte le film\u{00A0}"
            <var>"j"</var>
            " est\u{00A0}:"
        </p>
        <p class="math">
            "P(" <var>"i"</var> " bat " <var>"j"</var>
            ")\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            <b>"Intuition\u{00A0}:"</b>
            " voyez \u{03B2} comme la \u{00AB}\u{00A0}masse\u{00A0}\u{00BB} d\u{2019}un film. Sur une balance, le plus lourd des deux l\u{2019}emporte. Un film deux fois plus fort qu\u{2019}un autre gagne environ deux fois sur trois. \u{00C0} force \u{00E9}gale, la probabilit\u{00E9} vaut \u{00BD}\u{00A0}\u{2014} pile ou face."
        </p>
        <BtSimulation />

        <h4>"Estimer les forces\u{00A0}: l\u{2019}algorithme MM"</h4>
        <p>
            "On observe des victoires et des d\u{00E9}faites, mais pas les vraies valeurs de \u{03B2}. L\u{2019}"
            <a href="https://en.wikipedia.org/wiki/MM_algorithm" target="_blank">"algorithme\u{00A0}MM"</a>
            " (minoration\u{2013}maximisation) les trouve en r\u{00E9}p\u{00E9}tant une r\u{00E8}gle simple jusqu\u{2019}\u{00E0} convergence\u{00A0}:"
        </p>
        <p class="math">
            "\u{03B2}" <sub><var>"i"</var></sub> <sup>"(nouveau)"</sup>
            "\u{00A0}=\u{00A0}"
            <var>"w" <sub>"i"</sub></var>
            "\u{2009}/\u{2009}\u{2211}" <sub><var>"j"</var> "\u{2260}" <var>"i"</var></sub>
            "\u{00A0}"
            <var>"n" <sub>"ij"</sub></var>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            "o\u{00F9} " <var>"w" <sub>"i"</sub></var>
            " est le nombre total de victoires du film\u{00A0}"
            <var>"i"</var>
            " contre tous ses adversaires et "
            <var>"n" <sub>"ij"</sub></var>
            " le nombre total de comparaisons entre "
            <var>"i"</var> " et\u{00A0}" <var>"j"</var> "."
        </p>
        <p>
            <b>"Intuition\u{00A0}:"</b>
            " le num\u{00E9}rateur dit \u{00AB}\u{00A0}combien de fois ce film gagne-t-il vraiment\u{00A0}?\u{00A0}\u{00BB} Le d\u{00E9}nominateur dit \u{00AB}\u{00A0}combien de fois "
            <em>"devrait"</em>
            "-il gagner selon le mod\u{00E8}le actuel\u{00A0}?\u{00A0}\u{00BB} Un film qui gagne plus que pr\u{00E9}vu voit sa force monter\u{00A0}; s\u{2019}il gagne moins, elle baisse. L\u{2019}algorithme ajuste jusqu\u{2019}\u{00E0} ce que ces deux quantit\u{00E9}s s\u{2019}\u{00E9}quilibrent pour tous les films \u{00E0} la fois."
        </p>
        <p>
            "Apr\u{00E8}s chaque it\u{00E9}ration, les forces sont renormalis\u{00E9}es pour que leur moyenne g\u{00E9}om\u{00E9}trique vaille\u{00A0}1 (on soustrait la moyenne des log\u{00A0}\u{03B2}). Les nombres ne d\u{00E9}rivent ainsi pas vers l\u{2019}infini, et tous les rapports qui comptent sont pr\u{00E9}serv\u{00E9}s. Un film sans victoire est fix\u{00E9} \u{00E0} un score quasi nul\u{00A0}(10"
            <sup>"\u{2212}6"</sup>
            "). La convergence est atteinte quand la plus grande variation relative des \u{03B2} passe sous\u{00A0}10"
            <sup>"\u{2212}8"</sup> "."
        </p>

        <h4>"Score affich\u{00E9}"</h4>
        <p class="math">
            "score\u{00A0}=\u{00A0}round(500\u{00A0}\u{00D7}\u{00A0}log"
            <sub>"2"</sub> "(1\u{00A0}+\u{00A0}\u{03B2}))"
        </p>
        <p>
            <b>"Intuition\u{00A0}:"</b>
            " les \u{03B2} bruts peuvent couvrir plusieurs ordres de grandeur, ce qui les rend durs \u{00E0} comparer d\u{2019}un coup d\u{2019}\u{0153}il. Le logarithme ram\u{00E8}ne cette \u{00E9}tendue \u{00E0} une \u{00E9}chelle lisible. Le +1 envoie un film de \u{03B2}\u{00A0}=\u{00A0}0 sur le score\u{00A0}0, et le facteur 500 \u{00E9}tale les valeurs sur une plage confortable (environ 0\u{2013}5000+). Chaque doublement de la force d\u{2019}un film ajoute 500\u{00A0}points."
        </p>

        <h4>"Choix des paires\u{00A0}: plan D-optimal"</h4>
        <p>
            "Les paires ne sont pas tir\u{00E9}es au hasard. Le syst\u{00E8}me utilise un "
            <a href="https://en.wikipedia.org/wiki/Optimal_design#D-optimality" target="_blank">"plan d\u{2019}exp\u{00E9}rience D-optimal"</a>
            " pour proposer le duel le plus informatif \u{00E0} chaque vote."
        </p>
        <p>
            "D\u{2019}abord, le serveur construit la "
            <a href="https://fr.wikipedia.org/wiki/Information_de_Fisher" target="_blank">"matrice d\u{2019}information de Fisher"</a>
            "\u{00A0}" <b>"F"</b>
            ". Pour chaque paire (" <var>"i"</var> ",\u{00A0}" <var>"j"</var>
            ") d\u{00E9}j\u{00E0} compar\u{00E9}e, l\u{2019}information apport\u{00E9}e est\u{00A0}:"
        </p>
        <p class="math">
            "I" <sub><var>"ij"</var></sub>
            "\u{00A0}=\u{00A0}"
            <var>"n" <sub>"ij"</sub></var>
            "\u{00A0}\u{22C5}\u{00A0}"
            <var>"p" <sub>"ij"</sub></var>
            "\u{00A0}\u{22C5}\u{00A0}(1\u{00A0}\u{2212}\u{00A0}"
            <var>"p" <sub>"ij"</sub></var>
            ")\u{2003}o\u{00F9}\u{2003}"
            <var>"p" <sub>"ij"</sub></var>
            "\u{00A0}=\u{00A0}\u{03B2}" <sub><var>"i"</var></sub>
            "\u{2009}/\u{2009}(\u{03B2}" <sub><var>"i"</var></sub>
            "\u{00A0}+\u{00A0}\u{03B2}" <sub><var>"j"</var></sub> ")"
        </p>
        <p>
            <b>"Intuition\u{00A0}:"</b>
            " " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
            ") est la variance d\u{2019}une \u{00E9}preuve de Bernoulli. Elle est maximale pour "
            <var>"p"</var> "\u{00A0}=\u{00A0}\u{00BD} (films de force \u{00E9}gale) et s\u{2019}annule quand un film gagne toujours. Un duel serr\u{00E9} en apprend plus qu\u{2019}une d\u{00E9}b\u{00E2}cle. Multiplier par le nombre de comparaisons "
            <var>"n" <sub>"ij"</sub></var>
            " tient compte des donn\u{00E9}es d\u{00E9}j\u{00E0} r\u{00E9}unies."
        </p>
        <p>
            "Ces contributions remplissent une matrice sym\u{00E9}trique "
            <b>"F"</b>
            " (I" <sub><var>"ij"</var></sub>
            " est ajout\u{00E9} aux termes diagonaux " <b>"F"</b> "[" <var>"i"</var> "," <var>"i"</var>
            "] et " <b>"F"</b> "[" <var>"j"</var> "," <var>"j"</var>
            "], et retranch\u{00E9} du terme hors diagonale " <b>"F"</b> "[" <var>"i"</var> "," <var>"j"</var>
            "]). La diagonale est r\u{00E9}gularis\u{00E9}e par un petit a priori\u{00A0}(0,25) pour la stabilit\u{00E9} num\u{00E9}rique. La matrice est ensuite invers\u{00E9}e par \u{00E9}limination de Gauss\u{2013}Jordan avec pivot partiel, ce qui donne "
            <b>"F"</b> <sup>"\u{2212}1"</sup> "."
        </p>
        <p>"Pour chaque paire candidate (a, b), le score D-optimal est\u{00A0}:"</p>
        <p class="math">
            "d" <sub><var>"ab"</var></sub>
            "\u{00A0}=\u{00A0}"
            <var>"p" <sub>"ab"</sub></var>
            "(1\u{00A0}\u{2212}\u{00A0}"
            <var>"p" <sub>"ab"</sub></var>
            ")\u{00A0}\u{22C5}\u{00A0}("
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
            "]\u{00A0}+\u{00A0}"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
            "]\u{00A0}\u{2212}\u{00A0}2"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var> "])"
        </p>
        <p>
            <b>"Intuition\u{00A0}:"</b> " "
            <b>"F"</b> <sup>"\u{2212}1"</sup>
            " mesure notre incertitude sur la force de chaque film. L\u{2019}expression "
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"a"</var>
            "]\u{00A0}+\u{00A0}"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"b"</var> "," <var>"b"</var>
            "]\u{00A0}\u{2212}\u{00A0}2"
            <b>"F"</b> <sup>"\u{2212}1"</sup> "[" <var>"a"</var> "," <var>"b"</var>
            "] est la variance de la diff\u{00E9}rence \u{03B2}" <sub><var>"a"</var></sub>
            "\u{00A0}\u{2212}\u{00A0}\u{03B2}" <sub><var>"b"</var></sub>
            ". La multiplier par " <var>"p"</var> "(1\u{2212}" <var>"p"</var>
            ") la pond\u{00E8}re par ce qu\u{2019}une seule nouvelle comparaison peut r\u{00E9}ellement r\u{00E9}duire de cette incertitude. Le syst\u{00E8}me choisit donc le duel o\u{00F9} un vote de plus r\u{00E9}duirait le plus l\u{2019}incertitude globale."
        </p>
        <p>
            "Ces scores D-optimaux deviennent des probabilit\u{00E9}s de tirage par un softmax (temp\u{00E9}rature inverse\u{00A0}=\u{00A0}5)\u{00A0}: la meilleure paire est nettement favoris\u{00E9}e sans \u{00EA}tre choisie \u{00E0} coup s\u{00FB}r\u{00A0}\u{2014} il reste de l\u{2019}exploration."
        </p>
    }
}

/// Mini Bradley-Terry simulation: shows votes arriving, filling a win matrix,
/// and β scores converging via the MM algorithm.
#[component]
fn BtSimulation() -> impl IntoView {
    use wasm_bindgen::JsCast;

    const N: usize = 4;
    const LABELS: [&str; N] = ["A", "B", "C", "D"];
    // Hidden "true" strengths used to generate votes probabilistically
    const TRUE_BETA: [f64; N] = [4.0, 2.0, 1.0, 0.5];

    // wins[i][j] = number of times i beat j
    let wins: RwSignal<[[u32; N]; N]> = RwSignal::new([[0; N]; N]);
    // Current estimated β values
    let betas: RwSignal<[f64; N]> = RwSignal::new([1.0; N]);
    // Step counter for display
    let step: RwSignal<u32> = RwSignal::new(0);
    // Last vote highlight (winner, loser)
    let last_vote: RwSignal<Option<(usize, usize)>> = RwSignal::new(None);
    // Timer handle
    let timer_handle: RwSignal<Option<i32>> = RwSignal::new(None);
    let playing = RwSignal::new(true);

    // Run MM algorithm on current wins, return new betas
    let run_mm = move |w: [[u32; N]; N]| -> [f64; N] {
        let mut scores = [1.0_f64; N];
        // Check which films have wins
        let has_wins: Vec<bool> = (0..N).map(|i| (0..N).any(|j| w[i][j] > 0)).collect();
        let has_comparisons: Vec<bool> = (0..N)
            .map(|i| (0..N).any(|j| w[i][j] + w[j][i] > 0))
            .collect();

        for i in 0..N {
            if has_comparisons[i] && !has_wins[i] {
                scores[i] = 1e-6;
            }
        }

        for _ in 0..200 {
            let old = scores;
            let mut max_rel = 0.0_f64;

            for i in 0..N {
                if !has_wins[i] {
                    continue;
                }
                let w_i: f64 = (0..N).map(|j| w[i][j] as f64).sum();
                let denom: f64 = (0..N)
                    .filter(|&j| j != i)
                    .filter_map(|j| {
                        let n_ij = (w[i][j] + w[j][i]) as f64;
                        if n_ij > 0.0 {
                            Some(n_ij / (old[i] + old[j]))
                        } else {
                            None
                        }
                    })
                    .sum();
                if denom > 0.0 {
                    scores[i] = w_i / denom;
                    let rel = (scores[i] - old[i]).abs() / old[i];
                    max_rel = max_rel.max(rel);
                }
            }

            // Normalize: geometric mean of ranked films = 1
            let ranked: Vec<usize> = (0..N).filter(|&i| has_wins[i]).collect();
            if !ranked.is_empty() {
                let log_mean =
                    ranked.iter().map(|&i| scores[i].ln()).sum::<f64>() / ranked.len() as f64;
                let scale = (-log_mean).exp();
                for &i in &ranked {
                    scores[i] *= scale;
                }
            }

            if max_rel < 1e-8 {
                break;
            }
        }
        scores
    };

    // Add one random vote based on true strengths
    let add_vote = move || {
        // Pick a random pair
        let window = web_sys::window().unwrap();
        let crypto = window.crypto().unwrap();
        let mut buf = [0u8; 4];
        crypto.get_random_values_with_u8_array(&mut buf).ok();
        let r = u32::from_le_bytes(buf);

        // Pick pair (i, j) where i < j
        let pair_count = N * (N - 1) / 2;
        let pair_idx = (r as usize) % pair_count;
        let mut idx = 0;
        let mut pi = 0;
        let mut pj = 1;
        for i in 0..N {
            for j in (i + 1)..N {
                if idx == pair_idx {
                    pi = i;
                    pj = j;
                }
                idx += 1;
            }
        }

        // Determine winner based on true strengths
        let p_i = TRUE_BETA[pi] / (TRUE_BETA[pi] + TRUE_BETA[pj]);
        let mut buf2 = [0u8; 4];
        crypto.get_random_values_with_u8_array(&mut buf2).ok();
        let r2 = u32::from_le_bytes(buf2) as f64 / u32::MAX as f64;
        let (winner, loser) = if r2 < p_i { (pi, pj) } else { (pj, pi) };

        wins.update(|w| w[winner][loser] += 1);
        let new_betas = run_mm(wins.get_untracked());
        betas.set(new_betas);
        step.update(|s| *s += 1);
        last_vote.set(Some((winner, loser)));
    };

    let start_timer = move || {
        let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            add_vote();
        });
        let window = web_sys::window().unwrap();
        let id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                600,
            )
            .unwrap();
        cb.forget();
        timer_handle.set(Some(id));
    };

    let stop_timer = move || {
        if let Some(id) = timer_handle.get_untracked() {
            web_sys::window().unwrap().clear_interval_with_handle(id);
            timer_handle.set(None);
        }
    };

    let reset = move |_| {
        stop_timer();
        wins.set([[0; N]; N]);
        betas.set([1.0; N]);
        step.set(0);
        last_vote.set(None);
        if playing.get_untracked() {
            start_timer();
        }
    };

    let toggle_play = move |_| {
        if playing.get_untracked() {
            stop_timer();
            playing.set(false);
        } else {
            playing.set(true);
            start_timer();
        }
    };

    let step_one = move |_| {
        add_vote();
    };

    // Auto-start
    start_timer();

    // Draw on canvas whenever state changes
    Effect::new(move || {
        let w = wins.get();
        let b = betas.get();
        let _s = step.get();
        let lv = last_vote.get();

        let doc = web_sys::window().unwrap().document().unwrap();
        let Some(canvas) = doc.get_element_by_id("bt-sim-canvas") else {
            return;
        };
        let canvas: web_sys::HtmlCanvasElement = canvas.unchecked_into();
        let dpr = web_sys::window().unwrap().device_pixel_ratio();
        let css_w = 340.0;
        let css_h = 170.0;
        canvas.set_width((css_w * dpr) as u32);
        canvas.set_height((css_h * dpr) as u32);

        let ctx: web_sys::CanvasRenderingContext2d =
            canvas.get_context("2d").unwrap().unwrap().unchecked_into();
        ctx.scale(dpr, dpr).ok();
        ctx.clear_rect(0.0, 0.0, css_w, css_h);

        // === Left half: Win matrix ===
        let mat_x = 20.0;
        let mat_y = 22.0;
        let cell = 26.0;
        let hdr = 16.0; // space for header labels

        // Header label
        ctx.set_font("bold 11px system-ui, sans-serif");
        ctx.set_fill_style_str("#2d1f3d");
        ctx.set_text_align("center");

        // Column headers
        for j in 0..N {
            let x = mat_x + hdr + j as f64 * cell + cell / 2.0;
            ctx.fill_text(LABELS[j], x, mat_y - 4.0).ok();
        }
        // Row headers
        ctx.set_text_align("right");
        for i in 0..N {
            let y = mat_y + i as f64 * cell + cell / 2.0 + 4.0;
            ctx.fill_text(LABELS[i], mat_x + hdr - 4.0, y).ok();
        }

        // Cells
        for i in 0..N {
            for j in 0..N {
                let x = mat_x + hdr + j as f64 * cell;
                let y = mat_y + i as f64 * cell;

                if i == j {
                    // Diagonal: dark
                    ctx.set_fill_style_str("#d4c4dd");
                    ctx.fill_rect(x, y, cell, cell);
                } else {
                    // Highlight last vote cell
                    let is_last = lv.map_or(false, |(wi, lo)| wi == i && lo == j);
                    if is_last {
                        ctx.set_fill_style_str("#f5b43666");
                    } else {
                        ctx.set_fill_style_str("#f5f0f7");
                    }
                    ctx.fill_rect(x, y, cell, cell);

                    // Win count
                    let count = w[i][j];
                    if count > 0 {
                        ctx.set_font("bold 11px system-ui, sans-serif");
                        ctx.set_text_align("center");
                        ctx.set_fill_style_str("#742a85");
                        ctx.fill_text(&count.to_string(), x + cell / 2.0, y + cell / 2.0 + 4.0)
                            .ok();
                    }
                }

                // Cell border
                ctx.set_stroke_style_str("#d4c4dd");
                ctx.set_line_width(0.5);
                ctx.stroke_rect(x, y, cell, cell);
            }
        }

        // "wins >" label
        ctx.set_font("9px system-ui, sans-serif");
        ctx.set_fill_style_str("#7a6888");
        ctx.set_text_align("left");
        ctx.fill_text(t().sim_row_beats_col, mat_x, mat_y + N as f64 * cell + 12.0)
            .ok();

        // === Right half: β bars ===
        let bar_x = mat_x + hdr + N as f64 * cell + 30.0;
        let bar_area_w = css_w - bar_x - 10.0;
        let bar_h = 16.0;
        let bar_gap = 6.0;
        let bars_top = mat_y;

        // Find max display score for scaling
        let display_scores: Vec<f64> = b.iter().map(|&s| (1.0 + s).log2() * 500.0).collect();
        let max_score = display_scores.iter().cloned().fold(1.0_f64, f64::max);

        ctx.set_font("bold 11px system-ui, sans-serif");
        for i in 0..N {
            let y = bars_top + i as f64 * (bar_h + bar_gap);
            let score = display_scores[i];
            let bar_w = (score / max_score) * (bar_area_w - 32.0);
            let bar_w = bar_w.max(0.0);

            // Label
            ctx.set_text_align("right");
            ctx.set_fill_style_str("#2d1f3d");
            ctx.fill_text(LABELS[i], bar_x + 10.0, y + bar_h / 2.0 + 4.0)
                .ok();

            // Bar
            ctx.set_fill_style_str("#742a85");
            ctx.fill_rect(bar_x + 14.0, y, bar_w, bar_h);

            // Score value
            ctx.set_text_align("left");
            ctx.set_font("10px system-ui, sans-serif");
            ctx.set_fill_style_str("#7a6888");
            ctx.fill_text(
                &format!("{:.0}", score),
                bar_x + 14.0 + bar_w + 3.0,
                y + bar_h / 2.0 + 3.5,
            )
            .ok();
            ctx.set_font("bold 11px system-ui, sans-serif");
        }

        // β header
        ctx.set_font("bold 11px system-ui, sans-serif");
        ctx.set_text_align("left");
        ctx.set_fill_style_str("#2d1f3d");
        ctx.fill_text(t().sim_score, bar_x, bars_top - 6.0).ok();
    });

    view! {
        <div class="bt-demo">
            <div class="bt-sim-header">
                {move || fill(t().sim_votes, &[("n", &step.get())])}
            </div>
            <canvas id="bt-sim-canvas"
                style="width:340px;height:170px;display:block;margin:0 auto;" />
            <div class="bt-sim-controls">
                <button on:click=toggle_play class="bt-sim-btn">
                    {move || if playing.get() { t().sim_pause } else { t().sim_play }}
                </button>
                <button on:click=step_one class="bt-sim-btn">
                    {t().sim_step}
                </button>
                <button on:click=reset class="bt-sim-btn">
                    {t().sim_reset}
                </button>
            </div>
        </div>
    }
}

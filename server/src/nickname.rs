//! Every voter's public nickname, which the list of who voted most shows
//! instead of who they are: a film-crew job and a Swiss adjective, drawn at
//! random among those no one bears yet, or whatever its owner writes.

use crate::config::Lang;
use crate::models::{Nickname, UserState};
use crate::persistence::AppState;
use versus_shared::NamePayload;

use actix_web::{HttpResponse, web};
use rand::seq::SliceRandom;
use std::collections::{BTreeSet, HashMap, HashSet};

const MAX_NICKNAME_CHARS: usize = 40;

/// A French word in its masculine and feminine forms, the same twice when
/// it has one.
type Gendered = (&'static str, &'static str);

const JOBS_FR: [Gendered; 26] = [
    ("Perchiste", "Perchiste"),
    ("Cadreur", "Cadreuse"),
    ("Monteur", "Monteuse"),
    ("Scripte", "Scripte"),
    ("Machiniste", "Machiniste"),
    ("Électro", "Électro"),
    ("Chef op", "Cheffe op"),
    ("Réalisateur", "Réalisatrice"),
    ("Producteur", "Productrice"),
    ("Régisseur", "Régisseuse"),
    ("Maquilleur", "Maquilleuse"),
    ("Coiffeur", "Coiffeuse"),
    ("Costumier", "Costumière"),
    ("Accessoiriste", "Accessoiriste"),
    ("Cascadeur", "Cascadeuse"),
    ("Figurant", "Figurante"),
    ("Ingé son", "Ingé son"),
    ("Mixeur", "Mixeuse"),
    ("Bruiteur", "Bruiteuse"),
    ("Étalonneur", "Étalonneuse"),
    ("Décorateur", "Décoratrice"),
    ("Projectionniste", "Projectionniste"),
    ("Scénariste", "Scénariste"),
    ("Assistant réal", "Assistante réal"),
    ("Cantinier", "Cantinière"),
    ("Stagiaire", "Stagiaire"),
];

const ADJECTIVES_FR: [Gendered; 39] = [
    // The clichés.
    ("Neutre", "Neutre"),
    ("Ponctuel", "Ponctuelle"),
    ("Discret", "Discrète"),
    ("Chocolaté", "Chocolatée"),
    ("Horloger", "Horlogère"),
    ("Alpin", "Alpine"),
    ("Précis", "Précise"),
    ("Fromager", "Fromagère"),
    ("Bancaire", "Bancaire"),
    ("Propret", "Proprette"),
    ("Montagnard", "Montagnarde"),
    ("Fondu", "Fondue"),
    ("Consensuel", "Consensuelle"),
    ("Trilingue", "Trilingue"),
    // The cantons and the regions.
    ("Vaudois", "Vaudoise"),
    ("Valaisan", "Valaisanne"),
    ("Genevois", "Genevoise"),
    ("Jurassien", "Jurassienne"),
    ("Fribourgeois", "Fribourgeoise"),
    ("Neuchâtelois", "Neuchâteloise"),
    ("Lémanique", "Lémanique"),
    ("Bernois", "Bernoise"),
    ("Tessinois", "Tessinoise"),
    ("Gruérien", "Gruérienne"),
    ("Grison", "Grisonne"),
    ("Lausannois", "Lausannoise"),
    ("Zurichois", "Zurichoise"),
    // The words of Romandy.
    ("Bobet", "Bobette"),
    ("Royal", "Royale"),
    ("Déguillé", "Déguillée"),
    ("Gâtion", "Gâtionne"),
    ("Tip-top", "Tip-top"),
    ("Fatigant", "Fatigante"),
    ("Encoublé", "Encoublée"),
    ("Rapicolé", "Rapicolée"),
    ("Agendé", "Agendée"),
    ("Panossé", "Panossée"),
    ("Septante", "Septante"),
    ("Nonante", "Nonante"),
];

const JOBS_EN: [&str; 24] = [
    "Boom Operator",
    "Gaffer",
    "Best Boy",
    "Grip",
    "Script Supervisor",
    "Editor",
    "Director",
    "Producer",
    "Foley Artist",
    "Stunt Double",
    "Extra",
    "Camera Operator",
    "Colorist",
    "Makeup Artist",
    "Set Dresser",
    "Location Scout",
    "Runner",
    "Intern",
    "Projectionist",
    "Screenwriter",
    "Sound Mixer",
    "Clapper Loader",
    "Caterer",
    "Prop Master",
];

const ADJECTIVES_EN: [&str; 30] = [
    // The clichés.
    "Neutral",
    "Punctual",
    "Discreet",
    "Chocolatey",
    "Clockwork",
    "Alpine",
    "Precise",
    "Cheesy",
    "Yodelling",
    "Spotless",
    "Trilingual",
    "Consensual",
    "Fondue-Dipped",
    "Banking",
    // The cantons and the regions.
    "Vaudois",
    "Valaisan",
    "Genevan",
    "Bernese",
    "Ticinese",
    "Jurassian",
    "Lakeside",
    "Zurich",
    "Gruyère",
    "Grisons",
    // The words of Romandy, as English would say them.
    "Tip-Top",
    "Royal",
    "Tumbled",
    "Spoilt",
    "Mopped",
    "Raclette-Fed",
];

/// Every nickname there is to draw in `lang`, each once.
fn every_nickname(lang: Lang) -> Vec<String> {
    let all: BTreeSet<String> = match lang {
        Lang::Fr => JOBS_FR
            .iter()
            .flat_map(|job| {
                ADJECTIVES_FR.iter().flat_map(move |adjective| {
                    [
                        format!("{} {}", job.0, adjective.0),
                        format!("{} {}", job.1, adjective.1),
                    ]
                })
            })
            .collect(),
        Lang::En => JOBS_EN
            .iter()
            .flat_map(|job| {
                ADJECTIVES_EN
                    .iter()
                    .map(move |adjective| format!("{adjective} {job}"))
            })
            .collect(),
    };
    all.into_iter().collect()
}

/// A nickname no one in `users` bears, drawn at random; should every one be
/// taken, the first number after them that frees some.
fn draw(users: &HashMap<String, UserState>, lang: Lang) -> Nickname {
    let taken: HashSet<String> = users
        .values()
        .filter_map(|u| u.nickname.as_ref())
        .map(|n| n.text.to_lowercase())
        .collect();
    let all = every_nickname(lang);
    let mut rng = rand::thread_rng();
    let text = (1..)
        .find_map(|n: usize| {
            let free: Vec<String> = all
                .iter()
                .map(|name| {
                    if n == 1 {
                        name.clone()
                    } else {
                        format!("{name} {n}")
                    }
                })
                .filter(|name| !taken.contains(&name.to_lowercase()))
                .collect();
            free.choose(&mut rng).cloned()
        })
        .expect("some number frees a nickname");
    Nickname {
        text,
        written: false,
    }
}

/// Give the user `user_id` a nickname drawn at random if they have none.
pub fn name(users: &mut HashMap<String, UserState>, user_id: &str, lang: Lang) {
    if users.get(user_id).is_some_and(|u| u.nickname.is_none()) {
        let nickname = draw(users, lang);
        users.get_mut(user_id).unwrap().nickname = Some(nickname);
    }
}

/// Give every user who voted and has no nickname one drawn at random.
/// Answers how many it named.
pub fn name_voters(users: &mut HashMap<String, UserState>, lang: Lang) -> usize {
    let mut unnamed: Vec<String> = users
        .iter()
        .filter(|(_, u)| u.nickname.is_none() && !u.compared_pairs.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    unnamed.sort();
    for user_id in &unnamed {
        name(users, user_id, lang);
    }
    unnamed.len()
}

/// Set the nickname the user writes, or draw another at random for an empty
/// one. Answers with the nickname.
pub async fn set_nickname(
    data: web::Data<AppState>,
    payload: web::Json<NamePayload>,
) -> HttpResponse {
    if data.is_banned(&payload.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "banned"}));
    }
    let text = payload.name.trim();
    if text.chars().count() > MAX_NICKNAME_CHARS || text.chars().any(char::is_control) {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "bad_text"}));
    }
    let nickname = {
        let mut users = data.users.lock().unwrap();
        let nickname = if text.is_empty() {
            draw(&users, data.lang)
        } else {
            Nickname {
                text: text.to_string(),
                written: true,
            }
        };
        let user = users.entry(payload.user_id.clone()).or_default();
        user.nickname = Some(nickname.clone());
        nickname
    };
    data.save();
    HttpResponse::Ok().json(serde_json::json!({"nickname": nickname}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_nicknames_agree_in_gender_and_are_many() {
        let all = every_nickname(Lang::Fr);
        assert!(all.contains(&"Cadreuse Déguillée".to_string()));
        assert!(all.contains(&"Cadreur Déguillé".to_string()));
        assert!(!all.contains(&"Cadreuse Déguillé".to_string()));
        assert!(all.len() > 1500, "{}", all.len());
        assert_eq!(
            every_nickname(Lang::En).len(),
            JOBS_EN.len() * ADJECTIVES_EN.len()
        );
    }

    #[test]
    fn drawn_nicknames_are_never_borne_twice() {
        let mut users: HashMap<String, UserState> = HashMap::new();
        let total = every_nickname(Lang::En).len() + 5;
        for i in 0..total {
            let id = format!("u{i}");
            users.insert(id.clone(), UserState::default());
            name(&mut users, &id, Lang::En);
        }
        let names: HashSet<String> = users
            .values()
            .map(|u| u.nickname.as_ref().unwrap().text.clone())
            .collect();
        assert_eq!(names.len(), total);
        assert_eq!(names.iter().filter(|n| n.ends_with(" 2")).count(), 5);
    }
}

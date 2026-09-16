//! Construction de la carte qui représente un message du forum.
//!
//! La maquette du CDC ne montre qu'un titre et un texte sur fond coloré. S'y
//! ajoute, à la demande de l'équipe, la liste des rôles que la carte accorde :
//! le rôle du message, et le rôle parent du fil s'il y en a un.
//!
//! La couleur vient du fil, sauf si le message en porte une : c'est ainsi qu'on
//! grise un groupe endormi, sans avoir à décrire un état quelque part.

use crate::db::categories::Category;
use crate::db::posts::Post;
use poise::serenity_prelude as serenity;

/// Discord tronque les titres au-delà de 256 caractères et rejette les
/// descriptions au-delà de 4096.
pub const TITLE_LIMIT: usize = 256;
pub const DESCRIPTION_LIMIT: usize = 4096;

/// La carte accorde-t-elle un rôle ? Elle porte alors la main levée, et
/// annonce ce qu'elle donne.
///
/// Une carte peut porter la main levée sans rien accorder : celle qui renonce
/// au rôle parent de son fil pour n'envoyer qu'un message privé. Elle reste
/// muette sur les rôles, faute d'en donner.
pub fn grants_a_role(category: &Category, post: &Post) -> bool {
    !post.information
        && (post.role_id.is_some() || (post.grants_parent && category.parent_role_id.is_some()))
}

/// La carte porte-t-elle la main levée ? Un message d'information n'en porte
/// jamais ; les autres l'affichent dès qu'il y a quelque chose à obtenir, rôle
/// ou message privé.
pub fn carries_a_reaction(category: &Category, post: &Post) -> bool {
    grants_a_role(category, post)
        || (!post.information
            && post
                .dm_text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty()))
}

pub fn card(category: &Category, post: &Post) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new().title(truncate(&post.title, TITLE_LIMIT));

    if let Some(colour) = post
        .colour
        .or(category.colour)
        .and_then(|value| u32::try_from(value).ok())
    {
        embed = embed.colour(serenity::Colour::new(colour));
    }
    if !post.body.trim().is_empty() {
        embed = embed.description(truncate(&post.body, DESCRIPTION_LIMIT));
    }

    // Une carte dit ce qui arrive quand on lève la main. Le nom du rôle ne
    // suffit pas : « @portail-équipe » ne veut rien dire pour qui arrive, et le
    // message privé qui l'explique n'arrive qu'après le clic.
    if carries_a_reaction(category, post) {
        // Les mentions ne se rendent pas dans un pied d'embed, seulement dans
        // un champ : c'est donc un champ, non aligné, pour qu'il ait sa ligne.
        embed = embed.field("En levant la main 🙋", promise(category, post), false);
    }

    embed
}

/// Ce que la carte promet, en français plutôt qu'en noms de rôles.
fn promise(category: &Category, post: &Post) -> String {
    let granted: Vec<String> = [
        post.role_id,
        category.parent_role_id.filter(|_| post.grants_parent),
    ]
    .into_iter()
    .flatten()
    .map(|role_id| format!("<@&{role_id}>"))
    .collect();

    let mut lines = Vec::new();
    match granted.len() {
        0 => {}
        1 => lines.push(format!(
            "Tu reçois le rôle {}, qui t'ouvre les salons correspondants.",
            granted[0]
        )),
        _ => lines.push(format!(
            "Tu reçois les rôles {}, qui t'ouvrent les salons correspondants.",
            granted.join(" et ")
        )),
    }
    if sends_a_message(category, post) {
        lines.push("Tu reçois un message privé qui explique la suite.".to_owned());
    }
    // Retirer sa réaction reprend le rôle sans un mot : mieux vaut l'avoir lu
    // avant de cliquer qu'après.
    if granted.is_empty() {
        lines.push("Aucun rôle ne t'est attribué.".to_owned());
    } else {
        lines.push("Retire la réaction pour rendre ce que tu as reçu.".to_owned());
    }
    lines.join("\n")
}

/// Une réaction sur cette carte déclenche-t-elle un message privé ? Celui de la
/// carte s'il existe, celui du fil sinon.
fn sends_a_message(category: &Category, post: &Post) -> bool {
    let filled = |text: &Option<String>| {
        text.as_deref()
            .is_some_and(|content| !content.trim().is_empty())
    };
    filled(&post.dm_text) || filled(&category.dm_text)
}

/// Discord compte en points de code ; couper sur des octets casserait un
/// caractère accentué et l'API rejetterait le message.
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    text.chars()
        .take(limit.saturating_sub(1))
        .chain(['…'])
        .collect()
}

/// Rétablit les retours à la ligne saisis en `\n` littéral.
///
/// Les paramètres de commande Discord ne peuvent pas contenir de saut de ligne.
/// Le fichier de contenu, lui, utilise de vrais retours à la ligne et n'a pas
/// besoin de cette conversion — elle est sans effet sur un texte qui n'en
/// contient pas.
pub fn format_description(raw: &str) -> String {
    raw.replace("\\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{categories, posts};

    fn thread(parent_role_id: Option<i64>) -> Category {
        Category {
            colour: Some(0xF2994A),
            parent_role_id,
            ..categories::fixture("Groupes locaux", 100)
        }
    }

    fn message(title: &str, body: &str, role_id: Option<i64>) -> Post {
        Post {
            title: title.to_owned(),
            body: body.to_owned(),
            ..posts::fixture(1, "paris", role_id)
        }
    }

    fn rendered(category: &Category, post: &Post) -> serde_json::Value {
        serde_json::to_value(card(category, post)).unwrap()
    }

    #[test]
    fn newlines_are_restored() {
        assert_eq!(format_description("une\\ndeux"), "une\ndeux");
        assert_eq!(format_description("sans"), "sans");
    }

    #[test]
    fn a_card_renouncing_the_parent_role_promises_nothing() {
        let thread = thread(Some(999));
        let card = Post {
            grants_parent: false,
            dm_text: Some("Voici le lien du groupe".into()),
            ..message("PauseAction", "Cinq minutes quand tu peux.", None)
        };

        assert!(!grants_a_role(&thread, &card));
        // Elle porte quand même la main levée : c'est ainsi qu'on demande le
        // message privé.
        assert!(carries_a_reaction(&thread, &card));
    }

    #[test]
    fn an_information_card_stays_silent_even_with_a_message() {
        let card = Post {
            information: true,
            dm_text: Some("jamais envoyé".into()),
            ..message("Idées reçues", "", None)
        };
        assert!(!carries_a_reaction(&thread(Some(999)), &card));
    }

    #[test]
    fn truncation_counts_characters_not_bytes() {
        // 300 caractères accentués font 600 octets : couper sur les octets
        // produirait une chaîne invalide.
        let long = "é".repeat(300);
        let cut = truncate(&long, TITLE_LIMIT);

        assert_eq!(cut.chars().count(), TITLE_LIMIT);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn the_card_shows_a_title_a_text_and_its_roles() {
        let json = rendered(
            &thread(Some(999)),
            &message("Paris", "Un groupe très actif", Some(10)),
        );

        assert_eq!(json["title"], "Paris");
        assert_eq!(json["description"], "Un groupe très actif");
        assert_eq!(json["color"], 0xF2994A);

        let fields = json["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0]["name"], "En levant la main 🙋");
        let promise = fields[0]["value"].as_str().unwrap();
        assert!(promise.contains("<@&10> et <@&999>"), "{promise}");
        assert!(promise.contains("Retire la réaction"), "{promise}");
    }

    #[test]
    fn a_thread_without_parent_role_shows_a_single_one() {
        let json = rendered(&thread(None), &message("Paris", "", Some(10)));
        let fields = json["fields"].as_array().unwrap();

        assert!(
            fields[0]["value"]
                .as_str()
                .unwrap()
                .contains("le rôle <@&10>,")
        );
    }

    #[test]
    fn a_message_without_role_shows_the_parent_role_of_its_thread() {
        // Sans rôle nominatif mais dans un fil à rôle parent, la carte accorde
        // ce parent : elle doit l'annoncer.
        let json = rendered(&thread(Some(999)), &message("Fresque", "Un projet", None));
        let fields = json["fields"].as_array().unwrap();

        assert!(
            fields[0]["value"]
                .as_str()
                .unwrap()
                .contains("le rôle <@&999>,")
        );
    }

    #[test]
    fn an_information_message_shows_no_role_at_all() {
        // Marqué information : rien à annoncer, même dans un fil à rôle parent.
        let json = rendered(
            &thread(Some(999)),
            &Post {
                information: true,
                ..message("Bienvenue", "Levez la main", None)
            },
        );

        assert!(json.get("fields").is_none() || json["fields"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_message_colour_wins_over_the_thread() {
        // C'est ce qui permet de griser un groupe endormi au milieu d'un fil
        // orange, sans introduire de notion de statut.
        let dormant = Post {
            colour: Some(0x99AAB5),
            ..message("Colmar", "Groupe inactif", Some(10))
        };
        let json = rendered(&thread(Some(999)), &dormant);

        assert_eq!(json["color"], 0x99AAB5);
    }

    #[test]
    fn a_thread_without_colour_leaves_it_to_discord() {
        let bare = categories::fixture("Projets", 100);
        let json = rendered(&bare, &message("Paris", "", Some(10)));

        assert!(json.get("color").is_none() || json["color"].is_null());
    }

    #[test]
    fn empty_text_is_omitted() {
        let json = rendered(&thread(None), &message("Paris", "   ", Some(10)));
        assert!(json.get("description").is_none() || json["description"].is_null());
    }
}

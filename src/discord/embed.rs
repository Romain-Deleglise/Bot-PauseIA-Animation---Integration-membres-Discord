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
const TITLE_LIMIT: usize = 256;
const DESCRIPTION_LIMIT: usize = 4096;

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

    // Une carte annonce les rôles qu'elle accorde : le sien, et/ou le rôle
    // parent du fil. Un message d'information n'accorde rien et ne porte aucune
    // réaction : sa carte reste muette sur les rôles.
    if !post.information && (post.role_id.is_some() || category.parent_role_id.is_some()) {
        // Les mentions ne se rendent pas dans un pied d'embed, seulement dans
        // un champ : c'est donc un champ, non aligné, pour qu'il ait sa ligne.
        let granted: Vec<String> = [post.role_id, category.parent_role_id]
            .into_iter()
            .flatten()
            .map(|role_id| format!("<@&{role_id}>"))
            .collect();
        let label = if granted.len() > 1 { "Rôles" } else { "Rôle" };
        embed = embed.field(label, granted.join(" "), false);
    }

    embed
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
        assert_eq!(fields[0]["name"], "Rôles");
        assert_eq!(fields[0]["value"], "<@&10> <@&999>");
    }

    #[test]
    fn a_thread_without_parent_role_shows_a_single_one() {
        let json = rendered(&thread(None), &message("Paris", "", Some(10)));
        let fields = json["fields"].as_array().unwrap();

        assert_eq!(fields[0]["name"], "Rôle");
        assert_eq!(fields[0]["value"], "<@&10>");
    }

    #[test]
    fn a_message_without_role_shows_the_parent_role_of_its_thread() {
        // Sans rôle nominatif mais dans un fil à rôle parent, la carte accorde
        // ce parent : elle doit l'annoncer.
        let json = rendered(&thread(Some(999)), &message("Fresque", "Un projet", None));
        let fields = json["fields"].as_array().unwrap();

        assert_eq!(fields[0]["name"], "Rôle");
        assert_eq!(fields[0]["value"], "<@&999>");
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

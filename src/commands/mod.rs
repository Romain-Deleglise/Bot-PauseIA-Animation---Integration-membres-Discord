//! Commandes slash, et les quelques aides qu'elles partagent.
//!
//! Les noms et descriptions sont écrits ici, en français, plutôt que lus dans
//! l'environnement comme le faisait l'ancien bot. Vingt variables de `.env` ne
//! servaient qu'à ça, et une seule oubliée empêchait le démarrage.

pub mod import;
pub mod posts;
pub mod threads;

use crate::mentions;
use crate::state::{Context, Data, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;

/// Valeur qui, passée à un paramètre facultatif, efface le champ existant.
pub const CLEAR_SENTINEL: &str = "-";

/// Longueur maximale d'un message Discord, donc d'un message privé d'accueil.
pub const MAX_DM_CHARS: usize = 2_000;

/// Commandes à enregistrer. `/forum importer` n'est proposé que si
/// `ENABLE_IMPORT` le permet : il ne sert en principe qu'à l'amorçage.
pub fn all(enable_import: bool) -> Vec<poise::Command<Data, Error>> {
    let mut forum = forum();
    if !enable_import {
        forum
            .subcommands
            .retain(|command| command.name != "importer");
    }
    vec![forum]
}

#[poise::command(
    slash_command,
    guild_only,
    rename = "forum",
    subcommands(
        "threads::fil",
        "posts::message",
        "import::importer",
        "import::republier",
    )
)]
pub async fn forum(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Autorise ou non l'appelant, quelle que soit la commande.
///
/// Posé une fois dans `FrameworkOptions::command_check`, il couvre aussi les
/// sous-commandes ajoutées plus tard — impossible d'en oublier une.
pub async fn is_manager(ctx: Context<'_>) -> Result<bool, Error> {
    let Some(member) = ctx.author_member().await else {
        return Ok(false);
    };
    let allowed = member
        .roles
        .iter()
        .any(|role_id| ctx.data().config.manage_role_ids.contains(role_id));

    // Répondre uniquement à une vraie invocation. poise applique ce contrôle
    // aussi aux requêtes d'autocomplétion, où toute tentative de réponse
    // panique : sans ce garde, un membre non autorisé qui saisit une commande
    // en déclencherait une à chaque frappe.
    if !allowed && !is_autocomplete(ctx) {
        ctx.send(
            poise::CreateReply::default()
                .content("Cette commande est réservée aux rôles de gestion du serveur.")
                .ephemeral(true),
        )
        .await?;
    }
    Ok(allowed)
}

/// Vrai si le contexte décrit une saisie en cours, et non une commande lancée.
///
/// À consulter avant toute réponse depuis un `check` : poise y passe aussi les
/// requêtes d'autocomplétion, où `ctx.say` et `ctx.send` paniquent.
pub fn is_autocomplete(ctx: Context<'_>) -> bool {
    matches!(
        ctx,
        poise::Context::Application(application)
            if application.interaction_type == poise::CommandInteractionType::Autocomplete
    )
}

/// Prépare une réponse éphémère et longue : toutes ces commandes dépassent
/// allègrement la fenêtre de trois secondes imposée aux interactions.
pub async fn begin(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    Ok(())
}

/// Salons capables d'accueillir les cartes d'un fil.
pub fn is_writable(kind: serenity::ChannelType) -> bool {
    matches!(
        kind,
        serenity::ChannelType::Text
            | serenity::ChannelType::News
            | serenity::ChannelType::PublicThread
            | serenity::ChannelType::PrivateThread
            | serenity::ChannelType::NewsThread
    )
}

/// `#FF6600`, `0xFF6600` ou `FF6600` → entier stockable.
pub fn parse_colour(raw: &str) -> Result<i64, Error> {
    let cleaned = raw
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start_matches("0X");

    if cleaned.len() != 6 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("`{raw}` n'est pas une couleur hexadécimale (attendu `#RRGGBB`)");
    }
    i64::from_str_radix(cleaned, 16).map_err(Into::into)
}

/// Discord n'affiche que des images servies en HTTP(S).
pub fn parse_url(raw: &str) -> Result<String, Error> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        anyhow::bail!("`{raw}` n'est pas une URL http(s)");
    }
    Ok(trimmed.to_owned())
}

/// Applique un paramètre facultatif : absent = inchangé, `-` = effacé.
pub fn apply_optional(
    current: Option<String>,
    input: Option<String>,
    parse: impl Fn(&str) -> Result<String, Error>,
) -> Result<Option<String>, Error> {
    match input.as_deref().map(str::trim) {
        None => Ok(current),
        Some(CLEAR_SENTINEL) => Ok(None),
        Some(value) => parse(value).map(Some),
    }
}

/// Lit un identifiant Discord saisi en texte.
///
/// Les paramètres entiers d'une commande slash sont plafonnés à 2^53 par
/// Discord : un snowflake les dépasse et doit transiter par une chaîne.
pub fn parse_snowflake(raw: &str) -> Result<u64, Error> {
    let trimmed = raw.trim();
    match trimmed.parse::<u64>() {
        Ok(0) | Err(_) => anyhow::bail!("`{raw}` n'est pas un identifiant Discord valide"),
        Ok(id) => Ok(id),
    }
}

/// Pseudos des membres du serveur, en minuscules, avec leur identifiant.
///
/// Parcourt toute la liste des membres, ce qu'autorise l'intent `GUILD_MEMBERS`.
/// Réservé aux commandes d'administration : le coût est payé sur un chemin froid.
pub async fn member_handles(ctx: Context<'_>) -> Result<HashMap<String, u64>, Error> {
    const PAGE: usize = 1000;
    let guild_id = ctx.data().config.guild_id;
    let mut handles = HashMap::new();
    let mut after: Option<serenity::UserId> = None;

    loop {
        let page = guild_id
            .members(ctx.http(), Some(PAGE as u64), after)
            .await?;
        let Some(last) = page.last() else {
            break;
        };
        after = Some(last.user.id);
        let complete = page.len() < PAGE;
        for member in page {
            handles.insert(member.user.name.to_lowercase(), member.user.id.get());
        }
        if complete {
            break;
        }
    }
    Ok(handles)
}

/// Convertit les `@pseudo` d'un texte saisi en commande en vraies mentions.
/// Le serveur n'est interrogé que si le texte contient un `@`.
pub async fn link_mentions(ctx: Context<'_>, text: String) -> Result<(String, Vec<String>), Error> {
    if !text.contains('@') {
        return Ok((text, Vec::new()));
    }
    let members = member_handles(ctx).await?;
    Ok(mentions::link_handles(&text, &members))
}

/// Ligne à ajouter à une réponse quand des pseudos n'ont pas été reconnus.
pub fn unknown_handles_note(unknown: &[String]) -> String {
    if unknown.is_empty() {
        return String::new();
    }
    let handles: Vec<String> = unknown.iter().map(|handle| format!("@{handle}")).collect();
    format!(
        "\nPseudos introuvables sur le serveur, laissés en texte : {}.",
        handles.join(", ")
    )
}

/// Discord refuse un message de plus de 2000 caractères. La marge couvre sa
/// façon de compter, qui n'est pas tout à fait celle de Rust.
const MESSAGE_LIMIT: usize = 1900;

/// Répond en autant de messages éphémères qu'il le faut : la liste complète
/// des cartes du forum dépasse à elle seule la limite d'un message.
pub async fn say_long(ctx: Context<'_>, text: String) -> Result<(), Error> {
    for chunk in split_message(&text, MESSAGE_LIMIT) {
        ctx.send(poise::CreateReply::default().content(chunk).ephemeral(true))
            .await?;
    }
    Ok(())
}

/// Découpe un texte en morceaux d'au plus `limit` caractères, entre deux lignes
/// pour qu'une carte de la liste ne se retrouve pas à cheval sur deux messages.
fn split_message(text: &str, limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for mut line in text.lines() {
        // Une ligne trop longue à elle seule est coupée net, sur une frontière
        // de caractère : un texte accentué ne doit pas faire paniquer le bot.
        while line.chars().count() > limit {
            let cut = line
                .char_indices()
                .nth(limit)
                .map_or(line.len(), |(index, _)| index);
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            chunks.push(line[..cut].to_owned());
            line = &line[cut..];
        }

        let separator = usize::from(!current.is_empty());
        if current.chars().count() + separator + line.chars().count() > limit {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_answer_stays_in_one_message() {
        assert_eq!(split_message("a\nb", 100), vec!["a\nb"]);
    }

    #[test]
    fn a_long_list_is_split_between_lines() {
        let text = (0..300)
            .map(|i| format!("ligne {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = split_message(&text, 100);

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 100));
        assert_eq!(chunks.join("\n"), text);
    }

    #[test]
    fn an_oversized_line_is_cut_without_splitting_a_character() {
        let line = "é".repeat(250);
        let chunks = split_message(&line, 100);

        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 100));
        assert_eq!(chunks.concat(), line);
    }

    fn subcommands(enable_import: bool) -> Vec<String> {
        all(enable_import)[0]
            .subcommands
            .iter()
            .map(|command| command.name.clone())
            .collect()
    }

    #[test]
    fn import_is_offered_when_enabled() {
        assert!(subcommands(true).contains(&"importer".to_owned()));
    }

    #[test]
    fn import_disappears_when_disabled_and_nothing_else_does() {
        let without = subcommands(false);
        assert!(!without.contains(&"importer".to_owned()));
        // Seul l'import doit partir : les autres commandes restent.
        assert_eq!(without.len(), subcommands(true).len() - 1);
        assert!(without.contains(&"republier".to_owned()));
    }

    #[test]
    fn colours_accept_the_usual_notations() {
        for raw in ["#FF6600", "FF6600", "0xFF6600", "  #ff6600  "] {
            assert_eq!(parse_colour(raw).unwrap(), 0xFF6600);
        }
    }

    #[test]
    fn colours_reject_anything_else() {
        for raw in ["", "#FFF", "#GGGGGG", "FF66000", "rouge"] {
            assert!(parse_colour(raw).is_err(), "`{raw}` aurait dû être refusé");
        }
    }

    #[test]
    fn urls_must_be_http() {
        assert_eq!(
            parse_url(" https://exemple.test/a.png ").unwrap(),
            "https://exemple.test/a.png"
        );
        assert!(parse_url("exemple.test/a.png").is_err());
        assert!(parse_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn snowflakes_come_through_strings() {
        // Au-delà de 2^53, un paramètre entier Discord perdrait des chiffres.
        assert_eq!(
            parse_snowflake("1076526842414125106").unwrap(),
            1076526842414125106
        );
        assert!(parse_snowflake("0").is_err());
        assert!(parse_snowflake("abc").is_err());
    }

    #[test]
    fn optional_parameters_distinguish_absent_from_cleared() {
        let keep = |value: &str| Ok(value.to_owned());
        let current = Some("valeur".to_owned());

        assert_eq!(
            apply_optional(current.clone(), None, keep).unwrap(),
            current
        );
        assert_eq!(
            apply_optional(current.clone(), Some(CLEAR_SENTINEL.into()), keep).unwrap(),
            None
        );
        assert_eq!(
            apply_optional(current, Some("neuve".into()), keep).unwrap(),
            Some("neuve".to_owned())
        );
    }

    #[test]
    fn optional_parameters_propagate_validation_errors() {
        assert!(apply_optional(None, Some("pas-une-url".into()), parse_url).is_err());
    }

    #[test]
    fn only_writable_channels_can_host_a_thread() {
        assert!(is_writable(serenity::ChannelType::PublicThread));
        assert!(is_writable(serenity::ChannelType::Text));
        // Un salon forum n'accepte pas de message : seuls ses fils en reçoivent.
        assert!(!is_writable(serenity::ChannelType::Forum));
        assert!(!is_writable(serenity::ChannelType::Voice));
        assert!(!is_writable(serenity::ChannelType::Category));
    }
}

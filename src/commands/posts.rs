//! `/forum message …` — création, édition et suppression des messages du forum.
//!
//! Chaque opération ne touche que la carte concernée. L'ancien bot réécrivait
//! le salon entier à chaque fois, ce qui effaçait au passage les réactions déjà
//! posées par les membres — exactement ce que le CDC interdit.

use crate::commands::{self, threads::autocomplete_thread};
use crate::db;
use crate::db::posts::Post;
use crate::discord::channel;
use crate::discord::embed;
use crate::ids;
use crate::state::{Context, Error};
use poise::serenity_prelude as serenity;

/// Propose les messages existants, préfixés de leur fil.
///
/// La valeur transmise est l'identifiant du message, pas son titre : deux
/// messages peuvent porter le même titre, et un titre se corrige.
pub async fn autocomplete_post(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<serenity::AutocompleteChoice> {
    let (Ok(posts), Ok(categories)) = (
        db::posts::all(&ctx.data().db).await,
        db::categories::list(&ctx.data().db).await,
    ) else {
        return Vec::new();
    };
    let needle = partial.to_lowercase();

    posts
        .into_iter()
        .filter(|post| post.title.to_lowercase().contains(&needle))
        .take(25)
        .map(|post| {
            let thread = categories
                .iter()
                .find(|category| category.id == post.category_id)
                .map(|category| category.name.as_str())
                .unwrap_or("?");
            serenity::AutocompleteChoice::new(
                format!("{thread} › {}", post.title),
                post.id.to_string(),
            )
        })
        .collect()
}

/// Retrouve un message depuis ce qu'a transmis l'autocomplétion, ou depuis un
/// titre saisi à la main.
async fn resolve(ctx: Context<'_>, raw: &str) -> Result<Option<Post>, Error> {
    if let Ok(id) = raw.trim().parse::<i64>()
        && let Some(post) = db::posts::by_id(&ctx.data().db, id).await?
    {
        return Ok(Some(post));
    }
    let needle = raw.trim().to_lowercase();
    Ok(db::posts::all(&ctx.data().db)
        .await?
        .into_iter()
        .find(|post| post.title.to_lowercase() == needle))
}

/// Identifiant lisible construit depuis le titre, pour le réimport.
fn slug_from(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "message".to_owned()
    } else {
        trimmed
    }
}

#[poise::command(
    slash_command,
    rename = "message",
    subcommands("creer", "modifier", "supprimer", "liste")
)]
pub async fn message(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Ajouter un message à un fil.
#[poise::command(slash_command, rename = "créer")]
pub async fn creer(
    ctx: Context<'_>,
    #[description = "Fil d'accueil"]
    #[autocomplete = "autocomplete_thread"]
    fil: String,
    #[description = "Titre affiché sur la carte"] titre: String,
    #[description = "Texte de la carte (\\n pour un saut de ligne)"] texte: String,
    #[description = "Rôle accordé. Sans rôle, aucune réaction n'est posée"] role: Option<
        serenity::Role,
    >,
    #[description = "Couleur de la carte, ex. #99AAB5 pour la griser. Défaut : celle du fil"]
    couleur: Option<String>,
    #[description = "Rang dans le fil. Défaut : à la suite"] position: Option<i64>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let titre = titre.trim().to_owned();
    if titre.is_empty() {
        ctx.say("Le titre ne peut pas être vide.").await?;
        return Ok(());
    }
    let Some(category) = db::categories::by_name(&ctx.data().db, &fil).await? else {
        ctx.say(format!("Le fil **{fil}** n'existe pas.")).await?;
        return Ok(());
    };

    let role_id = role.as_ref().map(|role| ids::to_db(role.id.get()));
    if let Some(role_id) = role_id
        && let Some(existing) = db::posts::by_role(&ctx.data().db, role_id).await?
    {
        ctx.say(format!(
            "Ce rôle est déjà accordé par le message **{}**. Un rôle n'appartient qu'à un message.",
            existing.title
        ))
        .await?;
        return Ok(());
    }

    let (body, unknown) = commands::link_mentions(ctx, embed::format_description(&texte)).await?;
    let stored = db::posts::insert(
        &ctx.data().db,
        &Post {
            id: 0,
            category_id: category.id,
            slug: slug_from(&titre),
            title: titre.clone(),
            body,
            role_id,
            colour: couleur.as_deref().map(commands::parse_colour).transpose()?,
            message_id: None,
            position: match position {
                Some(rank) => rank,
                None => db::posts::next_position(&ctx.data().db, category.id).await?,
            },
        },
    )
    .await?;

    channel::publish(ctx.http(), ctx.data(), &category, &stored).await?;
    ctx.data().reload_caches().await?;

    // Le rôle parent vient du fil : il s'ajoute sans qu'on ait à le préciser.
    let granted = match (stored.role_id, category.parent_role_id) {
        (None, _) => " Aucun rôle : pas de réaction posée.".to_owned(),
        (Some(role), None) => format!(" Il accorde <@&{role}>."),
        (Some(role), Some(parent)) => format!(" Il accorde <@&{role}> et <@&{parent}>."),
    };
    ctx.say(format!(
        "Message **{titre}** publié dans <#{}>.{granted}{}",
        category.channel_id,
        commands::unknown_handles_note(&unknown)
    ))
    .await?;
    Ok(())
}

/// Modifier un message. Les paramètres omis restent inchangés.
///
/// L'arité vient de Discord : chaque champ modifiable est une option de la
/// commande, il n'y a pas de structure à regrouper côté interface.
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command)]
pub async fn modifier(
    ctx: Context<'_>,
    #[description = "Message à modifier"]
    #[autocomplete = "autocomplete_post"]
    message: String,
    #[description = "Nouveau titre"] titre: Option<String>,
    #[description = "Nouveau texte (\\n pour un saut de ligne)"] texte: Option<String>,
    #[description = "Nouveau rôle accordé"] role: Option<serenity::Role>,
    #[description = "Couleur de la carte, #99AAB5 pour la griser, - pour revenir à celle du fil"]
    couleur: Option<String>,
    #[description = "Déplacer vers un autre fil"]
    #[autocomplete = "autocomplete_thread"]
    fil: Option<String>,
    #[description = "Nouveau rang dans le fil"] position: Option<i64>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(current) = resolve(ctx, &message).await? else {
        ctx.say("Message introuvable. Choisissez-le dans les suggestions.")
            .await?;
        return Ok(());
    };
    let Some(old_thread) = db::categories::by_id(&ctx.data().db, current.category_id).await? else {
        ctx.say("Le fil de ce message est introuvable, la base est incohérente.")
            .await?;
        return Ok(());
    };

    let new_thread = match fil.as_deref() {
        None => old_thread.clone(),
        Some(name) => match db::categories::by_name(&ctx.data().db, name).await? {
            Some(category) => category,
            None => {
                ctx.say(format!("Le fil **{name}** n'existe pas.")).await?;
                return Ok(());
            }
        },
    };

    let role_id = match &role {
        None => current.role_id,
        Some(role) => Some(ids::to_db(role.id.get())),
    };
    if let Some(role_id) = role_id
        && role.is_some()
        && let Some(existing) = db::posts::by_role(&ctx.data().db, role_id).await?
        && existing.id != current.id
    {
        ctx.say(format!(
            "Ce rôle est déjà accordé par le message **{}**.",
            existing.title
        ))
        .await?;
        return Ok(());
    }

    let (body, unknown) = match texte.as_deref() {
        Some(text) => commands::link_mentions(ctx, embed::format_description(text)).await?,
        None => (current.body.clone(), Vec::new()),
    };
    let updated = Post {
        title: titre
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| current.title.clone()),
        body,
        role_id,
        colour: match couleur.as_deref().map(str::trim) {
            None => current.colour,
            Some(commands::CLEAR_SENTINEL) => None,
            Some(value) => Some(commands::parse_colour(value)?),
        },
        category_id: new_thread.id,
        position: position.unwrap_or(current.position),
        ..current.clone()
    };
    db::posts::update(&ctx.data().db, &updated).await?;

    if new_thread.id == old_thread.id {
        // Édition sur place : les réactions et les rôles déjà accordés survivent.
        channel::refresh(ctx.http(), ctx.data(), &new_thread, &updated).await?;
    } else {
        channel::remove(ctx.http(), ctx.data(), &old_thread, &updated).await?;
        channel::publish(
            ctx.http(),
            ctx.data(),
            &new_thread,
            &Post {
                message_id: None,
                ..updated.clone()
            },
        )
        .await?;
    }
    ctx.data().reload_caches().await?;

    ctx.say(format!(
        "Message **{}** modifié.{}",
        updated.title,
        commands::unknown_handles_note(&unknown)
    ))
    .await?;
    Ok(())
}

/// Supprimer la carte d'un message.
///
/// Le rôle reste sur le serveur, et ceux qui le portent le gardent : un rôle
/// règle souvent l'accès à des salons, le détruire se décide à la main dans
/// Discord, pas en effet de bord d'une commande.
#[poise::command(slash_command)]
pub async fn supprimer(
    ctx: Context<'_>,
    #[description = "Message à supprimer"]
    #[autocomplete = "autocomplete_post"]
    message: String,
    #[description = "Confirmer la suppression de la carte"] confirmer: bool,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(current) = resolve(ctx, &message).await? else {
        ctx.say("Message introuvable. Choisissez-le dans les suggestions.")
            .await?;
        return Ok(());
    };
    let Some(thread) = db::categories::by_id(&ctx.data().db, current.category_id).await? else {
        ctx.say("Le fil de ce message est introuvable, la base est incohérente.")
            .await?;
        return Ok(());
    };

    if !confirmer {
        let effect = match current.role_id {
            Some(role) => format!(
                "Le rôle <@&{role}> restera sur le serveur, et ceux qui le portent le garderont."
            ),
            None => "Ce message n'accorde aucun rôle.".to_owned(),
        };
        ctx.say(format!(
            "Supprimer **{}** effacera sa carte de <#{}>. {effect}\nRelancez avec `confirmer: True`.",
            current.title, thread.channel_id
        ))
        .await?;
        return Ok(());
    }

    channel::remove(ctx.http(), ctx.data(), &thread, &current).await?;
    db::posts::delete(&ctx.data().db, current.id).await?;
    ctx.data().reload_caches().await?;

    let mut report = format!("Message **{}** supprimé.", current.title);
    if let Some(role) = current.role_id {
        report.push_str(&format!(
            " Le rôle <@&{role}> est conservé : supprimez-le dans Discord s'il n'a plus lieu d'être."
        ));
    }
    ctx.say(report).await?;
    Ok(())
}

/// Lister les messages d'un fil, ou de tout le forum.
#[poise::command(slash_command)]
pub async fn liste(
    ctx: Context<'_>,
    #[description = "Fil à détailler. Défaut : tous"]
    #[autocomplete = "autocomplete_thread"]
    fil: Option<String>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let categories = match fil.as_deref() {
        None => db::categories::list(&ctx.data().db).await?,
        Some(name) => match db::categories::by_name(&ctx.data().db, name).await? {
            Some(category) => vec![category],
            None => {
                ctx.say(format!("Le fil **{name}** n'existe pas.")).await?;
                return Ok(());
            }
        },
    };

    let mut sections = Vec::new();
    for category in &categories {
        let posts = db::posts::by_category(&ctx.data().db, category.id).await?;
        let lines: Vec<String> = posts
            .iter()
            .map(|post| {
                let roles = match (post.role_id, category.parent_role_id) {
                    (None, _) => "information".to_owned(),
                    (Some(role), None) => format!("<@&{role}>"),
                    (Some(role), Some(parent)) => format!("<@&{role}> + <@&{parent}>"),
                };
                format!("  {}. **{}** — {roles}", post.position, post.title)
            })
            .collect();

        sections.push(if lines.is_empty() {
            format!("**{}** — aucun message", category.name)
        } else {
            format!("**{}**\n{}", category.name, lines.join("\n"))
        });
    }

    if sections.is_empty() {
        ctx.say("Aucun fil configuré.").await?;
        return Ok(());
    }
    commands::say_long(ctx, sections.join("\n\n")).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_stable_and_readable() {
        assert_eq!(slug_from("Fresque de l'IA"), "fresque-de-l-ia");
        assert_eq!(slug_from("Groupes locaux"), "groupes-locaux");
        // Les accents sont conservés : ils restent lisibles et le slug n'est
        // jamais présenté à Discord, seulement à un relecteur du fichier.
        assert_eq!(slug_from("Créa photo /vidéo"), "créa-photo--vidéo");
    }

    #[test]
    fn a_title_without_letters_still_yields_a_slug() {
        assert_eq!(slug_from("---"), "message");
        assert_eq!(slug_from(""), "message");
    }
}

//! `/forum fil …` — gestion des fils du forum.
//!
//! Ces enregistrements se créaient dans l'interface de Directus. La base étant
//! désormais un simple fichier SQLite sans interface, ces commandes et l'import
//! du fichier de contenu sont les seuls moyens de configurer le bot.

use crate::commands::{self, CLEAR_SENTINEL};
use crate::db;
use crate::db::categories::Category;
use crate::discord::channel;
use crate::ids;
use crate::rules;
use crate::state::{Context, Error};
use poise::ChoiceParameter as _;
use poise::serenity_prelude as serenity;

/// Propose les fils existants pendant la saisie.
///
/// Remplace l'énumération figée de l'ancien bot, calculée une fois à l'import :
/// un fil créé n'apparaissait qu'après redémarrage.
pub async fn autocomplete_thread(ctx: Context<'_>, partial: &str) -> Vec<String> {
    let Ok(categories) = db::categories::list(&ctx.data().db).await else {
        return Vec::new();
    };
    let needle = db::categories::name_key(partial);
    categories
        .into_iter()
        .filter(|category| db::categories::name_key(&category.name).contains(&needle))
        // Discord n'affiche que 25 suggestions.
        .take(25)
        .map(|category| category.name)
        .collect()
}

#[poise::command(
    slash_command,
    rename = "fil",
    subcommands("creer", "modifier", "mp", "confirmation", "supprimer", "liste")
)]
pub async fn fil(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Créer un fil du forum.
#[poise::command(slash_command, rename = "créer")]
pub async fn creer(
    ctx: Context<'_>,
    #[description = "Nom affiché du fil"] nom: String,
    #[description = "Fil ou salon où publier les cartes"] salon: serenity::PartialChannel,
    #[description = "Couleur des cartes, ex. #FF6600"] couleur: Option<String>,
    #[description = "URL de l'illustration publiée en tête"] illustration: Option<String>,
    #[description = "Texte d'introduction (\\n pour un saut de ligne)"] description: Option<String>,
    #[description = "Rôle parent : accordé en plus du sien par chaque message du fil"]
    role_parent: Option<serenity::Role>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let nom = nom.trim().to_owned();
    if nom.is_empty() {
        ctx.say("Le nom du fil ne peut pas être vide.").await?;
        return Ok(());
    }
    if !commands::is_writable(salon.kind) {
        ctx.say("Ce type de salon ne peut pas recevoir de messages. Choisissez un fil de forum ou un salon textuel.")
            .await?;
        return Ok(());
    }
    if db::categories::by_name(&ctx.data().db, &nom)
        .await?
        .is_some()
    {
        ctx.say(format!("Le fil **{nom}** existe déjà.")).await?;
        return Ok(());
    }
    let channel_id = ids::to_db(salon.id.get());
    if let Some(existing) = db::categories::by_channel(&ctx.data().db, channel_id).await? {
        ctx.say(format!(
            "<#{}> accueille déjà le fil **{}**.",
            salon.id, existing.name
        ))
        .await?;
        return Ok(());
    }

    if let Some(excess) = description
        .as_deref()
        .and_then(commands::too_long_for_a_message)
    {
        ctx.say(format!(
            "Introduction trop longue : {excess}. Elle est publiée telle quelle, Discord la refuserait."
        ))
        .await?;
        return Ok(());
    }

    let category = Category {
        id: 0,
        name: nom.clone(),
        channel_id,
        header_image_url: commands::apply_optional(None, illustration, commands::parse_url)?,
        header_text: description.map(|text| text.trim().to_owned()),
        colour: couleur.as_deref().map(commands::parse_colour).transpose()?,
        parent_role_id: role_parent.map(|role| ids::to_db(role.id.get())),
        dm_text: None,
        confirmations: false,
        notify_channel_id: None,
        joined_text: None,
        left_text: None,
    };
    db::categories::insert(&ctx.data().db, &category).await?;
    ctx.data().reload_caches().await?;

    ctx.say(format!(
        "Fil **{nom}** créé sur <#{}>.\nAjoutez-y des messages avec `/forum message créer`, ou importez le fichier de contenu.",
        salon.id
    ))
    .await?;
    Ok(())
}

/// Modifier un fil. Les paramètres omis restent inchangés, `-` efface.
///
/// L'arité vient de Discord : chaque champ modifiable est une option de la
/// commande, il n'y a pas de structure à regrouper côté interface.
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command)]
pub async fn modifier(
    ctx: Context<'_>,
    #[description = "Fil à modifier"]
    #[autocomplete = "autocomplete_thread"]
    fil: String,
    #[description = "Nouveau nom"] nouveau_nom: Option<String>,
    #[description = "Nouveau fil ou salon de publication"] salon: Option<serenity::PartialChannel>,
    #[description = "Couleur #RRGGBB, ou - pour l'enlever"] couleur: Option<String>,
    #[description = "URL de l'illustration, ou - pour l'enlever"] illustration: Option<String>,
    #[description = "Texte d'introduction, ou - pour l'enlever"] description: Option<String>,
    #[description = "Rôle parent du fil, accordé par tous ses messages"] role_parent: Option<
        serenity::Role,
    >,
    #[description = "Retirer le rôle parent du fil"] sans_role_parent: Option<bool>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(current) = db::categories::by_name(&ctx.data().db, &fil).await? else {
        ctx.say(format!("Aucun fil nommé **{fil}**.")).await?;
        return Ok(());
    };

    if let Some(channel) = &salon
        && !commands::is_writable(channel.kind)
    {
        ctx.say("Ce type de salon ne peut pas recevoir de messages.")
            .await?;
        return Ok(());
    }

    let renamed = nouveau_nom
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());
    if let Some(name) = renamed
        && let Some(clash) = db::categories::by_name(&ctx.data().db, name).await?
        && clash.id != current.id
    {
        ctx.say(format!("Un autre fil s'appelle déjà **{name}**."))
            .await?;
        return Ok(());
    }

    let moved_to = salon.as_ref().map(|channel| ids::to_db(channel.id.get()));
    if let Some(channel_id) = moved_to
        && let Some(clash) = db::categories::by_channel(&ctx.data().db, channel_id).await?
        && clash.id != current.id
    {
        ctx.say(format!(
            "Le salon visé accueille déjà le fil **{}**.",
            clash.name
        ))
        .await?;
        return Ok(());
    }

    if let Some(excess) = description
        .as_deref()
        .and_then(commands::too_long_for_a_message)
    {
        ctx.say(format!(
            "Introduction trop longue : {excess}. Elle est publiée telle quelle, Discord la refuserait."
        ))
        .await?;
        return Ok(());
    }

    let updated = Category {
        name: renamed
            .map(str::to_owned)
            .unwrap_or_else(|| current.name.clone()),
        channel_id: moved_to.unwrap_or(current.channel_id),
        header_image_url: commands::apply_optional(
            current.header_image_url.clone(),
            illustration,
            commands::parse_url,
        )?,
        header_text: commands::apply_optional(current.header_text.clone(), description, |text| {
            Ok(text.to_owned())
        })?,
        colour: match couleur.as_deref().map(str::trim) {
            None => current.colour,
            Some(CLEAR_SENTINEL) => None,
            Some(value) => Some(commands::parse_colour(value)?),
        },
        parent_role_id: match (role_parent, sans_role_parent) {
            (Some(role), _) => Some(ids::to_db(role.id.get())),
            (None, Some(true)) => None,
            (None, _) => current.parent_role_id,
        },
        ..current.clone()
    };
    db::categories::update(&ctx.data().db, &updated).await?;
    ctx.data().reload_caches().await?;

    // Un changement de salon laisse les cartes derrière lui : on vide l'ancien
    // et on republie dans le nouveau.
    if moved_to.is_some_and(|channel_id| channel_id != current.channel_id) {
        channel::purge_channel(ctx.http(), ids::channel(current.channel_id)).await?;
        let published = channel::rewrite(ctx.http(), ctx.data(), &updated).await?;
        ctx.data().reload_caches().await?;
        ctx.say(format!(
            "Fil **{}** déplacé sur <#{}>, {published} message·s republié·s.",
            updated.name, updated.channel_id
        ))
        .await?;
        return Ok(());
    }

    // Le rôle parent et la couleur du fil figurent sur chaque carte : les
    // changer demande de les rafraîchir, sur place, sans perdre les réactions.
    if updated.parent_role_id != current.parent_role_id || updated.colour != current.colour {
        for post in db::posts::by_category(&ctx.data().db, updated.id).await? {
            channel::refresh(ctx.http(), ctx.data(), &updated, &post).await?;
        }
    }

    ctx.say(format!(
        "Fil **{}** modifié. Utilisez `/forum republier` si l'introduction ou l'illustration doivent être republiées.",
        updated.name
    ))
    .await?;
    Ok(())
}

/// Affiche les messages privés de tous les fils, un message par fil.
///
/// Chacun peut faire 2000 caractères : les empiler dans une seule réponse
/// dépasserait la limite de Discord, d'où l'envoi séparé.
async fn show_all(ctx: Context<'_>) -> Result<(), Error> {
    let categories = db::categories::list(&ctx.data().db).await?;
    if categories.is_empty() {
        ctx.say("Aucun fil n'est encore configuré.").await?;
        return Ok(());
    }

    for category in &categories {
        let body = match category.dm_text.as_deref() {
            Some(text) if !text.trim().is_empty() => text,
            _ => "*aucun message privé*",
        };
        ctx.say(fit(&format!("**{}**\n\n{body}", category.name)))
            .await?;
    }

    // Les cartes qui portent leur propre message privé passent avant celui du
    // fil : les omettre ici donnerait une liste fausse.
    let posts = db::posts::all(&ctx.data().db).await?;
    for post in &posts {
        let Some(text) = post
            .dm_text
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            continue;
        };
        ctx.say(fit(&format!(
            "**{}** *(carte, remplace le message privé de son fil)*\n\n{text}",
            post.title
        )))
        .await?;
    }
    Ok(())
}

/// Tronque proprement ce qui dépasserait la limite d'un message Discord.
fn fit(text: &str) -> String {
    const SUFFIX: &str = "\n\n*(tronqué)*";
    if text.chars().count() <= commands::MAX_MESSAGE_CHARS {
        return text.to_owned();
    }
    let keep = commands::MAX_MESSAGE_CHARS - SUFFIX.chars().count();
    text.chars().take(keep).collect::<String>() + SUFFIX
}

/// Configurer le message privé envoyé à la première réaction dans ce fil.
#[poise::command(slash_command)]
pub async fn mp(
    ctx: Context<'_>,
    #[description = "Fil concerné. Sans lui, affiche les messages privés de tous les fils"]
    #[autocomplete = "autocomplete_thread"]
    fil: Option<String>,
    #[description = "Texte du message privé, ou - pour ne plus rien envoyer"] texte: Option<String>,
    #[description = "Identifiant d'un message à recopier comme modèle"] depuis_message: Option<
        String,
    >,
    #[description = "Salon du message modèle. Défaut : le salon courant"] salon: Option<
        serenity::PartialChannel,
    >,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(fil) = fil else {
        return show_all(ctx).await;
    };
    let Some(current) = db::categories::by_name(&ctx.data().db, &fil).await? else {
        ctx.say(format!("Aucun fil nommé **{fil}**.")).await?;
        return Ok(());
    };

    // Le texte est recopié une fois pour toutes : le modèle peut ensuite être
    // supprimé sans que le message privé cesse de partir.
    let text = match (texte.as_deref().map(str::trim), depuis_message.as_deref()) {
        (Some(CLEAR_SENTINEL), _) => None,
        (Some(text), _) if !text.is_empty() => {
            Some(crate::discord::embed::format_description(text))
        }
        (_, Some(raw)) => {
            let message_id = serenity::MessageId::new(commands::parse_snowflake(raw)?);
            let source = salon
                .map(|channel| channel.id)
                .unwrap_or_else(|| ctx.channel_id());
            match source.message(ctx.http(), message_id).await {
                Ok(message) => Some(message.content.to_string()),
                Err(err) => {
                    ctx.say(format!(
                        "Message `{message_id}` introuvable dans <#{source}> ({err}). Vérifiez le salon."
                    ))
                    .await?;
                    return Ok(());
                }
            }
        }
        _ => {
            let state = match current.dm_text.as_deref() {
                Some(text) if !text.trim().is_empty() => {
                    format!("Message privé actuel de **{}** :\n\n{text}", current.name)
                }
                _ => format!("**{}** n'envoie aucun message privé.", current.name),
            };
            ctx.say(state).await?;
            return Ok(());
        }
    };

    if let Some(excess) = text.as_deref().and_then(commands::too_long_for_a_message) {
        ctx.say(format!(
            "Message privé non enregistré : {excess}. Discord refuserait de l'envoyer."
        ))
        .await?;
        return Ok(());
    }

    db::categories::update(
        &ctx.data().db,
        &Category {
            dm_text: text.clone(),
            ..current.clone()
        },
    )
    .await?;
    ctx.data().reload_caches().await?;

    // Les membres qui ont déjà réagi ne sont pas notifiés : le texte vit en
    // base, aucun message Discord n'est modifié.
    match text {
        Some(_) => {
            ctx.say(format!(
                "Message privé de **{}** enregistré. Les membres qui ont déjà réagi ne sont pas notifiés, et ne le recevront pas de nouveau.",
                current.name
            ))
            .await?
        }
        None => {
            ctx.say(format!(
                "**{}** n'enverra plus de message privé.",
                current.name
            ))
            .await?
        }
    };
    Ok(())
}

/// Lequel des deux messages de confirmation on règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, poise::ChoiceParameter)]
pub enum Moment {
    #[name = "arrivée"]
    Arrivee,
    #[name = "départ"]
    Depart,
}

/// Consulter, réécrire ou rétablir une confirmation d'entrée ou de sortie.
///
/// Ces messages partent à chaque main levée ou baissée, là où le message privé
/// d'accueil ne part qu'une fois : une coquille s'y répète.
#[poise::command(slash_command)]
pub async fn confirmation(
    ctx: Context<'_>,
    #[description = "Fil concerné"]
    #[autocomplete = "autocomplete_thread"]
    fil: String,
    #[description = "Message d'arrivée ou de départ"] moment: Moment,
    #[description = "Nouveau texte, ou - pour revenir à celui d'origine"] texte: Option<String>,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(current) = db::categories::by_name(&ctx.data().db, &fil).await? else {
        ctx.say(format!("Aucun fil nommé **{fil}**.")).await?;
        return Ok(());
    };
    let (stored, default) = match moment {
        Moment::Arrivee => (&current.joined_text, rules::JOINED_DEFAULT),
        Moment::Depart => (&current.left_text, rules::LEFT_DEFAULT),
    };

    let Some(texte) = texte
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    else {
        let (origine, texte) = match stored.as_deref() {
            Some(text) if !text.trim().is_empty() => ("propre au fil", text),
            _ => ("par défaut", default),
        };
        ctx.say(fit(&format!(
            "Confirmation d'{} de **{}** *({origine})* :\n\n{texte}\n\nMarqueurs disponibles : `{{carte}}` et `{{rôles}}`.",
            moment.name(),
            current.name
        )))
        .await?;
        return Ok(());
    };

    let text = match texte {
        CLEAR_SENTINEL => None,
        raw => Some(crate::discord::embed::format_description(raw)),
    };

    // Un marqueur mal orthographié partirait tel quel à chaque membre : mieux
    // vaut refuser la saisie que découvrir `{crate}` dans les messages privés.
    if let Some(raw) = text.as_deref() {
        let unknown = rules::unknown_markers(raw);
        if !unknown.is_empty() {
            ctx.say(format!(
                "Marqueur inconnu : {}. Seuls `{{carte}}` et `{{rôles}}` sont remplacés.",
                unknown.join(", ")
            ))
            .await?;
            return Ok(());
        }
        if let Some(excess) = commands::too_long_for_a_message(raw) {
            ctx.say(format!(
                "Confirmation non enregistrée : {excess}. Discord refuserait de l'envoyer."
            ))
            .await?;
            return Ok(());
        }
    }

    let updated = match moment {
        Moment::Arrivee => Category {
            joined_text: text.clone(),
            ..current.clone()
        },
        Moment::Depart => Category {
            left_text: text.clone(),
            ..current.clone()
        },
    };
    db::categories::update(&ctx.data().db, &updated).await?;
    ctx.data().reload_caches().await?;

    let rappel = if current.confirmations {
        String::new()
    } else {
        format!(
            "\n\nAttention : **{}** n'envoie aucune confirmation pour l'instant. Activez `confirmations` avec `/forum fil modifier`.",
            current.name
        )
    };
    let report = match text {
        Some(text) => format!(
            "Confirmation d'{} de **{}** enregistrée :\n\n{text}{rappel}",
            moment.name(),
            current.name
        ),
        None => format!(
            "Confirmation d'{} de **{}** rétablie :\n\n{default}{rappel}",
            moment.name(),
            current.name
        ),
    };
    ctx.say(fit(&report)).await?;
    Ok(())
}

/// Supprimer un fil, ses messages et ses cartes.
#[poise::command(slash_command)]
pub async fn supprimer(
    ctx: Context<'_>,
    #[description = "Fil à supprimer"]
    #[autocomplete = "autocomplete_thread"]
    fil: String,
    #[description = "Confirmer la suppression du fil et de ses cartes"] confirmer: bool,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(current) = db::categories::by_name(&ctx.data().db, &fil).await? else {
        ctx.say(format!("Aucun fil nommé **{fil}**.")).await?;
        return Ok(());
    };
    let posts = db::posts::by_category(&ctx.data().db, current.id).await?;

    if !confirmer {
        ctx.say(format!(
            "**{}** contient {} message·s. Les supprimer effacera les cartes de <#{}>. Les rôles Discord, eux, restent sur le serveur, et les membres qui les portent les gardent.\nRelancez avec `confirmer: True`.",
            current.name,
            posts.len(),
            current.channel_id
        ))
        .await?;
        return Ok(());
    }

    channel::purge_channel(ctx.http(), ids::channel(current.channel_id)).await?;
    // Les messages partent avec le fil par cascade, ainsi que les envois de
    // messages privés déjà mémorisés.
    db::categories::delete(&ctx.data().db, current.id).await?;
    ctx.data().reload_caches().await?;

    ctx.say(format!(
        "Fil **{}** supprimé, {} message·s retiré·s.",
        current.name,
        posts.len()
    ))
    .await?;
    Ok(())
}

/// Lister les fils et leur configuration.
#[poise::command(slash_command)]
pub async fn liste(ctx: Context<'_>) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let categories = db::categories::list(&ctx.data().db).await?;
    if categories.is_empty() {
        ctx.say("Aucun fil. Commencez par `/forum fil créer`, ou importez le fichier de contenu.")
            .await?;
        return Ok(());
    }

    let mut lines = Vec::with_capacity(categories.len());
    for category in &categories {
        let count = db::posts::by_category(&ctx.data().db, category.id)
            .await?
            .len();
        let colour = category
            .colour
            .map(|value| format!("#{value:06X}"))
            .unwrap_or_else(|| "sans couleur".to_owned());
        let parent = category
            .parent_role_id
            .map(|role| format!(", rôle parent <@&{role}>"))
            .unwrap_or_default();
        let dm = if category
            .dm_text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        {
            ", message privé actif"
        } else {
            ""
        };
        lines.push(format!(
            "- **{}** dans <#{}> : {count} message·s, {colour}{parent}{dm}",
            category.name, category.channel_id
        ));
    }

    commands::say_long(ctx, format!("Fils du forum :\n{}", lines.join("\n"))).await?;
    Ok(())
}

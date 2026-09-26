//! `/forum message …` — création, édition et suppression des messages du forum.
//!
//! Chaque opération ne touche que la carte concernée. L'ancien bot réécrivait
//! le salon entier à chaque fois, ce qui effaçait au passage les réactions déjà
//! posées par les membres — exactement ce que le CDC interdit.

use crate::commands::threads::DirectMessageModal;
use crate::commands::{self, threads::autocomplete_thread};
use crate::db;
use crate::db::posts::Post;
use crate::discord::channel;
use crate::discord::embed;
use crate::ids;
use crate::state::{Context, Data, Error};
use poise::Modal as _;
use poise::serenity_prelude as serenity;

/// Motif inscrit au journal d'audit du serveur, pour qu'un rôle disparu
/// s'explique sans avoir à relire les journaux du bot.
const REASON_DELETE: &str = "suppression de la carte du forum";

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

/// Titre retenu après édition : le champ saisi, ou l'ancien s'il est vidé.
///
/// Discord impose déjà un titre non vide (champ requis du modal) ; ce repli
/// couvre le cas d'un texte réduit à des espaces.
fn chosen_title(current: &str, input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        current.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Fenêtre d'édition du titre et du texte d'une carte.
///
/// Un modal offre une vraie zone de texte multiligne, là où un paramètre de
/// commande slash tient sur une ligne et force à écrire les sauts en `\n`.
#[derive(Debug, poise::Modal)]
#[name = "Éditer le message"]
struct EditModal {
    #[name = "Titre"]
    #[max_length = 256]
    titre: String,
    #[name = "Texte"]
    #[paragraph]
    #[max_length = 4000]
    texte: Option<String>,
}

#[poise::command(
    slash_command,
    rename = "message",
    subcommands("creer", "modifier", "editer", "mp", "supprimer", "liste")
)]
pub async fn message(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Éditer le titre et le texte d'un message, dans une fenêtre multiligne.
///
/// Contrairement aux autres commandes, elle ne diffère pas la réponse : un modal
/// doit être la toute première réponse à l'interaction. Le rôle, la couleur, le
/// fil et le rang restent du ressort de `/forum message modifier`.
#[poise::command(slash_command, rename = "éditer")]
pub async fn editer(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Message à éditer"]
    #[autocomplete = "autocomplete_post"]
    message: String,
) -> Result<(), Error> {
    let base = Context::Application(ctx);

    let Some(current) = resolve(base, &message).await? else {
        base.send(
            poise::CreateReply::default()
                .content("Message introuvable. Choisissez-le dans les suggestions.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };
    let Some(thread) = db::categories::by_id(&ctx.data().db, current.category_id).await? else {
        base.send(
            poise::CreateReply::default()
                .content("Le fil de ce message est introuvable, la base est incohérente.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    // Pré-remplissage avec le contenu actuel. Le corps stocké porte déjà ses
    // mentions sous forme `<@id>` : les re-traiter à la soumission est sans
    // effet, `link_handles` laissant intactes les mentions déjà écrites.
    let defaults = EditModal {
        titre: current.title.clone(),
        texte: Some(current.body.clone()).filter(|body| !body.is_empty()),
    };
    let Some(edited) = EditModal::execute_with_defaults(ctx, defaults).await? else {
        // Modal fermé sans soumission : Discord a déjà refermé la fenêtre.
        return Ok(());
    };

    let raw = edited.texte.unwrap_or_default();
    let (body, unknown) = commands::link_mentions(base, embed::format_description(&raw)).await?;
    let updated = Post {
        title: chosen_title(&current.title, &edited.titre),
        body,
        ..current.clone()
    };
    db::posts::update(&ctx.data().db, &updated).await?;
    // Édition sur place : les réactions et les rôles déjà accordés survivent.
    channel::refresh(ctx.http(), ctx.data(), &thread, &updated).await?;
    ctx.data().reload_caches().await?;

    base.send(
        poise::CreateReply::default()
            .content(format!(
                "Message **{}** modifié.{}",
                updated.title,
                commands::unknown_handles_note(&unknown)
            ))
            .ephemeral(true),
    )
    .await?;
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

    // Le slug se déduit du titre : deux titres proches se réduisent au même, et
    // l'index unique refusait l'insertion en renvoyant sa contrainte SQL brute.
    let slug = slug_from(&titre);
    if let Some(existing) = db::posts::by_slug(&ctx.data().db, category.id, &slug).await? {
        ctx.say(format!(
            "**{}** contient déjà un message sous l'identifiant `{slug}` : **{}**.\n\nChoisissez un titre différent, ou modifiez le message existant avec `/forum message éditer`.",
            category.name, existing.title
        ))
        .await?;
        return Ok(());
    }

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
            slug,
            title: titre.clone(),
            body,
            role_id,
            colour: couleur.as_deref().map(commands::parse_colour).transpose()?,
            message_id: None,
            position: match position {
                Some(rank) => rank,
                None => db::posts::next_position(&ctx.data().db, category.id).await?,
            },
            information: false,
            dm_text: None,
            grants_parent: true,
            notify_channel_id: None,
            referent_id: None,
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
    #[description = "Détacher le rôle de la carte. Le rôle Discord n'est pas supprimé"]
    #[rename = "retirer_rôle"]
    retirer_role: Option<bool>,
    #[description = "Couleur de la carte, #99AAB5 pour la griser, - pour revenir à celle du fil"]
    couleur: Option<String>,
    #[description = "Déplacer vers un autre fil"]
    #[autocomplete = "autocomplete_thread"]
    fil: Option<String>,
    #[description = "Nouveau rang dans le fil"] position: Option<i64>,
    #[description = "Désactiver : la carte reste affichée mais n'accorde plus rien"]
    desactiver: Option<bool>,
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

    // Le slug suit la carte quand elle change de fil : si le fil d'arrivée en a
    // déjà un du même nom, l'index unique refuserait la mise à jour.
    if new_thread.id != current.category_id
        && let Some(clash) =
            db::posts::by_slug(&ctx.data().db, new_thread.id, &current.slug).await?
    {
        ctx.say(format!(
            "**{}** contient déjà un message sous l'identifiant `{}` : **{}**.\n\nRenommez l'un des deux avant de déplacer celui-ci.",
            new_thread.name, current.slug, clash.title
        ))
        .await?;
        return Ok(());
    }

    // Détacher prime sur désigner : demander les deux à la fois n'a pas de sens,
    // et le refus le dit plutôt que de choisir à la place de l'appelant.
    let detach = retirer_role.unwrap_or(false);
    if detach && role.is_some() {
        ctx.say("Choisissez : un nouveau rôle, ou `retirer_rôle: True`, pas les deux.")
            .await?;
        return Ok(());
    }
    let role_id = match (&role, detach) {
        (_, true) => None,
        (None, _) => current.role_id,
        (Some(role), _) => Some(ids::to_db(role.id.get())),
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
        information: desactiver.unwrap_or(current.information),
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

    // Désactiver change ce que la carte fait, pas seulement ce qu'elle dit :
    // autant l'annoncer plutôt que de laisser vérifier.
    let etat = match (desactiver, current.information) {
        (Some(true), false) => " Elle est désactivée : plus de réaction, plus de rôle accordé.",
        (Some(false), true) => " Elle est réactivée : la main levée revient.",
        _ => "",
    };
    // Détacher un rôle ne le fait pas disparaître : sans cette précision, on
    // croirait la carte devenue inoffensive alors qu'elle accorde encore le
    // rôle parent de son fil.
    let detache = match (detach, current.role_id) {
        (true, Some(role)) => {
            let reste = match new_thread.parent_role_id {
                Some(parent) if !updated.information && updated.grants_parent => {
                    format!(" Elle accorde encore <@&{parent}>, le rôle du fil.")
                }
                _ => " Elle n'accorde plus aucun rôle.".to_owned(),
            };
            format!(" Le rôle <@&{role}> lui est détaché, il reste sur le serveur.{reste}")
        }
        _ => String::new(),
    };
    ctx.say(format!(
        "Message **{}** modifié.{etat}{detache}{}",
        updated.title,
        commands::unknown_handles_note(&unknown)
    ))
    .await?;
    Ok(())
}

/// Consulter, définir ou retirer le message privé propre à une carte.
///
/// Sans texte, il s'affiche. Une carte qui n'en a pas retombe sur celui de son
/// Afficher ou modifier le message privé propre à une carte.
///
/// Une fenêtre pré-remplie, comme pour le texte d'une carte. La vider rend la
/// carte au message privé de son fil : elle ne prive personne d'accueil.
#[poise::command(slash_command)]
pub async fn mp(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Message concerné"]
    #[autocomplete = "autocomplete_post"]
    message: String,
) -> Result<(), Error> {
    let base = Context::Application(ctx);

    let Some(current) = resolve(base, &message).await? else {
        commands::reply(
            base,
            "Message introuvable. Choisissez-le dans les suggestions.",
        )
        .await?;
        return Ok(());
    };
    let Some(thread) = db::categories::by_id(&ctx.data().db, current.category_id).await? else {
        commands::reply(
            base,
            "Le fil de ce message est introuvable, la base est incohérente.",
        )
        .await?;
        return Ok(());
    };

    let defaults = DirectMessageModal {
        texte: current
            .dm_text
            .clone()
            .filter(|text| !text.trim().is_empty()),
    };
    let Some(edited) = DirectMessageModal::execute_with_defaults(ctx, defaults).await? else {
        return Ok(());
    };

    let text = commands::submitted_text(edited.texte.as_deref());
    if let Some(excess) = text.as_deref().and_then(commands::too_long_for_a_message) {
        commands::reply(
            base,
            format!("Message privé non enregistré : {excess}. Discord refuserait de l'envoyer."),
        )
        .await?;
        return Ok(());
    }

    db::posts::update(
        &ctx.data().db,
        &Post {
            dm_text: text.clone(),
            ..current.clone()
        },
    )
    .await?;
    // Le message privé propre à une carte décide de son indexation sur le
    // chemin chaud : sans ce rechargement, la réaction ne le trouverait pas.
    base.data().reload_caches().await?;

    let report = match text {
        Some(_) => format!(
            "Message privé de **{}** enregistré. Ceux qui ont déjà levé la main ne le recevront pas.",
            current.title
        ),
        None => format!(
            "**{}** n'a plus de message privé à elle : ses mains levées recevront celui de **{}**.",
            current.title, thread.name
        ),
    };
    commands::reply(base, report).await?;
    Ok(())
}

/// Supprimer la carte d'un message.
///
/// Par défaut le rôle reste sur le serveur, et ceux qui le portent le gardent :
/// un rôle règle souvent l'accès à des salons, le détruire ne doit pas être un
/// effet de bord. `supprimer_role` le demande explicitement, pour la carte de
/// test qu'on retire sans vouloir laisser un rôle orphelin derrière elle.
#[poise::command(slash_command)]
pub async fn supprimer(
    ctx: Context<'_>,
    #[description = "Message à supprimer"]
    #[autocomplete = "autocomplete_post"]
    message: String,
    #[description = "Confirmer la suppression de la carte"] confirmer: bool,
    #[description = "Détruire aussi le rôle Discord. Défaut : non"]
    #[rename = "supprimer_rôle"]
    supprimer_role: Option<bool>,
    #[description = "Second accord, exigé pour détruire un rôle"]
    #[rename = "confirmer_rôle"]
    confirmer_role: Option<bool>,
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

    let drop_role = supprimer_role.unwrap_or(false);

    // Détruire un rôle retire l'accès à des salons à tous ceux qui le portent,
    // et rien ne le rend : ce geste se demande deux fois, séparément de la
    // suppression de la carte.
    if drop_role && confirmer && !confirmer_role.unwrap_or(false) {
        let porteurs = match current.role_id {
            Some(role) => format!(
                "Le rôle <@&{role}> va être **détruit**, et tous ceux qui le portent perdront \
                 l'accès aux salons qu'il ouvre. C'est sans retour."
            ),
            None => "Cette carte n'accorde aucun rôle : rien à détruire.".to_owned(),
        };
        ctx.say(format!(
            "{porteurs}\n\nSi c'est bien ce que vous voulez, relancez avec `confirmer: True` **et** `confirmer_rôle: True`."
        ))
        .await?;
        return Ok(());
    }

    if !confirmer {
        let effect = match (current.role_id, drop_role) {
            (Some(role), false) => format!(
                "Le rôle <@&{role}> restera sur le serveur, et ceux qui le portent le garderont. \
                 Ajoutez `supprimer_rôle: True` pour le détruire aussi."
            ),
            (Some(role), true) => format!(
                "Le rôle <@&{role}> sera **détruit** : tous ceux qui le portent le perdront, \
                 et les salons qu'il ouvrait leur seront fermés."
            ),
            (None, _) => "Ce message n'accorde aucun rôle.".to_owned(),
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
        if drop_role {
            // Le rôle est détruit après la carte : si Discord refuse (hiérarchie,
            // permissions), la carte est déjà partie et le message le dit, plutôt
            // que d'abandonner une suppression à moitié faite.
            match ctx
                .http()
                .delete_role(
                    ctx.data().config.guild_id,
                    ids::role(role),
                    Some(REASON_DELETE),
                )
                .await
            {
                Ok(()) => report.push_str(" Le rôle qu'il accordait a été détruit."),
                Err(err) => {
                    tracing::warn!(%err, role, "rôle non détruit");
                    report.push_str(&format!(
                        " En revanche le rôle <@&{role}> n'a pas pu être détruit : \
                         vérifiez qu'il est placé sous le rôle du bot."
                    ));
                }
            }
        } else {
            report.push_str(&format!(
                " Le rôle <@&{role}> est conservé : supprimez-le dans Discord s'il n'a plus lieu d'être."
            ));
        }
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
                // Sans rôle nominatif, une carte accorde quand même le rôle
                // parent de son fil : dire « information » serait faux.
                let granted: Vec<String> = [
                    post.role_id,
                    category.parent_role_id.filter(|_| post.grants_parent),
                ]
                .into_iter()
                .flatten()
                .map(|role_id| format!("<@&{role_id}>"))
                .collect();
                let has_dm = post
                    .dm_text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty());
                let effect = if post.information {
                    "information, aucune réaction".to_owned()
                } else if !granted.is_empty() {
                    granted.join(" + ")
                } else if has_dm {
                    "message privé seul, aucun rôle".to_owned()
                } else {
                    "n'accorde rien, aucune réaction".to_owned()
                };
                // Le slug est ce qui relie la carte au fichier de contenu :
                // sans lui, un réimport crée un doublon au lieu de la retrouver.
                format!(
                    "  {}. **{}** (`{}`) : {effect}",
                    post.position, post.title, post.slug
                )
            })
            .collect();

        sections.push(if lines.is_empty() {
            format!("**{}** : aucun message", category.name)
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
    fn two_close_titles_collide_on_the_same_slug() {
        // D'où le contrôle avant insertion : sans lui, l'index unique renvoyait
        // sa contrainte SQL à l'auteur de la commande.
        assert_eq!(slug_from("Veille"), slug_from("veille !"));
        assert_eq!(slug_from("Fresque de l'IA"), slug_from("Fresque de l’IA"));
    }

    #[test]
    fn a_title_without_letters_still_yields_a_slug() {
        assert_eq!(slug_from("---"), "message");
        assert_eq!(slug_from(""), "message");
    }

    #[test]
    fn an_edited_title_falls_back_to_the_old_one_when_blank() {
        assert_eq!(
            chosen_title("Paris", "Paris intra-muros"),
            "Paris intra-muros"
        );
        // Le titre est requis côté Discord ; ce repli couvre un champ d'espaces.
        assert_eq!(chosen_title("Paris", "   "), "Paris");
        assert_eq!(chosen_title("Paris", "  Lyon  "), "Lyon");
    }
}

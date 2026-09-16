//! Auto-assignation par réaction.
//!
//! Lever la main sur une carte accorde le rôle du message, et le rôle parent
//! du fil quand il y en a un. La première réaction dans un fil peut aussi
//! déclencher l'envoi d'un message privé.
//!
//! L'ancien bot récupérait le message par l'API puis cherchait un rôle Discord
//! portant le titre de son embed. Comme `posts.message_id` est mémorisé, la
//! résolution se fait maintenant en mémoire : plus aucun appel réseau pour
//! décider si une réaction nous concerne.

use crate::db;
use crate::ids;
use crate::rules;
use crate::state::{Data, PostRef};
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

const REASON_ADD: &str = "Ajouté par le Bot Rôles suite à une réaction.";
const REASON_REMOVE: &str = "Retiré par le Bot Rôles suite au retrait d'une réaction.";
const REASON_PARENT: &str =
    "Retiré par le Bot Rôles : plus aucun message du fil ne justifie ce rôle.";

pub async fn on_add(
    ctx: &serenity::Context,
    data: &Data,
    reaction: &serenity::Reaction,
) -> anyhow::Result<()> {
    let Some((guild_id, user_id, post)) = target(ctx, data, reaction) else {
        return Ok(());
    };

    let held = member_roles(ctx, guild_id, user_id, reaction.member.as_ref()).await?;

    let mut granted = Vec::new();
    for role_id in [post.role_id, post.parent_role_id].into_iter().flatten() {
        granted.push(role_id);
        if held.contains(&role_id) {
            continue;
        }
        tracing::info!(%user_id, role_id, "attribution du rôle sur réaction");
        ctx.http
            .add_member_role(guild_id, user_id, ids::role(role_id), Some(REASON_ADD))
            .await?;
    }

    // Une main levée sur un message sans rôle nominatif ne se retrouve pas dans
    // les rôles portés : on la mémorise pour savoir, à son retrait, s'il reste
    // une autre réaction du fil justifiant le rôle parent. Une carte qui
    // renonce au parent n'en justifie évidemment aucun.
    if post.role_id.is_none() && post.parent_role_id.is_some() {
        db::reactions::add(&data.db, ids::to_db(user_id.get()), post.post_id).await?;
    }

    // Le message privé vient après les rôles : il est un supplément, et son
    // échec ne doit jamais priver quelqu'un de son accès.
    let welcomed = send_welcome_dm(ctx, data, user_id, &post).await;

    // Pas deux messages d'affilée : le mot de bienvenue dit déjà où l'on vient
    // d'arriver, la confirmation ferait doublon.
    if !welcomed {
        confirm_change(ctx, data, user_id, &post, Change::Joined(&granted)).await;
    }
    announce(ctx, data, user_id, &post, Change::Joined(&granted)).await;
    Ok(())
}

pub async fn on_remove(
    ctx: &serenity::Context,
    data: &Data,
    reaction: &serenity::Reaction,
) -> anyhow::Result<()> {
    let Some((guild_id, user_id, post)) = target(ctx, data, reaction) else {
        return Ok(());
    };

    // L'événement de retrait ne porte pas le membre : il faut le lire.
    let held = member_roles(ctx, guild_id, user_id, None).await?;
    let member_id = ids::to_db(user_id.get());

    let mut taken = Vec::new();
    if let Some(role_id) = post.role_id
        && held.contains(&role_id)
    {
        tracing::info!(%user_id, role_id, "retrait du rôle sur retrait de réaction");
        ctx.http
            .remove_member_role(guild_id, user_id, ids::role(role_id), Some(REASON_REMOVE))
            .await?;
        taken.push(role_id);
    }

    // Oublier la main levée sur un message sans rôle, avant de réévaluer le parent.
    if post.role_id.is_none() && post.parent_role_id.is_some() {
        db::reactions::remove(&data.db, member_id, post.post_id).await?;
    }

    // Le rôle parent n'est repris que si plus rien ne le justifie, à deux
    // titres : un autre rôle nominatif du fil encore porté (Paris/Lyon), ou une
    // autre main levée du fil sur un message sans rôle.
    if let Some(parent) = post.parent_role_id
        && held.contains(&parent)
    {
        // Sans rôle nominatif, aucun rôle à exclure : `0` n'est jamais un rôle.
        let by_role = rules::parent_still_justified(
            &data.granting_roles(parent),
            &held,
            post.role_id.unwrap_or(0),
        );
        let by_reaction = db::reactions::parent_still_justified(
            &data.db,
            member_id,
            post.category_id,
            post.post_id,
        )
        .await?;
        if !by_role && !by_reaction {
            tracing::info!(%user_id, role_id = parent, "retrait du rôle parent, plus rien ne le justifie");
            ctx.http
                .remove_member_role(guild_id, user_id, ids::role(parent), Some(REASON_PARENT))
                .await?;
            taken.push(parent);
        }
    }

    // Un rôle repris sans un mot est la première source d'incompréhension :
    // le membre voit des salons disparaître sans savoir pourquoi.
    confirm_change(ctx, data, user_id, &post, Change::Left(&taken)).await;
    announce(ctx, data, user_id, &post, Change::Left(&taken)).await;
    Ok(())
}

/// Serveur, membre et message visés, si la réaction nous concerne.
fn target(
    ctx: &serenity::Context,
    data: &Data,
    reaction: &serenity::Reaction,
) -> Option<(serenity::GuildId, serenity::UserId, PostRef)> {
    let guild_id = reaction.guild_id?;
    if guild_id != data.config.guild_id {
        return None;
    }
    let user_id = reaction.user_id?;
    // Le bot pose lui-même la réaction modèle sur chaque carte.
    if user_id == ctx.cache.current_user().id {
        return None;
    }
    if !data.config.reaction_emoji.matches(&reaction.emoji) {
        return None;
    }
    // Absent de l'index : ce n'est pas une carte, ou c'est un message
    // d'information qui n'accorde rien.
    let post = data.post_for_message(ids::to_db(reaction.message_id.get()))?;
    Some((guild_id, user_id, post))
}

/// Rôles portés par le membre, tirés de l'événement quand il les porte.
async fn member_roles(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    from_event: Option<&serenity::Member>,
) -> anyhow::Result<HashSet<i64>> {
    let roles = match from_event {
        Some(member) => member.roles.clone(),
        None => guild_id.member(ctx, user_id).await?.roles.clone(),
    };
    Ok(roles
        .iter()
        .map(|role_id| ids::to_db(role_id.get()))
        .collect())
}

/// Envoie le message privé du fil, une seule fois par membre et par fil.
///
/// Rend `true` si un message est effectivement parti. Ne remonte jamais
/// d'erreur : un membre qui a fermé ses messages privés ne doit pas faire
/// échouer l'attribution de son rôle.
async fn send_welcome_dm(
    ctx: &serenity::Context,
    data: &Data,
    user_id: serenity::UserId,
    post: &PostRef,
) -> bool {
    // Le texte de la carte prime sur celui du fil : une carte qui dépareille
    // dans son fil a rarement le même accueil à donner.
    let own_dm = post.sends_own_dm;
    if !own_dm && !data.sends_dm(post.category_id) {
        return false;
    }
    let member_id = ids::to_db(user_id.get());

    // Réserver avant d'envoyer : deux réactions rapprochées sont traitées en
    // parallèle et liraient toutes deux une absence de ligne. La réservation
    // d'une carte est distincte de celle de son fil.
    let claimed = if own_dm {
        db::dm::claim_post(&data.db, member_id, post.post_id).await
    } else {
        db::dm::claim(&data.db, member_id, post.category_id).await
    };
    match claimed {
        Ok(false) => return false,
        Ok(true) => {}
        Err(err) => {
            tracing::error!(%err, "réservation du message privé impossible");
            return false;
        }
    }

    let text = if own_dm {
        match db::posts::by_id(&data.db, post.post_id).await {
            Ok(Some(found)) => found.dm_text.unwrap_or_default(),
            Ok(None) => return false,
            Err(err) => {
                tracing::error!(%err, "lecture du message privé de la carte impossible");
                return false;
            }
        }
    } else {
        match db::categories::by_id(&data.db, post.category_id).await {
            Ok(Some(category)) => category.dm_text.unwrap_or_default(),
            Ok(None) => return false,
            Err(err) => {
                tracing::error!(%err, "lecture du texte du message privé impossible");
                return false;
            }
        }
    };

    let sent = send_dm(ctx, user_id, &text).await;

    if let Err(err) = sent {
        if is_permanent(&err) {
            // Réessayer à chaque réaction du fil ne ferait qu'empiler des appels
            // voués au même refus : la réservation reste posée. Le membre garde
            // son rôle, et les liens du message privé figurent aussi dans la
            // description du fil, qu'il voit sans messages privés.
            tracing::info!(
                %user_id,
                %err,
                "membre injoignable en message privé, réservation conservée"
            );
            return false;
        }
        tracing::warn!(%user_id, %err, "message privé non délivré, réservation libérée");
        // Panne réseau ou indisponibilité de Discord : la prochaine réaction
        // dans ce fil retentera l'envoi.
        let released = if own_dm {
            db::dm::release_post(&data.db, member_id, post.post_id).await
        } else {
            db::dm::release(&data.db, member_id, post.category_id).await
        };
        if let Err(err) = released {
            tracing::error!(%err, "libération de la réservation impossible");
        }
        return false;
    }
    true
}

/// Ce qu'une réaction vient de changer, pour le dire au membre.
enum Change<'a> {
    Joined(&'a [i64]),
    Left(&'a [i64]),
}

/// Confirme en privé une entrée ou une sortie, quand le fil le demande.
///
/// Discord ne rend une réponse éphémère qu'à une interaction : une réaction
/// n'en est pas une, et le membre n'a donc aucun retour à l'écran. Sans ce
/// message, quitter une équipe par un clic malheureux ferait disparaître des
/// salons sans une explication.
async fn confirm_change(
    ctx: &serenity::Context,
    data: &Data,
    user_id: serenity::UserId,
    post: &PostRef,
    change: Change<'_>,
) {
    if !data.confirms_changes(post.category_id) {
        return;
    }
    let roles = match change {
        Change::Joined(roles) | Change::Left(roles) => roles,
    };
    // Rien n'a bougé : une réaction sur une carte qui n'accorde rien, ou un
    // rôle parent encore justifié par un autre groupe du fil.
    if roles.is_empty() {
        return;
    }
    let title = match db::posts::by_id(&data.db, post.post_id).await {
        Ok(Some(found)) => found.title,
        _ => return,
    };
    let mentions = roles
        .iter()
        .map(|role_id| format!("<@&{role_id}>"))
        .collect::<Vec<_>>()
        .join(" et ");

    let text = match change {
        Change::Joined(_) => format!(
            "🙋 Te voilà dans **{title}** !\n\nTu viens de recevoir {mentions}.\n\nTu changes d'avis ? Retire ta réaction sur la carte, le rôle est repris aussitôt."
        ),
        Change::Left(_) => format!(
            "Tu as quitté **{title}**, et {mentions} t'a été repris.\n\nLa porte reste ouverte : lève la main sur la carte quand tu veux revenir."
        ),
    };

    if let Err(err) = send_dm(ctx, user_id, &text).await {
        // Sans réservation ni reprise : une confirmation manquée n'empêche rien,
        // et le rôle, lui, a bien changé.
        tracing::info!(%user_id, %err, "confirmation non délivrée");
    }
}

/// Annonce le mouvement dans le salon du fil, et mentionne le·la référent·e.
///
/// Sans elle, personne n'apprend qu'une main s'est levée : le message privé
/// promet qu'on prendra contact, et rien ne prévient qui que ce soit.
async fn announce(
    ctx: &serenity::Context,
    data: &Data,
    user_id: serenity::UserId,
    post: &PostRef,
    change: Change<'_>,
) {
    let Ok(Some(card)) = db::posts::by_id(&data.db, post.post_id).await else {
        return;
    };
    // Le salon de la carte d'abord : chaque projet a le sien, et une annonce
    // tombée dans un salon commun ne serait lue par personne.
    let Some(channel_id) = card
        .notify_channel_id
        .or_else(|| data.notify_channel(post.category_id))
    else {
        return;
    };

    // La personne qui doit agir est nommée, et on lui dit quoi faire : une
    // annonce que personne ne s'attribue ne fait bouger personne.
    let text = match change {
        Change::Joined(_) => {
            let appel = match card.referent_id {
                Some(id) => format!("\n<@{id}>, à toi de l'accueillir."),
                None => String::new(),
            };
            format!(
                "🙋 <@{user_id}> vient de rejoindre **{}**.{appel}",
                card.title
            )
        }
        Change::Left(_) => format!("↩️ <@{user_id}> a quitté **{}**.", card.title),
    };

    // Mentionner sans notifier tout le serveur : seuls le membre et le·la
    // référente sont des mentions légitimes ici.
    let allowed = serenity::CreateAllowedMentions::new()
        .users(
            [Some(user_id), card.referent_id.map(ids::user)]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        )
        .everyone(false);
    let sent = ids::channel(channel_id)
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new()
                .content(text)
                .allowed_mentions(allowed),
        )
        .await;
    if let Err(err) = sent {
        tracing::warn!(%err, channel_id, "annonce non publiée");
    }
}

async fn send_dm(
    ctx: &serenity::Context,
    user_id: serenity::UserId,
    text: &str,
) -> serenity::Result<serenity::Message> {
    user_id
        .create_dm_channel(&ctx.http)
        .await?
        .send_message(&ctx.http, serenity::CreateMessage::new().content(text))
        .await
}

/// L'envoi est-il condamné, par opposition à une panne passagère ?
fn is_permanent(err: &serenity::Error) -> bool {
    match err {
        serenity::Error::Http(err) => err.status_code().is_some_and(refuses_for_good),
        _ => false,
    }
}

/// Un refus de Discord lui-même vaudra encore demain : messages privés fermés
/// (50007), bot bloqué, compte supprimé. Une limite de débit, elle, se retente,
/// comme tout ce qui vient du réseau ou d'une panne de Discord (5xx).
fn refuses_for_good(status: serenity::StatusCode) -> bool {
    status.is_client_error() && status != serenity::StatusCode::TOO_MANY_REQUESTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_direct_messages_are_never_retried() {
        assert!(refuses_for_good(serenity::StatusCode::FORBIDDEN));
    }

    #[test]
    fn a_rate_limit_is_worth_retrying() {
        assert!(!refuses_for_good(serenity::StatusCode::TOO_MANY_REQUESTS));
    }

    #[test]
    fn an_outage_at_discord_is_worth_retrying() {
        assert!(!refuses_for_good(serenity::StatusCode::BAD_GATEWAY));
    }

    #[test]
    fn a_network_failure_is_worth_retrying() {
        let err = serenity::Error::Other("connexion interrompue");
        assert!(!is_permanent(&err));
    }
}

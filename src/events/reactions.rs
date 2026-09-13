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

    for role_id in [Some(post.role_id), post.parent_role_id]
        .into_iter()
        .flatten()
    {
        if held.contains(&role_id) {
            continue;
        }
        tracing::info!(%user_id, role_id, "attribution du rôle sur réaction");
        ctx.http
            .add_member_role(guild_id, user_id, ids::role(role_id), Some(REASON_ADD))
            .await?;
    }

    // Le message privé vient après les rôles : il est un supplément, et son
    // échec ne doit jamais priver quelqu'un de son accès.
    send_welcome_dm(ctx, data, user_id, post.category_id).await;
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
    if held.contains(&post.role_id) {
        tracing::info!(%user_id, role_id = post.role_id, "retrait du rôle sur retrait de réaction");
        ctx.http
            .remove_member_role(
                guild_id,
                user_id,
                ids::role(post.role_id),
                Some(REASON_REMOVE),
            )
            .await?;
    }

    // Le rôle parent n'est repris que si plus rien ne le justifie : un membre
    // inscrit à Paris et à Lyon qui quitte Paris reste dans un groupe local.
    if let Some(parent) = post.parent_role_id
        && held.contains(&parent)
        && !rules::parent_still_justified(&data.granting_roles(parent), &held, post.role_id)
    {
        tracing::info!(%user_id, role_id = parent, "retrait du rôle parent, plus rien ne le justifie");
        ctx.http
            .remove_member_role(guild_id, user_id, ids::role(parent), Some(REASON_PARENT))
            .await?;
    }

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
/// Ne remonte jamais d'erreur : un membre qui a fermé ses messages privés ne
/// doit pas faire échouer l'attribution de son rôle.
async fn send_welcome_dm(
    ctx: &serenity::Context,
    data: &Data,
    user_id: serenity::UserId,
    category_id: i64,
) {
    if !data.sends_dm(category_id) {
        return;
    }
    let member_id = ids::to_db(user_id.get());

    // Réserver avant d'envoyer : deux réactions rapprochées sont traitées en
    // parallèle et liraient toutes deux une absence de ligne.
    match db::dm::claim(&data.db, member_id, category_id).await {
        Ok(false) => return,
        Ok(true) => {}
        Err(err) => {
            tracing::error!(%err, "réservation du message privé impossible");
            return;
        }
    }

    let text = match db::categories::by_id(&data.db, category_id).await {
        Ok(Some(category)) => category.dm_text.unwrap_or_default(),
        Ok(None) => return,
        Err(err) => {
            tracing::error!(%err, "lecture du texte du message privé impossible");
            return;
        }
    };

    let sent = async {
        user_id
            .create_dm_channel(&ctx.http)
            .await?
            .send_message(&ctx.http, serenity::CreateMessage::new().content(&text))
            .await
    }
    .await;

    if let Err(err) = sent {
        tracing::warn!(%user_id, %err, "message privé non délivré, réservation libérée");
        // Le membre a peut-être fermé ses messages privés : on libère pour que
        // la prochaine réaction réessaie.
        if let Err(err) = db::dm::release(&data.db, member_id, category_id).await {
            tracing::error!(%err, "libération de la réservation impossible");
        }
    }
}

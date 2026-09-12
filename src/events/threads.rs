//! Garde les fils du forum ouverts.
//!
//! Discord archive un fil après une période sans nouveau message — trois jours par défaut pour ceux du forum. Une réaction ne compte pas comme activité, et dans un fil archivé l'API refuse réactions comme éditions (`50083 Thread is archived`) : sans intervention, le forum cesserait de répondre au bout de quelques jours de calme. Le bot désarchive donc ses fils dès que Discord les archive, et au démarrage pour ceux archivés pendant qu'il était arrêté.
//!
//! Un fil **verrouillé** a été fermé exprès par un modérateur : le bot le laisse fermé.

use crate::db;
use crate::ids;
use crate::state::Data;
use poise::serenity_prelude as serenity;

/// Vrai si le fil doit être rouvert : archivé, mais pas verrouillé.
pub fn should_revive(archived: bool, locked: bool) -> bool {
    archived && !locked
}

pub async fn on_update(
    ctx: &serenity::Context,
    data: &Data,
    thread: &serenity::GuildChannel,
) -> anyhow::Result<()> {
    let Some(meta) = &thread.thread_metadata else {
        return Ok(());
    };
    if !should_revive(meta.archived, meta.locked) {
        return Ok(());
    }
    // Seuls nos fils nous concernent. L'événement est rare, une lecture en base suffit.
    if db::categories::by_channel(&data.db, ids::to_db(thread.id.get()))
        .await?
        .is_none()
    {
        return Ok(());
    }
    revive(ctx, thread.id, &thread.name).await
}

/// Rouvre au démarrage les fils archivés pendant que le bot était arrêté.
pub async fn revive_all(ctx: &serenity::Context, data: &Data) -> anyhow::Result<()> {
    for category in db::categories::list(&data.db).await? {
        let channel = match ids::channel(category.channel_id).to_channel(ctx).await {
            Ok(serenity::Channel::Guild(channel)) => channel,
            Ok(_) => continue,
            Err(err) => {
                tracing::warn!(fil = %category.name, %err, "fil introuvable au démarrage");
                continue;
            }
        };
        if let Some(meta) = &channel.thread_metadata
            && should_revive(meta.archived, meta.locked)
        {
            revive(ctx, channel.id, &category.name).await?;
        }
    }
    Ok(())
}

async fn revive(
    ctx: &serenity::Context,
    thread: serenity::ChannelId,
    name: &str,
) -> anyhow::Result<()> {
    thread
        .edit_thread(ctx, serenity::EditThread::new().archived(false))
        .await?;
    tracing::info!(fil = %name, "fil désarchivé pour que les réactions restent possibles");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_archived_thread_is_reopened() {
        assert!(should_revive(true, false));
    }

    #[test]
    fn a_locked_thread_is_left_closed() {
        // Verrouillé = fermé exprès par un modérateur, pas par l'inactivité.
        assert!(!should_revive(true, true));
    }

    #[test]
    fn an_open_thread_needs_nothing() {
        assert!(!should_revive(false, false));
        assert!(!should_revive(false, true));
    }
}

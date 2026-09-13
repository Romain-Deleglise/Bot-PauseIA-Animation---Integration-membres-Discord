//! Écriture des fils du forum.
//!
//! # Publication ciblée plutôt que réécriture systématique
//!
//! L'ancien bot purgeait le salon entier et republiait toutes les cartes à
//! chaque création, modification ou suppression. Outre le coût en appels API,
//! cela **effaçait les réactions des membres** : le message ayant disparu, plus
//! personne ne pouvait retirer son rôle en retirant sa réaction.
//!
//! Comme `posts.message_id` mémorise la carte de chaque message, on modifie
//! désormais celle qui est concernée, sur place. C'est ce qui permet de tenir
//! l'exigence du CDC : « éditer les descriptions des messages sans perdre le
//! comportement ni supprimer des rôles aux utilisateurs ».
//!
//! La réécriture complète reste nécessaire pour réordonner, l'ordre des
//! messages Discord ne pouvant être changé qu'en les republiant.

use crate::db;
use crate::db::categories::Category;
use crate::db::posts::Post;
use crate::discord::embed;
use crate::ids;
use crate::state::Data;
use anyhow::Context as _;
use poise::serenity_prelude as serenity;

/// Origine des identifiants Discord : 1er janvier 2015, en millisecondes.
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;
/// Au-delà, l'API refuse la suppression groupée et impose l'unitaire.
const BULK_DELETE_MAX_AGE_MS: u64 = 14 * 24 * 60 * 60 * 1000;
/// Garde-fou : une suppression qui échouerait en silence ferait boucler la purge.
const MAX_PURGE_ROUNDS: usize = 40;

/// Instant de création encodé dans un identifiant Discord.
fn created_at_ms(id: u64) -> u64 {
    (id >> 22) + DISCORD_EPOCH_MS
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

fn eligible_for_bulk_delete(message_id: u64, now: u64) -> bool {
    now.saturating_sub(created_at_ms(message_id)) < BULK_DELETE_MAX_AGE_MS
}

/// Publie la carte d'un message en fin de fil et y pose la réaction.
///
/// La réaction n'est posée que si le message accorde un rôle : le CDC demande
/// qu'un message d'information n'affiche « pas d'emoji visible ».
pub async fn publish(
    http: &serenity::Http,
    data: &Data,
    category: &Category,
    post: &Post,
) -> anyhow::Result<serenity::MessageId> {
    let channel = ids::channel(category.channel_id);
    let message = channel
        .send_message(
            http,
            serenity::CreateMessage::new().embed(embed::card(category, post)),
        )
        .await
        .with_context(|| format!("publication de la carte `{}`", post.title))?;

    if post.role_id.is_some() {
        channel
            .create_reaction(
                http,
                message.id,
                serenity::ReactionType::from(&data.config.reaction_emoji),
            )
            .await
            .with_context(|| format!("pose de la réaction sur la carte `{}`", post.title))?;
    }

    db::posts::set_message_id(&data.db, post.id, Some(ids::to_db(message.id.get()))).await?;
    Ok(message.id)
}

/// Met à jour la carte d'un message sans toucher aux réactions déjà posées.
///
/// Si la carte n'existe plus — message supprimé à la main — elle est republiée.
pub async fn refresh(
    http: &serenity::Http,
    data: &Data,
    category: &Category,
    post: &Post,
) -> anyhow::Result<()> {
    let Some(message_id) = post.message_id else {
        publish(http, data, category, post).await?;
        return Ok(());
    };

    let edited = ids::channel(category.channel_id)
        .edit_message(
            http,
            ids::message(message_id),
            serenity::EditMessage::new().embed(embed::card(category, post)),
        )
        .await;

    if let Err(err) = edited {
        tracing::warn!(
            message = %post.title,
            %err,
            "carte introuvable, republication à la suite du fil"
        );
        db::posts::set_message_id(&data.db, post.id, None).await?;
        publish(
            http,
            data,
            category,
            &Post {
                message_id: None,
                ..post.clone()
            },
        )
        .await?;
    }
    Ok(())
}

/// Supprime la carte d'un message et oublie sa référence.
pub async fn remove(
    http: &serenity::Http,
    data: &Data,
    category: &Category,
    post: &Post,
) -> anyhow::Result<()> {
    if let Some(message_id) = post.message_id {
        // La carte peut déjà avoir disparu : ce n'est pas une erreur.
        if let Err(err) = ids::channel(category.channel_id)
            .delete_message(http, ids::message(message_id))
            .await
        {
            tracing::warn!(%err, "suppression de la carte impossible, elle a probablement déjà disparu");
        }
        db::posts::set_message_id(&data.db, post.id, None).await?;
    }
    Ok(())
}

/// Réécrit intégralement le fil : en-têtes puis toutes les cartes, dans l'ordre
/// voulu.
///
/// Seule opération capable de corriger l'ordre d'affichage. Elle coûte une
/// purge plus un envoi par message, et fait perdre les réactions existantes —
/// d'où son usage réservé à la commande de republication.
pub async fn rewrite(
    http: &serenity::Http,
    data: &Data,
    category: &Category,
) -> anyhow::Result<usize> {
    let channel = ids::channel(category.channel_id);
    tracing::info!(fil = %category.name, "réécriture du fil");

    purge_channel(http, channel).await?;
    db::posts::clear_message_ids(&data.db, category.id).await?;

    // Les en-têtes valent pour un salon comme pour un fil. L'ancien bot les
    // envoyait sous une condition qui excluait les fils par accident.
    if let Some(url) = category
        .header_image_url
        .as_deref()
        .filter(|url| !url.is_empty())
    {
        channel
            .send_message(http, serenity::CreateMessage::new().content(url))
            .await
            .context("envoi de l'illustration du fil")?;
    }
    if let Some(text) = category.header_text.as_deref().filter(|t| !t.is_empty()) {
        channel
            .send_message(
                http,
                serenity::CreateMessage::new().content(embed::format_description(text)),
            )
            .await
            .context("envoi du texte d'introduction")?;
    }

    let posts = db::posts::by_category(&data.db, category.id).await?;
    for post in &posts {
        publish(http, data, category, post).await?;
    }

    Ok(posts.len())
}

/// Vide un salon de tous ses messages, en préservant le message d'amorce d'un fil.
///
/// Dans un fil, le message d'amorce porte le même identifiant que le fil
/// lui-même et ne peut pas être supprimé sans supprimer le fil.
pub async fn purge_channel(
    http: &serenity::Http,
    channel: serenity::ChannelId,
) -> anyhow::Result<usize> {
    let mut deleted = 0;

    for round in 0..MAX_PURGE_ROUNDS {
        let messages = channel
            .messages(http, serenity::GetMessages::new().limit(100))
            .await
            .context("lecture des messages du fil")?;

        let ids: Vec<serenity::MessageId> = messages
            .iter()
            .map(|message| message.id)
            .filter(|id| id.get() != channel.get())
            .collect();

        if ids.is_empty() {
            return Ok(deleted);
        }

        let now = now_ms();
        let (recent, old): (Vec<_>, Vec<_>) = ids
            .into_iter()
            .partition(|id| eligible_for_bulk_delete(id.get(), now));

        if !recent.is_empty() {
            // `delete_messages` traite le cas d'un message unique en interne.
            channel
                .delete_messages(http, &recent)
                .await
                .context("suppression groupée des messages")?;
            deleted += recent.len();
        }
        // Passé deux semaines, Discord n'accepte plus que l'unitaire.
        for id in &old {
            channel
                .delete_message(http, *id)
                .await
                .context("suppression d'un ancien message")?;
            deleted += 1;
        }

        if round + 1 == MAX_PURGE_ROUNDS {
            tracing::warn!(
                salon = channel.get(),
                deleted,
                "purge interrompue par le garde-fou, il reste des messages"
            );
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identifiant Discord correspondant à un instant donné.
    fn snowflake_at(unix_ms: u64) -> u64 {
        (unix_ms - DISCORD_EPOCH_MS) << 22
    }

    #[test]
    fn snowflake_timestamp_roundtrip() {
        let moment = DISCORD_EPOCH_MS + 1_000_000;
        assert_eq!(created_at_ms(snowflake_at(moment)), moment);
    }

    #[test]
    fn recent_messages_go_through_bulk_delete() {
        let now = DISCORD_EPOCH_MS + 30 * 24 * 60 * 60 * 1000;
        let yesterday = snowflake_at(now - 24 * 60 * 60 * 1000);

        assert!(eligible_for_bulk_delete(yesterday, now));
    }

    #[test]
    fn messages_older_than_two_weeks_do_not() {
        let now = DISCORD_EPOCH_MS + 30 * 24 * 60 * 60 * 1000;
        let three_weeks_ago = snowflake_at(now - 21 * 24 * 60 * 60 * 1000);

        assert!(!eligible_for_bulk_delete(three_weeks_ago, now));
    }

    #[test]
    fn the_two_week_boundary_is_exclusive() {
        let now = DISCORD_EPOCH_MS + 30 * 24 * 60 * 60 * 1000;
        let exactly = snowflake_at(now - BULK_DELETE_MAX_AGE_MS);

        // Pile à la limite, l'API peut refuser : on bascule en unitaire.
        assert!(!eligible_for_bulk_delete(exactly, now));
        assert!(eligible_for_bulk_delete(
            snowflake_at(now - BULK_DELETE_MAX_AGE_MS + 1000),
            now
        ));
    }

    #[test]
    fn a_clock_behind_the_message_does_not_panic() {
        // `saturating_sub` protège d'une horloge locale en retard sur Discord.
        let now = DISCORD_EPOCH_MS;
        assert!(eligible_for_bulk_delete(snowflake_at(now + 60_000), now));
    }
}

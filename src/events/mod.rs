//! Aiguillage des événements Gateway.
//!
//! L'ancien bot déclarait deux cogs à l'écoute de `on_raw_reaction_add`, chacun
//! refaisant ses propres tests. Ici, un point d'entrée unique décide une fois
//! pour toutes de ce qu'une réaction signifie.

pub mod reactions;
pub mod threads;

use crate::state::{Data, Error};
use poise::serenity_prelude as serenity;

pub async fn dispatch(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            tracing::info!(bot = %data_about_bot.user.name, "connecté à Discord");
            // Un échec ici ne doit pas empêcher le bot de servir : le prochain
            // archivage déclenchera de toute façon `ThreadUpdate`.
            if let Err(err) = threads::revive_all(ctx, data).await {
                tracing::warn!(%err, "réouverture des fils archivés au démarrage impossible");
            }
            Ok(())
        }
        serenity::FullEvent::ThreadUpdate { new, .. } => threads::on_update(ctx, data, new).await,
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            reactions::on_add(ctx, data, add_reaction).await
        }
        serenity::FullEvent::ReactionRemove { removed_reaction } => {
            reactions::on_remove(ctx, data, removed_reaction).await
        }
        _ => Ok(()),
    }
}

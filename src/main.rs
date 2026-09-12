//! Bot Rôles — publie et pilote le forum de rôles auto-assignables d'un serveur Discord.
//!
//! Voir CLAUDE.md pour l'architecture et README.md pour l'exploitation.

mod commands;
mod config;
mod db;
mod discord;
mod emoji;
mod events;
mod ids;
mod mentions;
mod rules;
mod state;

use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use state::Data;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Confort de développement : en production la configuration vient de
    // l'environnement du conteneur, et l'absence de fichier n'est pas une erreur.
    let _ = dotenvy::dotenv();

    let config = config::Config::from_env()?;
    init_tracing(&config.log_level);

    let pool = db::connect(&config.database_path)
        .await
        .context("initialisation de la base")?;
    tracing::info!(base = %config.database_path, "base prête");

    let guild_id = config.guild_id;
    let token = config.discord_token.clone();
    let enable_import = config.enable_import;
    let data = Data::new(pool, config).await?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(enable_import),
            // Posé globalement, il couvre chaque sous-commande, y compris
            // celles ajoutées plus tard.
            command_check: Some(|ctx| Box::pin(commands::is_manager(ctx))),
            on_error: |error| Box::pin(report_error(error)),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(events::dispatch(ctx, event, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Enregistrement sur la guilde : la propagation est immédiate,
                // là où des commandes globales mettent jusqu'à une heure.
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id)
                    .await?;
                tracing::info!(guilde = %guild_id, "commandes enregistrées");
                Ok(data)
            })
        })
        .build();

    // `GUILD_MEMBERS` est privilégié et doit être coché dans le portail
    // développeur. Sans lui, le cache ne reçoit pas les changements de rôles
    // des membres, et le retrait d'une réaction déciderait du rôle parent sur
    // des rôles périmés.
    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .context("création du client Discord")?;

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        wait_for_shutdown().await;
        tracing::info!("arrêt demandé, fermeture des connexions");
        shard_manager.shutdown_all().await;
    });

    client.start().await.context("boucle principale du bot")?;
    tracing::info!("arrêté proprement");
    Ok(())
}

/// Journalisation sur la sortie standard, au format attendu par `docker logs`.
///
/// `RUST_LOG` prend le pas sur `LOG_LEVEL` quand il est présent, pour pouvoir
/// affiner un diagnostic sans toucher à la configuration du conteneur.
fn init_tracing(level: &str) {
    let fallback = format!(
        "bot_roles={},serenity=warn,poise=warn,sqlx=warn",
        level.to_lowercase()
    );
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&fallback))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("bot_roles=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}

/// Attend SIGTERM (arrêt d'un conteneur) ou Ctrl+C (exécution locale).
async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(err) => {
                tracing::warn!(%err, "SIGTERM non écoutable, seul Ctrl+C arrêtera le bot");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Journalise l'incident et prévient l'appelant, plutôt que de le laisser
/// devant une interaction qui ne répond jamais.
async fn report_error(error: poise::FrameworkError<'_, Data, state::Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(commande = %ctx.command().qualified_name, ?error, "commande en échec");
            let _ = ctx
                .send(
                    poise::CreateReply::default()
                        .content(format!("La commande a échoué : {error}"))
                        .ephemeral(true),
                )
                .await;
        }
        poise::FrameworkError::CommandCheckFailed { ctx, .. } => {
            // `is_manager` a déjà répondu à l'appelant.
            tracing::debug!(commande = %ctx.command().qualified_name, "commande refusée");
        }
        other => {
            if let Err(err) = poise::builtins::on_error(other).await {
                tracing::error!(%err, "erreur du framework");
            }
        }
    }
}

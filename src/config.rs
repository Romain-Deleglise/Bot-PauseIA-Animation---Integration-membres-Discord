//! Configuration, lue depuis l'environnement au démarrage.
//!
//! Tout est validé d'un bloc : plutôt que d'échouer à la première variable
//! manquante, on les collecte toutes et on les rapporte ensemble. Corriger un
//! `.env` incomplet ne demande donc qu'un seul cycle.

use crate::emoji::Emoji;
use poise::serenity_prelude as serenity;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub guild_id: serenity::GuildId,
    /// Rôles autorisés à utiliser les commandes du bot.
    pub manage_role_ids: Vec<serenity::RoleId>,
    /// Emoji de la réaction posée sous chaque carte accordant un rôle.
    pub reaction_emoji: Emoji,
    pub database_path: String,
    pub log_level: String,
    /// Propose `/forum importer`. L'import ne sert en principe qu'une fois : on
    /// le coupe ensuite pour qu'il ne traîne pas dans la liste des commandes.
    pub enable_import: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut errors: Vec<String> = Vec::new();

        let discord_token = required("DISCORD_TOKEN", &mut errors);
        let guild_id = required("DISCORD_GUILD_ID", &mut errors)
            .and_then(|raw| parse_id("DISCORD_GUILD_ID", &raw, &mut errors))
            .map(serenity::GuildId::new);

        let manage_role_ids = required("MANAGE_ROLE_IDS", &mut errors).and_then(|raw| {
            let ids: Vec<_> = raw
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(|part| parse_id("MANAGE_ROLE_IDS", part, &mut errors))
                .collect();
            if ids.is_empty() {
                errors
                    .push("MANAGE_ROLE_IDS est vide : personne ne pourrait piloter le bot".into());
                return None;
            }
            ids.into_iter()
                .collect::<Option<Vec<_>>>()
                .map(|ids| ids.into_iter().map(serenity::RoleId::new).collect())
        });

        let reaction_emoji =
            required("REACTION_EMOJI", &mut errors).and_then(|raw| match Emoji::parse(&raw) {
                Ok(emoji) => Some(emoji),
                Err(err) => {
                    errors.push(format!("REACTION_EMOJI : {err}"));
                    None
                }
            });

        let enable_import = match optional("ENABLE_IMPORT")
            .map(|v| v.to_lowercase())
            .as_deref()
        {
            None | Some("true" | "1" | "oui" | "yes") => Some(true),
            Some("false" | "0" | "non" | "no") => Some(false),
            Some(other) => {
                errors.push(format!(
                    "ENABLE_IMPORT : `{other}` n'est pas un booléen (attendu true ou false)"
                ));
                None
            }
        };

        if !errors.is_empty() {
            anyhow::bail!(
                "configuration invalide :\n  - {}\n\nVoir .env.example pour le détail de chaque variable.",
                errors.join("\n  - ")
            );
        }

        Ok(Config {
            discord_token: discord_token.expect("validé ci-dessus"),
            guild_id: guild_id.expect("validé ci-dessus"),
            manage_role_ids: manage_role_ids.expect("validé ci-dessus"),
            reaction_emoji: reaction_emoji.expect("validé ci-dessus"),
            database_path: optional("DATABASE_PATH").unwrap_or_else(|| "/data/bot.db".into()),
            log_level: optional("LOG_LEVEL").unwrap_or_else(|| "info".into()),
            enable_import: enable_import.expect("validé ci-dessus"),
        })
    }

    #[cfg(test)]
    pub fn fixture() -> Self {
        Config {
            discord_token: "jeton-de-test".into(),
            guild_id: serenity::GuildId::new(1),
            manage_role_ids: vec![serenity::RoleId::new(2)],
            reaction_emoji: Emoji::parse("🙋").expect("emoji valide"),
            database_path: ":memory:".into(),
            log_level: "info".into(),
            enable_import: true,
        }
    }
}

fn optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn required(key: &str, errors: &mut Vec<String>) -> Option<String> {
    match optional(key) {
        Some(value) => Some(value),
        None => {
            errors.push(format!("{key} est absent ou vide"));
            None
        }
    }
}

fn parse_id(key: &str, raw: &str, errors: &mut Vec<String>) -> Option<u64> {
    match raw.parse::<u64>() {
        Ok(0) => {
            errors.push(format!(
                "{key} : `0` n'est pas un identifiant Discord valide"
            ));
            None
        }
        Ok(id) => Some(id),
        Err(_) => {
            errors.push(format!(
                "{key} : `{raw}` n'est pas un identifiant Discord (attendu un nombre)"
            ));
            None
        }
    }
}

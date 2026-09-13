//! Représentation d'un emoji, indépendante de la forme dans laquelle Discord nous
//! le livre.
//!
//! Un emoji arrive sous trois formes selon le contexte : saisi par un humain dans
//! une commande (`🌊` ou `<:meduse:872735843771633734>`), livré dans un événement
//! Gateway (`ReactionType`), ou stocké en base (TEXT). Ce module fait le pont
//! entre les trois.
//!
//! La comparaison mérite une attention particulière : dans les événements de
//! réaction, Discord n'envoie pas toujours le nom d'un emoji custom. Comparer
//! deux `ReactionType` avec `==` échoue donc de façon intermittente. `matches`
//! ne compare que l'identité réelle : l'ID pour un emoji custom, la chaîne pour
//! un emoji unicode.

use poise::serenity_prelude as serenity;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emoji {
    Unicode(String),
    Custom {
        id: serenity::EmojiId,
        name: String,
        animated: bool,
    },
}

impl Emoji {
    /// Parse la forme saisie par un humain ou relue depuis la base.
    ///
    /// Accepte `<:nom:123>`, `<a:nom:123>` et n'importe quel emoji unicode.
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            anyhow::bail!("emoji vide");
        }

        let Some(inner) = trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')) else {
            return Ok(Emoji::Unicode(trimmed.to_owned()));
        };

        let (animated, rest) = match inner.strip_prefix('a') {
            Some(rest) => (true, rest),
            None => (false, inner),
        };
        let mut parts = rest.split(':');
        // La forme est `:nom:id`, donc le premier segment est vide.
        let (Some(""), Some(name), Some(id), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            anyhow::bail!("emoji custom malformé : `{raw}` (attendu `<:nom:id>`)");
        };
        let id: u64 = id
            .parse()
            .map_err(|_| anyhow::anyhow!("identifiant d'emoji invalide dans `{raw}`"))?;

        Ok(Emoji::Custom {
            id: serenity::EmojiId::new(id),
            name: name.to_owned(),
            animated,
        })
    }

    /// Reconstruit un emoji à partir d'un événement Gateway.
    ///
    /// Rend `None` pour les variantes que Discord pourrait ajouter et que nous
    /// ne savons pas nommer : mieux vaut ignorer la réaction que se tromper de rôle.
    pub fn from_reaction(reaction: &serenity::ReactionType) -> Option<Self> {
        match reaction {
            serenity::ReactionType::Unicode(raw) => Some(Emoji::Unicode(raw.to_string())),
            serenity::ReactionType::Custom { id, name, animated } => Some(Emoji::Custom {
                id: *id,
                name: name.clone().unwrap_or_default(),
                animated: *animated,
            }),
            _ => None,
        }
    }

    /// Clé stable, utilisée comme identité en base et dans les caches.
    ///
    /// Elle ignore volontairement le nom d'un emoji custom : Discord ne le
    /// transmet pas systématiquement dans les événements de réaction, et un nom
    /// change au gré des renommages alors que l'identifiant, lui, ne bouge pas.
    pub fn key(&self) -> String {
        match self {
            Emoji::Unicode(raw) => raw.clone(),
            Emoji::Custom { id, .. } => format!("custom:{id}"),
        }
    }

    /// Vrai si l'emoji d'un événement Gateway désigne le même emoji que celui-ci.
    pub fn matches(&self, other: &serenity::ReactionType) -> bool {
        match (self, other) {
            (Emoji::Unicode(ours), serenity::ReactionType::Unicode(theirs)) => {
                ours.as_str() == theirs.as_str()
            }
            (Emoji::Custom { id: ours, .. }, serenity::ReactionType::Custom { id: theirs, .. }) => {
                ours == theirs
            }
            _ => false,
        }
    }
}

impl From<&Emoji> for serenity::ReactionType {
    fn from(emoji: &Emoji) -> Self {
        match emoji {
            Emoji::Unicode(raw) => serenity::ReactionType::Unicode(raw.clone()),
            Emoji::Custom { id, name, animated } => serenity::ReactionType::Custom {
                animated: *animated,
                id: *id,
                name: Some(name.clone()),
            },
        }
    }
}

/// Forme canonique : ce qui est stocké en base et affiché dans les réponses.
impl fmt::Display for Emoji {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Emoji::Unicode(raw) => f.write_str(raw),
            Emoji::Custom { id, name, animated } => {
                let prefix = if *animated { "a" } else { "" };
                write!(f, "<{prefix}:{name}:{id}>")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unicode() {
        assert_eq!(Emoji::parse("🌊").unwrap(), Emoji::Unicode("🌊".into()));
        // Les espaces autour d'une saisie humaine ne doivent pas polluer la clé.
        assert_eq!(Emoji::parse("  🌊 ").unwrap(), Emoji::Unicode("🌊".into()));
    }

    #[test]
    fn parse_custom() {
        let parsed = Emoji::parse("<:meduse:872735843771633734>").unwrap();
        assert_eq!(
            parsed,
            Emoji::Custom {
                id: serenity::EmojiId::new(872735843771633734),
                name: "meduse".into(),
                animated: false,
            }
        );
    }

    #[test]
    fn parse_animated_custom() {
        let parsed = Emoji::parse("<a:vague:1234>").unwrap();
        assert!(matches!(parsed, Emoji::Custom { animated: true, .. }));
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(Emoji::parse("").is_err());
        assert!(Emoji::parse("<:meduse>").is_err());
        assert!(Emoji::parse("<:meduse:pas_un_nombre>").is_err());
        assert!(Emoji::parse("<:a:b:c:1>").is_err());
    }

    #[test]
    fn roundtrip_through_storage() {
        for raw in ["🌊", "<:meduse:872735843771633734>", "<a:vague:1234>"] {
            let parsed = Emoji::parse(raw).unwrap();
            assert_eq!(Emoji::parse(&parsed.to_string()).unwrap(), parsed);
        }
    }

    #[test]
    fn matches_custom_without_name() {
        // Discord omet régulièrement le nom dans les payloads de réaction :
        // comparer les `ReactionType` avec `==` échouerait ici.
        let ours = Emoji::parse("<:meduse:872735843771633734>").unwrap();
        let from_gateway = serenity::ReactionType::Custom {
            animated: false,
            id: serenity::EmojiId::new(872735843771633734),
            name: None,
        };
        assert!(ours.matches(&from_gateway));
    }

    #[test]
    fn key_ignores_the_custom_emoji_name() {
        // Deux vues du même emoji, l'une nommée, l'autre non : elles doivent
        // produire la même clé, sans quoi une liaison enregistrée par commande
        // ne serait jamais retrouvée depuis un événement.
        let named = Emoji::parse("<:meduse:872735843771633734>").unwrap();
        let from_gateway = Emoji::from_reaction(&serenity::ReactionType::Custom {
            animated: false,
            id: serenity::EmojiId::new(872735843771633734),
            name: None,
        })
        .unwrap();

        assert_eq!(named.key(), from_gateway.key());
        assert_ne!(named.to_string(), from_gateway.to_string());
    }

    #[test]
    fn key_separates_unicode_from_custom() {
        assert_eq!(Emoji::parse("🌊").unwrap().key(), "🌊");
        assert_eq!(Emoji::parse("<:x:42>").unwrap().key(), "custom:42");
        assert_ne!(
            Emoji::parse("🌊").unwrap().key(),
            Emoji::parse("🔧").unwrap().key()
        );
    }

    #[test]
    fn from_reaction_roundtrips_unicode() {
        let reaction = serenity::ReactionType::Unicode("🌊".into());
        assert_eq!(
            Emoji::from_reaction(&reaction).unwrap(),
            Emoji::Unicode("🌊".into())
        );
    }

    #[test]
    fn matches_rejects_other_emoji() {
        let ours = Emoji::parse("<:meduse:1>").unwrap();
        assert!(!ours.matches(&serenity::ReactionType::Custom {
            animated: false,
            id: serenity::EmojiId::new(2),
            name: None,
        }));
        assert!(!ours.matches(&serenity::ReactionType::Unicode("🌊".into())));
    }
}

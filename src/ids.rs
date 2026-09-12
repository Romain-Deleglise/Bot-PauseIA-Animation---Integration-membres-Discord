//! Conversions entre les identifiants Discord (`u64`) et le type entier de
//! SQLite (`i64`).
//!
//! Un snowflake Discord tient dans 63 bits jusque vers l'an 2154 : la
//! conversion est exacte dans les deux sens, et le `as` ne perd rien.

use poise::serenity_prelude as serenity;

pub const fn to_db(id: u64) -> i64 {
    id as i64
}

pub const fn role(id: i64) -> serenity::RoleId {
    serenity::RoleId::new(id as u64)
}

pub const fn channel(id: i64) -> serenity::ChannelId {
    serenity::ChannelId::new(id as u64)
}

pub const fn message(id: i64) -> serenity::MessageId {
    serenity::MessageId::new(id as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_snowflakes() {
        // Un identifiant Discord réel, et la plus grande valeur représentable.
        for raw in [872735843771633734_u64, i64::MAX as u64] {
            assert_eq!(role(to_db(raw)).get(), raw);
            assert_eq!(message(to_db(raw)).get(), raw);
        }
    }
}

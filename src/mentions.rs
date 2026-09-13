//! Transforme les `@pseudo` écrits en texte en vraies mentions Discord.
//!
//! Le CDC et le fichier de contenu notent les référents `@pseudo`. Discord n'en
//! fait un lien cliquable que sous la forme `<@identifiant>`, que personne ne
//! saisit à la main : la conversion se fait donc à l'écriture, d'après la liste
//! des membres du serveur.

use std::collections::HashMap;

/// Mentions collectives de Discord : elles ne désignent aucun membre, inutile
/// de les signaler comme introuvables.
const COLLECTIVE: [&str; 2] = ["everyone", "here"];

/// Remplace chaque `@pseudo` connu par une mention, et renvoie les pseudos
/// introuvables, laissés tels quels dans le texte.
///
/// `members` associe un pseudo en minuscules à l'identifiant du membre.
pub fn link_handles(text: &str, members: &HashMap<String, u64>) -> (String, Vec<String>) {
    let mut linked = String::with_capacity(text.len());
    let mut unknown: Vec<String> = Vec::new();
    let mut rest = text;
    let mut previous: Option<char> = None;

    while let Some(at) = rest.find('@') {
        let before = &rest[..at];
        linked.push_str(before);
        let glued_to = before.chars().next_back().or(previous);
        let after = &rest[at + 1..];
        let handle = &after[..handle_len(after)];

        // Collé à un mot, c'est une adresse mail ; derrière `<`, une mention
        // déjà écrite. Discord refuse les pseudos d'un seul caractère.
        let attached =
            glued_to.is_some_and(|c| c.is_alphanumeric() || matches!(c, '<' | '_' | '.'));
        if attached || handle.chars().count() < 2 {
            linked.push('@');
            previous = Some('@');
            rest = after;
            continue;
        }

        let key = handle.to_lowercase();
        match members.get(&key) {
            Some(id) => linked.push_str(&format!("<@{id}>")),
            None => {
                linked.push('@');
                linked.push_str(handle);
                if !COLLECTIVE.contains(&key.as_str())
                    && !unknown.iter().any(|known| known.to_lowercase() == key)
                {
                    unknown.push(handle.to_owned());
                }
            }
        }
        previous = handle.chars().next_back();
        rest = &after[handle.len()..];
    }

    linked.push_str(rest);
    (linked, unknown)
}

/// Longueur en octets du pseudo en tête de `text` : lettres, chiffres, `_` et
/// `.`, sans le point qui terminerait la phrase.
fn handle_len(text: &str) -> usize {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.')))
        .unwrap_or(text.len());
    text[..end].trim_end_matches('.').len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members() -> HashMap<String, u64> {
        HashMap::from([("doweee1".to_owned(), 42), ("lady_mircalla".to_owned(), 7)])
    }

    #[test]
    fn a_known_handle_becomes_a_mention() {
        let (text, unknown) = link_handles("Référent : @doweee1", &members());
        assert_eq!(text, "Référent : <@42>");
        assert!(unknown.is_empty());
    }

    #[test]
    fn handles_are_matched_whatever_their_case() {
        assert_eq!(link_handles("@Doweee1", &members()).0, "<@42>");
    }

    #[test]
    fn several_handles_in_a_row_are_all_linked() {
        let (text, _) = link_handles("Référentes : @doweee1 @lady_mircalla", &members());
        assert_eq!(text, "Référentes : <@42> <@7>");
    }

    #[test]
    fn the_dot_ending_a_sentence_is_not_part_of_the_handle() {
        assert_eq!(
            link_handles("Contactez @doweee1.", &members()).0,
            "Contactez <@42>."
        );
    }

    #[test]
    fn an_unknown_handle_stays_as_text_and_is_reported_once() {
        let (text, unknown) = link_handles("@absent puis @Absent", &members());
        assert_eq!(text, "@absent puis @Absent");
        assert_eq!(unknown, vec!["absent"]);
    }

    #[test]
    fn emails_and_existing_mentions_are_left_alone() {
        let source = "écrire à contact@doweee1.fr ou à <@123>";
        let (text, unknown) = link_handles(source, &members());
        assert_eq!(text, source);
        assert!(unknown.is_empty());
    }

    #[test]
    fn collective_mentions_are_not_reported() {
        let (text, unknown) = link_handles("@everyone et @here", &members());
        assert_eq!(text, "@everyone et @here");
        assert!(unknown.is_empty());
    }
}

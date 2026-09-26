//! Les règles métier qui ne dépendent ni de Discord ni de la base.
//!
//! Isolées ici pour être vérifiables sans serveur ni pool : ce sont elles qui
//! décident si un membre garde ou perd un rôle, et une erreur y est invisible
//! tant qu'un membre ne s'en plaint pas.

use std::collections::HashSet;

/// Le rôle parent reste-t-il justifié après le retrait d'un rôle de message ?
///
/// Un membre inscrit à Paris et à Lyon porte `@groupe-local` pour deux raisons.
/// Quitter Paris ne doit pas lui faire perdre l'accès aux salons communs, mais
/// quitter le dernier groupe, si.
///
/// - `granting_roles` : tous les rôles de message qui accordent ce rôle parent ;
/// - `member_roles` : les rôles du membre **avant** le retrait ;
/// - `removed_role` : le rôle de message qu'on vient de lui reprendre.
pub fn parent_still_justified(
    granting_roles: &[i64],
    member_roles: &HashSet<i64>,
    removed_role: i64,
) -> bool {
    granting_roles
        .iter()
        .any(|role| *role != removed_role && member_roles.contains(role))
}

/// Confirmation d'entrée par défaut, quand un fil n'en définit pas.
pub const JOINED_DEFAULT: &str = "🙋 Te voilà dans **{carte}** !\n\nTu viens de recevoir {rôles}.\n\nTu changes d'avis ? Retire ta réaction sur la carte, le rôle est repris aussitôt.";

/// Confirmation de sortie par défaut.
pub const LEFT_DEFAULT: &str = "Tu as quitté **{carte}**, et {rôles} t'a été repris.\n\nLa porte reste ouverte : lève la main sur la carte quand tu veux revenir.";

/// Annonce d'une main levée, publiée dans le salon du projet.
///
/// « Souhaite rejoindre » plutôt que « vient de rejoindre » : côté salon, c'est
/// une candidature à accueillir, pas un fait accompli à enregistrer.
pub const ANNOUNCE_JOIN_DEFAULT: &str = "🙋 {membre} souhaite rejoindre **{carte}**.";

/// Annonce d'une main baissée.
pub const ANNOUNCE_LEAVE_DEFAULT: &str = "↩️ {membre} a quitté **{carte}**.";

/// Marqueurs reconnus dans une confirmation. `roles` sans accent est accepté :
/// la saisie se fait sur un téléphone aussi souvent que sur un clavier.
pub const MARKERS: [&str; 3] = ["{carte}", "{rôles}", "{roles}"];

/// Marqueurs reconnus dans une annonce. Pas de rôle ici : le salon parle d'une
/// personne et d'une carte, le détail des rôles est l'affaire du message privé.
pub const ANNOUNCE_MARKERS: [&str; 2] = ["{carte}", "{membre}"];

/// Remplit les marqueurs d'une confirmation.
pub fn render_confirmation(template: &str, title: &str, roles: &str) -> String {
    template
        .replace("{carte}", title)
        .replace("{rôles}", roles)
        .replace("{roles}", roles)
}

/// Remplit les marqueurs d'une annonce.
pub fn render_announce(template: &str, title: &str, member: &str) -> String {
    template
        .replace("{carte}", title)
        .replace("{membre}", member)
}

/// Liste les marqueurs inconnus d'un texte, pour les refuser à la saisie.
///
/// Une faute de frappe dans `{carte}` partirait sinon telle quelle à chaque
/// membre, et ne se verrait qu'une fois le mal fait.
pub fn unknown_markers(template: &str, allowed: &[&str]) -> Vec<String> {
    let mut rest = template;
    let mut found = Vec::new();
    while let Some(start) = rest.find('{') {
        let after = &rest[start..];
        let Some(end) = after.find('}') else { break };
        let marker = &after[..=end];
        if !allowed.contains(&marker) && !found.iter().any(|seen| seen == marker) {
            found.push(marker.to_owned());
        }
        rest = &after[end + 1..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les onze groupes locaux accordent tous `@groupe-local`.
    const GROUPES_LOCAUX: [i64; 3] = [10, 11, 12];

    fn holding(roles: &[i64]) -> HashSet<i64> {
        roles.iter().copied().collect()
    }

    #[test]
    fn another_group_still_justifies_it() {
        // Membre à Paris (10) et Lyon (11), il quitte Paris.
        assert!(parent_still_justified(
            &GROUPES_LOCAUX,
            &holding(&[10, 11]),
            10
        ));
    }

    #[test]
    fn leaving_the_last_group_ends_it() {
        assert!(!parent_still_justified(
            &GROUPES_LOCAUX,
            &holding(&[10]),
            10
        ));
    }

    #[test]
    fn unrelated_roles_do_not_count() {
        // Le membre porte d'autres rôles, mais aucun groupe local.
        assert!(!parent_still_justified(
            &GROUPES_LOCAUX,
            &holding(&[10, 500, 501]),
            10
        ));
    }

    #[test]
    fn a_role_granted_by_nothing_is_never_justified() {
        assert!(!parent_still_justified(&[], &holding(&[10]), 10));
    }

    #[test]
    fn the_removed_role_never_justifies_itself() {
        // Le membre porte encore le rôle au moment du calcul : c'est justement
        // pour cela qu'on l'exclut explicitement.
        assert!(!parent_still_justified(&[10], &holding(&[10]), 10));
    }

    #[test]
    fn a_group_the_member_never_joined_does_not_help() {
        // Lille (12) existe et accorde le rôle, mais ce membre n'y est pas.
        assert!(!parent_still_justified(
            &GROUPES_LOCAUX,
            &holding(&[10]),
            10
        ));
    }

    #[test]
    fn a_confirmation_keeps_text_without_markers() {
        assert_eq!(render_confirmation("Merci !", "Veille", "@x"), "Merci !");
    }

    #[test]
    fn both_spellings_of_the_roles_marker_are_filled() {
        assert_eq!(
            render_confirmation("{carte} : {rôles} et {roles}", "Veille", "@x"),
            "Veille : @x et @x"
        );
    }

    #[test]
    fn a_mistyped_marker_is_reported() {
        assert_eq!(
            unknown_markers("Bravo {crate} !", &MARKERS),
            vec!["{crate}"]
        );
        assert!(unknown_markers(JOINED_DEFAULT, &MARKERS).is_empty());
        assert!(unknown_markers(LEFT_DEFAULT, &MARKERS).is_empty());
        assert!(unknown_markers(ANNOUNCE_JOIN_DEFAULT, &ANNOUNCE_MARKERS).is_empty());
        assert!(unknown_markers(ANNOUNCE_LEAVE_DEFAULT, &ANNOUNCE_MARKERS).is_empty());
    }

    #[test]
    fn a_marker_of_another_message_is_refused() {
        // `{rôles}` n'a pas de sens dans une annonce : le salon n'en parle pas.
        assert_eq!(
            unknown_markers("Bienvenue {rôles}", &ANNOUNCE_MARKERS),
            vec!["{rôles}"]
        );
    }

    #[test]
    fn an_unclosed_brace_is_not_a_marker() {
        assert!(unknown_markers("accolade { seule", &MARKERS).is_empty());
    }

    #[test]
    fn an_announce_names_the_member_and_the_card() {
        assert_eq!(
            render_announce("{membre} → {carte}", "Veille", "<@1>"),
            "<@1> → Veille"
        );
    }
}

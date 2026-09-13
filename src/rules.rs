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
}

//! État partagé : le pool SQLite, la configuration, et les caches qui servent
//! le chemin chaud des événements.
//!
//! # Pourquoi des caches
//!
//! Une réaction sur une carte doit être traitée sans appel réseau ni requête
//! superflue : Discord en envoie autant que de membres qui cliquent. Les index
//! ci-dessous suffisent à décider, en mémoire, si un événement nous concerne,
//! quels rôles il accorde, et si le fil doit envoyer un message privé.
//!
//! Il n'y a volontairement pas d'index des salons : un identifiant de message
//! est unique à l'échelle de Discord, donc le retrouver dans `post_by_message`
//! suffit à savoir qu'il s'agit d'une de nos cartes.
//!
//! # Comment ils restent justes
//!
//! Toute commande qui écrit en base appelle ensuite [`Data::reload_caches`],
//! qui relit les index d'un bloc. C'est une poignée de requêtes sur un chemin
//! froid — une commande d'administration — en échange de l'impossibilité
//! structurelle d'oublier une invalidation.
//!
//! L'ancien bot faisait l'inverse : il lisait la liste des salons **une fois à
//! l'import du module**, si bien que créer un fil exigeait de le redémarrer.

use crate::config::Config;
use crate::db;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Ce qu'une réaction a besoin de savoir sur le message qu'elle vise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostRef {
    pub post_id: i64,
    pub category_id: i64,
    /// Rôle nominatif du message. `None` = le message n'accorde que le rôle
    /// parent de son fil (main levée quand même).
    pub role_id: Option<i64>,
    /// Rôle parent du fil, accordé en plus de celui du message. `None` aussi
    /// quand la carte y renonce : rien à accorder, rien à reprendre.
    pub parent_role_id: Option<i64>,
    /// La carte a son propre message privé, qui prime sur celui du fil.
    pub sends_own_dm: bool,
}

pub struct Data {
    pub db: SqlitePool,
    pub config: Config,
    caches: RwLock<Caches>,
}

#[derive(Default)]
struct Caches {
    /// Carte publiée → ce qu'elle accorde. Les messages d'information, sans
    /// rôle, n'y figurent pas : une réaction dessus n'a rien à déclencher.
    post_by_message: HashMap<i64, PostRef>,
    /// Fils qui envoient un message privé à la première réaction.
    dm_categories: HashSet<i64>,
    /// Rôle parent → tous les rôles de message qui l'accordent, c'est-à-dire
    /// ceux des messages des fils qui le déclarent.
    parent_grants: HashMap<i64, Vec<i64>>,
}

impl Data {
    pub async fn new(db: SqlitePool, config: Config) -> anyhow::Result<Self> {
        let data = Data {
            db,
            config,
            caches: RwLock::new(Caches::default()),
        };
        data.reload_caches().await?;
        Ok(data)
    }

    /// Relit tous les index depuis la base. À appeler après chaque écriture.
    pub async fn reload_caches(&self) -> anyhow::Result<()> {
        let categories = db::categories::list(&self.db).await?;
        let posts = db::posts::all(&self.db).await?;

        let parent_of: HashMap<i64, Option<i64>> = categories
            .iter()
            .map(|category| (category.id, category.parent_role_id))
            .collect();

        let mut post_by_message = HashMap::new();
        let mut parent_grants: HashMap<i64, Vec<i64>> = HashMap::new();
        for post in &posts {
            let parent_role_id = parent_of.get(&post.category_id).copied().flatten();

            // Une carte qui renonce au rôle parent n'en accorde aucun, quoi
            // qu'en dise son fil.
            let parent_role_id = parent_role_id.filter(|_| post.grants_parent);
            let sends_own_dm = post
                .dm_text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty());

            // Un message d'information n'accorde rien et ne porte pas de
            // réaction : il n'a pas à figurer dans l'index du chemin chaud. Un
            // message qui n'accorde aucun rôle non plus — sauf s'il a son
            // propre message privé, car alors la main levée sert à le demander.
            if post.information
                || (post.role_id.is_none() && parent_role_id.is_none() && !sends_own_dm)
            {
                continue;
            }

            // Seuls les rôles nominatifs alimentent `parent_grants` : c'est la
            // justification du rôle parent « par rôle porté ». Les messages sans
            // rôle, eux, sont justifiés par les réactions (table `reactions`).
            if let (Some(role_id), Some(parent)) = (post.role_id, parent_role_id) {
                parent_grants.entry(parent).or_default().push(role_id);
            }
            if let Some(message_id) = post.message_id {
                post_by_message.insert(
                    message_id,
                    PostRef {
                        post_id: post.id,
                        category_id: post.category_id,
                        role_id: post.role_id,
                        parent_role_id,
                        sends_own_dm,
                    },
                );
            }
        }

        *self.write() = Caches {
            dm_categories: categories
                .iter()
                .filter(|category| {
                    category
                        .dm_text
                        .as_deref()
                        .is_some_and(|text| !text.trim().is_empty())
                })
                .map(|category| category.id)
                .collect(),
            post_by_message,
            parent_grants,
        };
        Ok(())
    }

    pub fn post_for_message(&self, message_id: i64) -> Option<PostRef> {
        self.read().post_by_message.get(&message_id).copied()
    }

    pub fn sends_dm(&self, category_id: i64) -> bool {
        self.read().dm_categories.contains(&category_id)
    }

    /// Tous les rôles de message qui accordent ce rôle parent.
    pub fn granting_roles(&self, parent_role_id: i64) -> Vec<i64> {
        self.read()
            .parent_grants
            .get(&parent_role_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Un verrou empoisonné n'est pas dangereux ici : les caches sont des
    /// données dérivées, reconstruites par [`Data::reload_caches`]. On préfère
    /// récupérer le contenu plutôt que propager la panique à chaque événement.
    fn read(&self) -> std::sync::RwLockReadGuard<'_, Caches> {
        self.caches.read().unwrap_or_else(|err| err.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Caches> {
        self.caches.write().unwrap_or_else(|err| err.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{categories, posts};

    async fn data_with(pool: SqlitePool) -> Data {
        Data::new(pool, Config::fixture()).await.unwrap()
    }

    #[tokio::test]
    async fn caches_reflect_what_the_database_holds() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(
            &pool,
            &categories::Category {
                dm_text: Some("Bienvenue".into()),
                parent_role_id: Some(999),
                ..categories::fixture("Groupes locaux", 100)
            },
        )
        .await
        .unwrap();

        let paris = posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                ..posts::fixture(category.id, "paris", Some(10))
            },
        )
        .await
        .unwrap();

        let data = data_with(pool).await;

        assert!(data.sends_dm(category.id));
        assert_eq!(
            data.post_for_message(5000),
            Some(PostRef {
                post_id: paris.id,
                category_id: category.id,
                role_id: Some(10),
                parent_role_id: Some(999),
                sends_own_dm: false,
            })
        );
        assert_eq!(data.granting_roles(999), vec![10]);
    }

    #[tokio::test]
    async fn a_message_without_role_but_with_a_parent_is_indexed() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(
            &pool,
            &categories::Category {
                parent_role_id: Some(999),
                ..categories::fixture("Projets", 100)
            },
        )
        .await
        .unwrap();
        let post = posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                ..posts::fixture(category.id, "fresque", None)
            },
        )
        .await
        .unwrap();

        // Réagir dessus doit accorder le seul rôle parent : la carte est indexée
        // avec un rôle nominatif absent.
        assert_eq!(
            data_with(pool).await.post_for_message(5000),
            Some(PostRef {
                post_id: post.id,
                category_id: category.id,
                role_id: None,
                parent_role_id: Some(999),
                sends_own_dm: false,
            })
        );
    }

    #[tokio::test]
    async fn an_information_message_is_never_indexed_even_with_a_parent() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(
            &pool,
            &categories::Category {
                parent_role_id: Some(999),
                ..categories::fixture("Projets", 100)
            },
        )
        .await
        .unwrap();
        posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                information: true,
                ..posts::fixture(category.id, "intro", None)
            },
        )
        .await
        .unwrap();

        // Marqué information : aucune réaction, rien à déclencher.
        assert_eq!(data_with(pool).await.post_for_message(5000), None);
    }

    #[tokio::test]
    async fn a_card_that_only_sends_a_message_carries_a_hand_without_a_role() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(
            &pool,
            &categories::Category {
                parent_role_id: Some(999),
                ..categories::fixture("Projets", 100)
            },
        )
        .await
        .unwrap();
        let post = posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                dm_text: Some("Voici le lien du groupe".into()),
                grants_parent: false,
                ..posts::fixture(category.id, "pauseaction", None)
            },
        )
        .await
        .unwrap();

        // Indexée pour que la main levée déclenche son message privé, mais sans
        // le rôle parent du fil : elle y a renoncé.
        assert_eq!(
            data_with(pool).await.post_for_message(5000),
            Some(PostRef {
                post_id: post.id,
                category_id: category.id,
                role_id: None,
                parent_role_id: None,
                sends_own_dm: true,
            })
        );
    }

    #[tokio::test]
    async fn an_information_message_is_not_indexed() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(&pool, &categories::fixture("Projets", 100))
            .await
            .unwrap();
        posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                ..posts::fixture(category.id, "intro", None)
            },
        )
        .await
        .unwrap();

        // Réagir dessus ne doit rien déclencher : le message n'accorde rien.
        assert_eq!(data_with(pool).await.post_for_message(5000), None);
    }

    #[tokio::test]
    async fn a_thread_without_dm_text_sends_nothing() {
        let pool = db::connect_in_memory().await;
        let bare = categories::insert(&pool, &categories::fixture("Projets", 100))
            .await
            .unwrap();
        let blank = categories::insert(
            &pool,
            &categories::Category {
                dm_text: Some("   ".into()),
                ..categories::fixture("Équipes", 200)
            },
        )
        .await
        .unwrap();

        let data = data_with(pool).await;
        assert!(!data.sends_dm(bare.id));
        // Un texte réduit à des espaces ne vaut pas un message privé.
        assert!(!data.sends_dm(blank.id));
    }

    #[tokio::test]
    async fn every_group_granting_the_shared_role_is_listed() {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(
            &pool,
            &categories::Category {
                parent_role_id: Some(999),
                ..categories::fixture("Groupes locaux", 100)
            },
        )
        .await
        .unwrap();
        for (slug, role) in [("paris", 10), ("lyon", 11), ("lille", 12)] {
            posts::insert(&pool, &posts::fixture(category.id, slug, Some(role)))
                .await
                .unwrap();
        }

        let mut granting = data_with(pool).await.granting_roles(999);
        granting.sort_unstable();
        assert_eq!(granting, vec![10, 11, 12]);
    }

    #[tokio::test]
    async fn reloading_picks_up_a_new_thread_without_restart() {
        let pool = db::connect_in_memory().await;
        let data = data_with(pool.clone()).await;
        assert_eq!(data.post_for_message(5000), None);

        let category = categories::insert(&pool, &categories::fixture("Projets", 100))
            .await
            .unwrap();
        posts::insert(
            &pool,
            &posts::Post {
                message_id: Some(5000),
                ..posts::fixture(category.id, "fresque-ia", Some(10))
            },
        )
        .await
        .unwrap();
        data.reload_caches().await.unwrap();

        // Régression de l'ancien bot, qui exigeait un redémarrage après chaque
        // création de fil.
        assert!(data.post_for_message(5000).is_some());
    }
}

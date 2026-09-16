//! Les messages du forum.
//!
//! Un message porte son propre titre et son propre texte, indépendants du rôle
//! qu'il accorde : « Fresque de l'IA » n'est pas le nom du rôle `@fresque-ia`.
//! Il peut n'accorder aucun rôle — c'est alors un message d'information, sans
//! réaction.
//!
//! Le second rôle, lui, n'appartient pas au message : c'est le rôle parent de
//! son fil, le même pour tous ses messages.
//!
//! `message_id` mémorise la carte publiée. Elle sert à deux choses : modifier
//! la carte sur place au lieu de réécrire le fil entier, ce qui préserve les
//! réactions des membres, et résoudre une réaction en message sans aucun appel
//! à l'API Discord.

use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Post {
    pub id: i64,
    pub category_id: i64,
    /// Identité stable pour le réimport du fichier de contenu.
    pub slug: String,
    pub title: String,
    pub body: String,
    /// `None` = pas de rôle nominatif. Le message peut tout de même accorder le
    /// rôle parent de son fil, sauf s'il est marqué `information`.
    pub role_id: Option<i64>,
    /// Couleur propre au message, qui prime sur celle du fil.
    pub colour: Option<i64>,
    pub message_id: Option<i64>,
    pub position: i64,
    /// Message d'information : aucune réaction, n'accorde rien, même dans un fil
    /// à rôle parent.
    pub information: bool,
    /// Message privé propre à la carte, qui prime sur celui du fil.
    pub dm_text: Option<String>,
    /// La carte accorde-t-elle le rôle parent de son fil ? Vrai par défaut :
    /// c'est la règle du fil. Le mettre à faux laisse une carte porter une main
    /// levée sans accorder quoi que ce soit.
    pub grants_parent: bool,
    /// Référent·e à mentionner quand une main se lève sur cette carte.
    pub referent_id: Option<i64>,
}

pub async fn all(pool: &SqlitePool) -> sqlx::Result<Vec<Post>> {
    sqlx::query_as::<_, Post>(
        "SELECT id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id
           FROM posts ORDER BY category_id, position",
    )
    .fetch_all(pool)
    .await
}

pub async fn by_id(pool: &SqlitePool, id: i64) -> sqlx::Result<Option<Post>> {
    sqlx::query_as::<_, Post>(
        "SELECT id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id
           FROM posts WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn by_slug(
    pool: &SqlitePool,
    category_id: i64,
    slug: &str,
) -> sqlx::Result<Option<Post>> {
    sqlx::query_as::<_, Post>(
        "SELECT id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id
           FROM posts WHERE category_id = ? AND slug = ?",
    )
    .bind(category_id)
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn by_role(pool: &SqlitePool, role_id: i64) -> sqlx::Result<Option<Post>> {
    sqlx::query_as::<_, Post>(
        "SELECT id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id
           FROM posts WHERE role_id = ?",
    )
    .bind(role_id)
    .fetch_optional(pool)
    .await
}

/// Messages d'un fil, dans l'ordre d'affichage voulu.
pub async fn by_category(pool: &SqlitePool, category_id: i64) -> sqlx::Result<Vec<Post>> {
    sqlx::query_as::<_, Post>(
        "SELECT id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id
           FROM posts WHERE category_id = ? ORDER BY position, id",
    )
    .bind(category_id)
    .fetch_all(pool)
    .await
}

/// Rang libre à la fin d'un fil, pour un message ajouté sans position choisie.
pub async fn next_position(pool: &SqlitePool, category_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(position) FROM posts WHERE category_id = ?")
        .bind(category_id)
        .fetch_one(pool)
        .await
        .map(|highest| highest.unwrap_or(0) + 1)
}

pub async fn insert(pool: &SqlitePool, post: &Post) -> sqlx::Result<Post> {
    sqlx::query_as::<_, Post>(
        "INSERT INTO posts (category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, category_id, slug, title, body, role_id, colour, message_id, position, information,
                  dm_text, grants_parent, referent_id",
    )
    .bind(post.category_id)
    .bind(&post.slug)
    .bind(&post.title)
    .bind(&post.body)
    .bind(post.role_id)
    .bind(post.colour)
    .bind(post.message_id)
    .bind(post.position)
    .bind(post.information)
    .bind(&post.dm_text)
    .bind(post.grants_parent)
    .bind(post.referent_id)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &SqlitePool, post: &Post) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE posts
            SET category_id = ?, slug = ?, title = ?, body = ?,
                role_id = ?, colour = ?, message_id = ?, position = ?, information = ?,
                dm_text = ?, grants_parent = ?, referent_id = ?
          WHERE id = ?",
    )
    .bind(post.category_id)
    .bind(&post.slug)
    .bind(&post.title)
    .bind(&post.body)
    .bind(post.role_id)
    .bind(post.colour)
    .bind(post.message_id)
    .bind(post.position)
    .bind(post.information)
    .bind(&post.dm_text)
    .bind(post.grants_parent)
    .bind(post.referent_id)
    .bind(post.id)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn set_message_id(
    pool: &SqlitePool,
    id: i64,
    message_id: Option<i64>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE posts SET message_id = ? WHERE id = ?")
        .bind(message_id)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Oublie les cartes d'un fil avant sa réécriture complète : les messages
/// qu'elles désignaient vont être supprimés.
pub async fn clear_message_ids(pool: &SqlitePool, category_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE posts SET message_id = NULL WHERE category_id = ?")
        .bind(category_id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

#[cfg(test)]
pub(crate) fn fixture(category_id: i64, slug: &str, role_id: Option<i64>) -> Post {
    Post {
        id: 0,
        category_id,
        slug: slug.to_owned(),
        title: slug.to_owned(),
        body: String::new(),
        role_id,
        colour: None,
        message_id: None,
        position: 0,
        information: false,
        dm_text: None,
        grants_parent: true,
        referent_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::categories;

    async fn seeded() -> (SqlitePool, i64) {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(&pool, &categories::fixture("Groupes locaux", 100))
            .await
            .unwrap();
        (pool, category.id)
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let (pool, category_id) = seeded().await;

        let created = insert(&pool, &fixture(category_id, "paris", Some(10)))
            .await
            .unwrap();
        assert!(created.id > 0);
        assert_eq!(
            by_id(&pool, created.id).await.unwrap(),
            Some(created.clone())
        );
        assert_eq!(by_role(&pool, 10).await.unwrap(), Some(created.clone()));
        assert_eq!(
            by_slug(&pool, category_id, "paris").await.unwrap(),
            Some(created.clone())
        );

        set_message_id(&pool, created.id, Some(999)).await.unwrap();
        assert_eq!(
            by_id(&pool, created.id).await.unwrap().unwrap().message_id,
            Some(999)
        );

        delete(&pool, created.id).await.unwrap();
        assert!(by_id(&pool, created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn an_information_message_carries_no_role() {
        let (pool, category_id) = seeded().await;

        let created = insert(&pool, &fixture(category_id, "intro", None))
            .await
            .unwrap();
        assert_eq!(created.role_id, None);
        // Plusieurs messages sans rôle doivent coexister : l'index unique sur
        // role_id est partiel et ne doit pas les considérer comme identiques.
        assert!(
            insert(&pool, &fixture(category_id, "note", None))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_role_belongs_to_a_single_message() {
        let (pool, category_id) = seeded().await;
        insert(&pool, &fixture(category_id, "paris", Some(10)))
            .await
            .unwrap();

        // Sans cette contrainte, la réévaluation du rôle partagé n'aurait pas
        // de réponse unique.
        assert!(
            insert(&pool, &fixture(category_id, "lyon", Some(10)))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_slug_is_unique_within_its_thread_only() {
        let pool = db::connect_in_memory().await;
        let first = categories::insert(&pool, &categories::fixture("Projets", 100))
            .await
            .unwrap();
        let second = categories::insert(&pool, &categories::fixture("Équipes", 200))
            .await
            .unwrap();

        insert(&pool, &fixture(first.id, "communication", Some(10)))
            .await
            .unwrap();
        // Même slug dans un autre fil : accepté, c'est un autre message.
        assert!(
            insert(&pool, &fixture(second.id, "communication", Some(11)))
                .await
                .is_ok()
        );
        // Même slug dans le même fil : refusé, le réimport ne saurait plus
        // lequel des deux mettre à jour.
        assert!(
            insert(&pool, &fixture(first.id, "communication", Some(12)))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn messages_come_back_in_display_order() {
        let (pool, category_id) = seeded().await;
        for (slug, position) in [("lyon", 3), ("paris", 1), ("lille", 2)] {
            insert(
                &pool,
                &Post {
                    position,
                    ..fixture(category_id, slug, None)
                },
            )
            .await
            .unwrap();
        }

        let ordered: Vec<_> = by_category(&pool, category_id)
            .await
            .unwrap()
            .into_iter()
            .map(|post| post.slug)
            .collect();
        assert_eq!(ordered, vec!["paris", "lille", "lyon"]);
    }

    #[tokio::test]
    async fn next_position_appends_at_the_end() {
        let (pool, category_id) = seeded().await;
        assert_eq!(next_position(&pool, category_id).await.unwrap(), 1);

        insert(
            &pool,
            &Post {
                position: 7,
                ..fixture(category_id, "paris", None)
            },
        )
        .await
        .unwrap();
        assert_eq!(next_position(&pool, category_id).await.unwrap(), 8);
    }

    #[tokio::test]
    async fn deleting_a_thread_takes_its_messages() {
        let (pool, category_id) = seeded().await;
        insert(&pool, &fixture(category_id, "paris", Some(10)))
            .await
            .unwrap();

        categories::delete(&pool, category_id).await.unwrap();
        assert!(all(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn clearing_message_ids_leaves_the_rows() {
        let (pool, category_id) = seeded().await;
        let created = insert(&pool, &fixture(category_id, "paris", Some(10)))
            .await
            .unwrap();
        set_message_id(&pool, created.id, Some(999)).await.unwrap();

        clear_message_ids(&pool, category_id).await.unwrap();
        assert_eq!(
            by_id(&pool, created.id).await.unwrap().unwrap().message_id,
            None
        );
        assert_eq!(all(&pool).await.unwrap().len(), 1);
    }
}

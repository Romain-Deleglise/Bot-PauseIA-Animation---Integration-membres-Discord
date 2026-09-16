//! Les fils du forum.
//!
//! Un fil = un salon (ou un fil de forum) dans lequel les cartes de ses
//! messages sont publiées. Ces enregistrements vivaient dans la collection
//! `roles_categories` de Directus et se géraient depuis son interface ; ils se
//! pilotent désormais par les commandes `/forum fil …`.

use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub channel_id: i64,
    pub header_image_url: Option<String>,
    pub header_text: Option<String>,
    pub colour: Option<i64>,
    /// Rôle accordé en plus du sien par tous les messages de ce fil.
    pub parent_role_id: Option<i64>,
    /// Message privé envoyé à la première réaction dans ce fil.
    pub dm_text: Option<String>,
    /// Confirmer en privé chaque entrée et chaque sortie de ce fil.
    pub confirmations: bool,
    /// Salon où annoncer les mains levées de ce fil, pour que quelqu'un le sache.
    pub notify_channel_id: Option<i64>,
}

/// Clé de recherche et d'unicité d'un nom de fil.
///
/// Repliée en minuscules par Rust, qui applique les règles Unicode complètes,
/// là où la collation `NOCASE` de SQLite s'arrête à l'ASCII.
pub fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub async fn list(pool: &SqlitePool) -> sqlx::Result<Vec<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, name, channel_id, header_image_url, header_text, colour,
                parent_role_id, dm_text, confirmations,
                   notify_channel_id
           FROM categories ORDER BY name",
    )
    .fetch_all(pool)
    .await
}

pub async fn by_id(pool: &SqlitePool, id: i64) -> sqlx::Result<Option<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, name, channel_id, header_image_url, header_text, colour,
                parent_role_id, dm_text, confirmations,
                   notify_channel_id
           FROM categories WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Recherche insensible à la casse : les noms viennent d'une saisie humaine ou
/// d'une autocomplétion, pas d'un identifiant.
pub async fn by_name(pool: &SqlitePool, name: &str) -> sqlx::Result<Option<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, name, channel_id, header_image_url, header_text, colour,
                parent_role_id, dm_text, confirmations,
                   notify_channel_id
           FROM categories WHERE name_key = ?",
    )
    .bind(name_key(name))
    .fetch_optional(pool)
    .await
}

pub async fn by_channel(pool: &SqlitePool, channel_id: i64) -> sqlx::Result<Option<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, name, channel_id, header_image_url, header_text, colour,
                parent_role_id, dm_text, confirmations,
                   notify_channel_id
           FROM categories WHERE channel_id = ?",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
}

pub async fn insert(pool: &SqlitePool, category: &Category) -> sqlx::Result<Category> {
    sqlx::query_as::<_, Category>(
        "INSERT INTO categories (name, name_key, channel_id, header_image_url, header_text,
                                 colour, parent_role_id, dm_text, confirmations,
                                 notify_channel_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, name, channel_id, header_image_url, header_text, colour,
                   parent_role_id, dm_text, confirmations, notify_channel_id",
    )
    .bind(&category.name)
    .bind(name_key(&category.name))
    .bind(category.channel_id)
    .bind(&category.header_image_url)
    .bind(&category.header_text)
    .bind(category.colour)
    .bind(category.parent_role_id)
    .bind(&category.dm_text)
    .bind(category.confirmations)
    .bind(category.notify_channel_id)
    .fetch_one(pool)
    .await
}

/// Réécrit la ligne entière. Les commandes lisent l'existant, appliquent leurs
/// modifications et repassent l'objet complet.
pub async fn update(pool: &SqlitePool, category: &Category) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE categories
            SET name = ?, name_key = ?, channel_id = ?, header_image_url = ?,
                header_text = ?, colour = ?, parent_role_id = ?, dm_text = ?,
                confirmations = ?, notify_channel_id = ?
          WHERE id = ?",
    )
    .bind(&category.name)
    .bind(name_key(&category.name))
    .bind(category.channel_id)
    .bind(&category.header_image_url)
    .bind(&category.header_text)
    .bind(category.colour)
    .bind(category.parent_role_id)
    .bind(&category.dm_text)
    .bind(category.confirmations)
    .bind(category.notify_channel_id)
    .bind(category.id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Supprime le fil. Ses messages partent avec lui (`ON DELETE CASCADE`), ce qui
/// suppose `foreign_keys` activé sur la connexion.
pub async fn delete(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

#[cfg(test)]
pub(crate) fn fixture(name: &str, channel_id: i64) -> Category {
    Category {
        id: 0,
        name: name.to_owned(),
        channel_id,
        header_image_url: None,
        header_text: None,
        colour: None,
        parent_role_id: None,
        dm_text: None,
        confirmations: false,
        notify_channel_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn crud_roundtrip() {
        let pool = db::connect_in_memory().await;

        let created = insert(&pool, &fixture("Projets", 100)).await.unwrap();
        assert!(created.id > 0);

        assert_eq!(
            by_id(&pool, created.id).await.unwrap(),
            Some(created.clone())
        );
        assert_eq!(by_channel(&pool, 100).await.unwrap(), Some(created.clone()));

        let renamed = Category {
            name: "Projets PauseIA".into(),
            colour: Some(0xFF6600),
            parent_role_id: Some(42),
            dm_text: Some("Bienvenue".into()),
            ..created.clone()
        };
        update(&pool, &renamed).await.unwrap();
        assert_eq!(by_id(&pool, created.id).await.unwrap(), Some(renamed));

        delete(&pool, created.id).await.unwrap();
        assert_eq!(by_id(&pool, created.id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn lookup_by_name_ignores_case_including_accents() {
        let pool = db::connect_in_memory().await;
        insert(&pool, &fixture("Compétences", 100)).await.unwrap();

        assert!(by_name(&pool, "compétences").await.unwrap().is_some());
        // La collation NOCASE de SQLite échouerait ici : elle ignore le É.
        assert!(by_name(&pool, "COMPÉTENCES").await.unwrap().is_some());
        assert!(by_name(&pool, "  Compétences  ").await.unwrap().is_some());
        assert!(by_name(&pool, "autre").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn names_are_unique_regardless_of_case() {
        let pool = db::connect_in_memory().await;
        insert(&pool, &fixture("Équipes", 100)).await.unwrap();

        // Deux fils que les humains liraient comme le même : l'autocomplétion
        // deviendrait ambiguë, la base doit refuser.
        assert!(insert(&pool, &fixture("ÉQUIPES", 200)).await.is_err());
    }

    #[tokio::test]
    async fn one_thread_per_channel() {
        let pool = db::connect_in_memory().await;
        insert(&pool, &fixture("Projets", 100)).await.unwrap();

        // Deux fils dans le même salon rendraient l'assignation ambiguë.
        assert!(insert(&pool, &fixture("Équipes", 100)).await.is_err());
    }
}

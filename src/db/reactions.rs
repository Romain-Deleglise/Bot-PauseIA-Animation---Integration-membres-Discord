//! Suivi des mains levées, par membre et par message.
//!
//! Quand un message n'a pas de rôle nominatif, il n'accorde que le rôle parent
//! de son fil. On ne peut alors plus déduire des rôles portés s'il reste, au
//! retrait d'une réaction, une autre main levée du fil justifiant ce parent.
//! Cette table conserve donc les réactions posées sur les messages **sans rôle**
//! (les réactions sur un message à rôle nominatif sont, elles, retrouvables via
//! le rôle porté). Le rôle parent n'est repris que lorsque plus aucune de ces
//! réactions ne subsiste dans le fil.

use sqlx::SqlitePool;

/// Enregistre une main levée. Idempotent : réagir deux fois ne crée qu'une ligne.
pub async fn add(pool: &SqlitePool, member_id: i64, post_id: i64) -> sqlx::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO reactions (member_id, post_id) VALUES (?, ?)")
        .bind(member_id)
        .bind(post_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Oublie la main levée d'un membre sur un message.
pub async fn remove(pool: &SqlitePool, member_id: i64, post_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM reactions WHERE member_id = ? AND post_id = ?")
        .bind(member_id)
        .bind(post_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Reste-t-il une main levée du membre sur un **autre** message sans rôle du
/// même fil, qui justifierait encore le rôle parent ?
///
/// `excluded_post_id` est le message dont on vient de retirer la réaction : il
/// ne doit pas se justifier lui-même.
pub async fn parent_still_justified(
    pool: &SqlitePool,
    member_id: i64,
    category_id: i64,
    excluded_post_id: i64,
) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1
               FROM reactions r
               JOIN posts p ON p.id = r.post_id
              WHERE r.member_id = ?
                AND p.category_id = ?
                AND p.role_id IS NULL
                AND p.information = 0
                AND p.id != ?
         )",
    )
    .bind(member_id)
    .bind(category_id)
    .bind(excluded_post_id)
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::{categories, posts};

    /// Un fil avec un rôle parent et deux messages sans rôle nominatif.
    async fn seeded() -> (SqlitePool, i64, i64, i64) {
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
        let a = posts::insert(&pool, &posts::fixture(category.id, "a", None))
            .await
            .unwrap();
        let b = posts::insert(&pool, &posts::fixture(category.id, "b", None))
            .await
            .unwrap();
        (pool, category.id, a.id, b.id)
    }

    #[tokio::test]
    async fn a_claim_is_recorded_once() {
        let (pool, _cat, a, _b) = seeded().await;
        add(&pool, 1, a).await.unwrap();
        // Réagir de nouveau ne doit pas échouer ni dupliquer.
        add(&pool, 1, a).await.unwrap();
    }

    #[tokio::test]
    async fn another_claim_in_the_thread_still_justifies_the_parent() {
        let (pool, cat, a, b) = seeded().await;
        add(&pool, 1, a).await.unwrap();
        add(&pool, 1, b).await.unwrap();

        // On retire la réaction sur a : b la justifie encore.
        remove(&pool, 1, a).await.unwrap();
        assert!(parent_still_justified(&pool, 1, cat, a).await.unwrap());
    }

    #[tokio::test]
    async fn the_last_claim_leaving_ends_the_parent() {
        let (pool, cat, a, b) = seeded().await;
        add(&pool, 1, a).await.unwrap();

        remove(&pool, 1, a).await.unwrap();
        // Plus aucune réaction du membre dans le fil.
        assert!(!parent_still_justified(&pool, 1, cat, a).await.unwrap());
        // Un autre message existe (b) mais le membre n'y a pas réagi.
        let _ = b;
    }

    #[tokio::test]
    async fn claims_of_other_members_do_not_count() {
        let (pool, cat, a, b) = seeded().await;
        add(&pool, 2, b).await.unwrap();
        // Le membre 1 n'a aucune réaction : le membre 2 ne le justifie pas.
        assert!(!parent_still_justified(&pool, 1, cat, a).await.unwrap());
    }

    #[tokio::test]
    async fn a_deleted_message_takes_its_claims() {
        let (pool, _cat, a, _b) = seeded().await;
        add(&pool, 1, a).await.unwrap();

        posts::delete(&pool, a).await.unwrap();
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0);
    }
}

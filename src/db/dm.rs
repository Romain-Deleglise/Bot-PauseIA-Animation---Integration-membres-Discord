//! Suivi des messages privés déjà envoyés.
//!
//! Le CDC demande qu'un membre qui réagit à plusieurs messages d'un même fil ne
//! reçoive le message privé du fil **qu'une seule fois, peu importe le délai**.
//! Une ligne par couple membre-fil suffit, et elle n'expire jamais.
//!
//! L'envoi se **réserve** avant de partir. Serenity traite les événements de
//! réaction en parallèle : deux clics rapprochés liraient tous deux une absence
//! de ligne et enverraient chacun leur message. `claim` s'appuie donc sur
//! l'atomicité de l'insertion plutôt que sur une lecture préalable.

use sqlx::SqlitePool;

/// Réserve l'envoi. Rend `true` à celui qui a gagné la course, et à lui seul.
pub async fn claim(pool: &SqlitePool, member_id: i64, category_id: i64) -> sqlx::Result<bool> {
    sqlx::query("INSERT OR IGNORE INTO dm_sent (member_id, category_id) VALUES (?, ?)")
        .bind(member_id)
        .bind(category_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected() == 1)
}

/// Libère une réservation dont l'envoi a échoué, pour que la prochaine réaction
/// réessaie. Un membre aux messages privés fermés peut les rouvrir plus tard.
pub async fn release(pool: &SqlitePool, member_id: i64, category_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM dm_sent WHERE member_id = ? AND category_id = ?")
        .bind(member_id)
        .bind(category_id)
        .execute(pool)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::categories;

    async fn seeded() -> (SqlitePool, i64) {
        let pool = db::connect_in_memory().await;
        let category = categories::insert(&pool, &categories::fixture("Équipes", 100))
            .await
            .unwrap();
        (pool, category.id)
    }

    #[tokio::test]
    async fn only_the_first_claim_wins() {
        let (pool, category_id) = seeded().await;

        assert!(claim(&pool, 1, category_id).await.unwrap());
        // Deuxième réaction du même membre dans le même fil : pas de second envoi.
        assert!(!claim(&pool, 1, category_id).await.unwrap());
        assert!(!claim(&pool, 1, category_id).await.unwrap());
    }

    #[tokio::test]
    async fn a_failed_send_can_be_retried() {
        let (pool, category_id) = seeded().await;

        assert!(claim(&pool, 1, category_id).await.unwrap());
        release(&pool, 1, category_id).await.unwrap();
        // Messages privés rouverts depuis : la prochaine réaction repart.
        assert!(claim(&pool, 1, category_id).await.unwrap());
    }

    #[tokio::test]
    async fn claims_are_per_member_and_per_thread() {
        let pool = db::connect_in_memory().await;
        let first = categories::insert(&pool, &categories::fixture("Équipes", 100))
            .await
            .unwrap();
        let second = categories::insert(&pool, &categories::fixture("Projets", 200))
            .await
            .unwrap();

        assert!(claim(&pool, 1, first.id).await.unwrap());
        // Un autre membre dans le même fil, et le même membre dans un autre
        // fil, reçoivent bien leur message.
        assert!(claim(&pool, 2, first.id).await.unwrap());
        assert!(claim(&pool, 1, second.id).await.unwrap());
    }

    #[tokio::test]
    async fn deleting_a_thread_forgets_its_sends() {
        let (pool, category_id) = seeded().await;
        claim(&pool, 1, category_id).await.unwrap();

        categories::delete(&pool, category_id).await.unwrap();
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dm_sent")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0);
    }
}

//! Accès à la base SQLite.
//!
//! Chaque sous-module correspond à une table et n'expose que des opérations
//! nommées métier — aucun SQL ne remonte au-dessus de cette couche.
//!
//! Les identifiants Discord voyagent en `i64` ici : c'est le type entier de
//! SQLite. La conversion depuis les types serenity (`RoleId`, `MessageId`, …)
//! se fait aux frontières, via les aides de [`crate::ids`].

pub mod categories;
pub mod dm;
pub mod posts;

use anyhow::Context as _;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::time::Duration;

/// Ouvre la base, applique les migrations embarquées et rend le pool.
///
/// Le fichier et son répertoire parent sont créés au besoin : un premier
/// démarrage sur un volume vide doit fonctionner sans préparation manuelle.
pub async fn connect(path: &str) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = std::path::Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création du répertoire de la base `{}`", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // WAL : les lectures des événements ne bloquent pas les écritures des commandes.
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .with_context(|| {
            // Panne numéro un au premier démarrage : le volume appartient à
            // root alors que le conteneur tourne en 65534. Le dire ici évite
            // de partir sur une fausse piste.
            format!(
                "ouverture de la base `{path}`\n\
                 En conteneur, vérifiez que le volume monté appartient bien à \
                 l'utilisateur du conteneur : `chown -R 65534:65534 <volume>`."
            )
        })?;

    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("application des migrations")
}

/// Base éphémère en mémoire, pour les tests.
///
/// Une seule connexion : avec `sqlite::memory:`, chaque connexion ouvrirait sa
/// propre base et les tests ne verraient pas leurs propres écritures.
#[cfg(test)]
pub async fn connect_in_memory() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("ouverture de la base en mémoire");
    migrate(&pool).await.expect("migrations");
    pool
}

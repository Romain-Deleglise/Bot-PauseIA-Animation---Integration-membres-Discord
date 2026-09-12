-- Schéma initial du Bot Forum.
--
-- Le modèle est centré sur le message, pas sur le rôle : un message porte un
-- titre et un texte qui lui sont propres, et donne zéro ou un rôle. Le fil peut
-- y ajouter un rôle parent, accordé par tous ses messages.
--
-- Les PRAGMA (WAL, foreign_keys, busy_timeout) ne sont pas ici : SQLite refuse
-- de changer le mode de journal dans une transaction, or sqlx exécute chaque
-- migration dans une transaction. Ils sont posés à l'ouverture de la connexion,
-- dans src/db/mod.rs.
--
-- Tous les identifiants Discord sont stockés en INTEGER : un snowflake tient
-- dans un i64 signé.

-- Un fil du forum.
CREATE TABLE categories (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Nom tel qu'affiché, casse et accents d'origine préservés.
    name                      TEXT    NOT NULL,
    -- Même nom replié en minuscules par Rust. La collation NOCASE de SQLite ne
    -- replie que l'ASCII : « COMPÉTENCES » et « Compétences » lui semblent
    -- distincts, ce qui casserait autant la recherche que l'unicité.
    name_key                  TEXT    NOT NULL UNIQUE,
    channel_id                INTEGER NOT NULL UNIQUE,
    -- Illustration publiée en tête du fil.
    header_image_url          TEXT,
    -- Texte d'introduction publié en tête du fil.
    header_text               TEXT,
    -- 0xRRGGBB, couleur de la bordure des cartes du fil.
    colour                    INTEGER,
    -- Rôle parent : accordé en plus du sien par tous les messages du fil.
    -- @groupe-local pour les groupes locaux, @portail-équipe pour les équipes.
    -- Un fil en a zéro ou un, et il se change par commande.
    parent_role_id            INTEGER,
    -- Message privé envoyé à la première réaction dans ce fil. NULL = aucun.
    dm_text                   TEXT,
    created_at                INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    -- Identité stable pour le réimport du fichier de contenu. Un message sans
    -- rôle n'a pas d'autre clé métier : `role_id` est NULL, et le titre peut
    -- être corrigé d'une version à l'autre.
    slug        TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    body        TEXT    NOT NULL DEFAULT '',
    -- NULL = message d'information : aucune réaction n'est posée dessus.
    role_id     INTEGER,
    -- 0xRRGGBB. Prime sur la couleur du fil : sert à griser un message endormi
    -- sans avoir à décrire un état quelque part. NULL = couleur du fil.
    colour      INTEGER,
    -- Carte publiée. Permet de la modifier sur place plutôt que de réécrire le
    -- fil, et de résoudre une réaction en message sans appel API.
    message_id  INTEGER,
    -- Ordre d'affichage dans le fil. La hiérarchie des rôles ne peut pas servir
    -- de tri : un message d'information n'a pas de rôle.
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE UNIQUE INDEX idx_posts_slug ON posts (category_id, slug);
CREATE UNIQUE INDEX idx_posts_message ON posts (message_id) WHERE message_id IS NOT NULL;
-- Un rôle ne peut être le rôle principal que d'un seul message : sans cette
-- garantie, la réévaluation du rôle partagé n'aurait pas de réponse unique.
CREATE UNIQUE INDEX idx_posts_role ON posts (role_id) WHERE role_id IS NOT NULL;
CREATE INDEX idx_posts_order ON posts (category_id, position);

-- Message privé déjà envoyé à un membre pour un fil donné.
-- La ligne n'expire jamais : le CDC demande un envoi unique « peu importe le
-- délai entre les réactions ».
CREATE TABLE dm_sent (
    member_id   INTEGER NOT NULL,
    category_id INTEGER NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    sent_at     INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (member_id, category_id)
);

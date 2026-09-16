-- Un message peut désormais porter son propre message privé, et se soustraire
-- au rôle parent de son fil.
--
-- Le besoin : la carte PauseAction vit dans le fil Projets, dont toute carte
-- accorde @portail-équipe, le rôle de celles et ceux qui s'engagent chaque
-- semaine. PauseAction est l'inverse — cinq minutes quand on peut. Elle doit
-- donc porter une main levée, n'accorder aucun rôle, et envoyer le lien du
-- groupe WhatsApp en privé plutôt que de l'afficher sur le forum.

-- 1. Le message privé de la carte, qui prime sur celui du fil quand il existe.
ALTER TABLE posts ADD COLUMN dm_text TEXT;

-- 2. L'exception au rôle parent. Par défaut une carte l'accorde toujours :
--    c'est la règle du fil, et l'équipe a écarté un réglage par message pour
--    les rôles nominatifs. Celle-ci ne choisit pas un rôle, elle renonce au
--    seul qu'elle recevrait.
ALTER TABLE posts ADD COLUMN grants_parent INTEGER NOT NULL DEFAULT 1;

-- 3. La réservation du message privé d'une carte. `dm_sent` réserve par fil ;
--    une carte qui a son propre texte a besoin de sa propre réservation, sans
--    quoi le message du fil et celui de la carte se voleraient mutuellement
--    leur unique envoi. Même atomicité que `dm_sent` (invariant #5).
CREATE TABLE dm_post_sent (
    member_id  INTEGER NOT NULL,
    post_id    INTEGER NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (member_id, post_id)
);

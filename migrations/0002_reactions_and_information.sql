-- Deux ajouts pour que les messages sans rôle nominatif fonctionnent.
--
-- 1. `information` : un marqueur explicite de message d'information. Un tel
--    message ne porte jamais de réaction et n'accorde rien, même dans un fil
--    doté d'un rôle parent. Sans ce marqueur, « pas de rôle » deviendrait
--    ambigu : un message d'un fil à rôle parent doit désormais accorder ce
--    parent, alors qu'un vrai message d'information ne doit rien accorder.
ALTER TABLE posts ADD COLUMN information INTEGER NOT NULL DEFAULT 0;

-- 2. `reactions` : le suivi des mains levées, par membre et par message.
--    Quand un message n'a pas de rôle nominatif, on ne peut plus déduire des
--    rôles portés s'il reste une autre réaction du fil justifiant le rôle
--    parent. Cette table conserve donc les réactions posées sur les messages
--    sans rôle, afin de ne reprendre le rôle parent que lorsque plus aucune
--    d'entre elles ne subsiste dans le fil.
CREATE TABLE reactions (
    member_id  INTEGER NOT NULL,
    post_id    INTEGER NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (member_id, post_id)
);

-- Retrouver rapidement toutes les réactions d'un membre.
CREATE INDEX idx_reactions_member ON reactions (member_id);

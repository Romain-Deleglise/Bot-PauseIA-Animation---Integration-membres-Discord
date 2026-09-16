-- Prévenir quelqu'un quand une main se lève.
--
-- Jusqu'ici le bot accordait le rôle, envoyait le message privé, et s'arrêtait
-- là. Le message annonçait pourtant qu'un·e référent·e prendrait contact : rien
-- ne le ou la prévenait. La seule trace était la liste des réactions sur la
-- carte, que personne ne consulte.
--
-- Deux réglages, à deux niveaux différents :
--
-- 1. Le salon où le fil annonce ses mouvements. Par fil, car tous n'ont pas le
--    même public : une main levée sur une équipe intéresse le bureau, dix
--    compétences cochées d'affilée n'intéressent personne.
ALTER TABLE categories ADD COLUMN notify_channel_id INTEGER;

-- 2. La personne à mentionner dans cette annonce, propre à chaque carte : c'est
--    le ou la référente du projet, pas celle du fil.
ALTER TABLE posts ADD COLUMN referent_id INTEGER;

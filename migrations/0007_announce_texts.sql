-- Le texte de l'annonce publiée dans le salon d'un projet était écrit dans le
-- code. NULL = celui par défaut du bot, ce qui laisse les fils inchangés.
ALTER TABLE categories ADD COLUMN announce_join_text TEXT;
ALTER TABLE categories ADD COLUMN announce_leave_text TEXT;

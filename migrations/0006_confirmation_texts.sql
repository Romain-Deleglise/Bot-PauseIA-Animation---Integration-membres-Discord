-- Le texte des confirmations d'entrée et de sortie vivait dans le code : le
-- corriger demandait un déploiement. NULL = le texte par défaut du bot, ce qui
-- laisse les fils existants inchangés.
ALTER TABLE categories ADD COLUMN joined_text TEXT;
ALTER TABLE categories ADD COLUMN left_text TEXT;

#!/bin/sh
# Boucle de sauvegarde exécutée par le conteneur `backup` du docker-compose.
#
# Un simple conteneur alpine qui monte le même volume `./data` que le bot et en
# prend un instantané cohérent à intervalle régulier, puis fait tourner les
# sauvegardes. Rien à programmer sur l'hôte : `docker compose up -d` suffit, et
# le conteneur repart tout seul au redémarrage du serveur.
#
# Réglable par variables d'environnement (voir docker-compose.yml) :
#   INTERVAL   secondes entre deux sauvegardes (défaut 86400 = 1 jour)
#   KEEP_DAYS  jours de rétention avant purge (défaut 14)
#   DB         chemin de la base dans le conteneur (défaut /data/bot.db)
#   DEST       dossier des sauvegardes (défaut /backups)

set -eu

INTERVAL="${INTERVAL:-86400}"
KEEP_DAYS="${KEEP_DAYS:-14}"
DB="${DB:-/data/bot.db}"
DEST="${DEST:-/backups}"

# sqlite3 n'est pas dans l'image de base ; on l'installe une fois au démarrage.
if ! command -v sqlite3 >/dev/null 2>&1; then
    apk add --no-cache sqlite >/dev/null 2>&1 || true
fi

mkdir -p "$DEST"
echo "$(date -Iseconds) sauvegarde toutes les ${INTERVAL}s, rétention ${KEEP_DAYS} jours"

while true; do
    if [ -f "$DB" ]; then
        ts="$(date +%Y%m%d_%H%M%S)"
        out="$DEST/bot-$ts.db"
        # .backup gère le mode WAL : instantané cohérent, sans arrêter le bot.
        if sqlite3 "$DB" ".backup '$out'"; then
            echo "$(date -Iseconds) sauvegarde créée : $out"
        else
            echo "$(date -Iseconds) échec de la sauvegarde" >&2
        fi
        # Purge des sauvegardes trop anciennes.
        find "$DEST" -maxdepth 1 -name 'bot-*.db' -type f -mtime "+$KEEP_DAYS" -delete 2>/dev/null || true
    else
        echo "$(date -Iseconds) base absente ($DB), on réessaie plus tard" >&2
    fi
    sleep "$INTERVAL"
done

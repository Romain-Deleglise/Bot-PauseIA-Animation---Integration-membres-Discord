#!/usr/bin/env bash
# Sauvegarde cohérente de la base du bot.
#
# La base est en mode WAL : une simple copie du fichier pendant que le bot tourne
# peut être incohérente. `sqlite3 .backup` prend un instantané propre à chaud,
# sans arrêter le bot. Le script fait tourner les sauvegardes et supprime les
# plus anciennes.
#
# Prérequis : sqlite3 sur l'hôte (`sudo apt install -y sqlite3`).
#
# Usage :
#   deploy/backup-db.sh                      # chemins par défaut
#   DB=/chemin/bot.db DEST=/backups deploy/backup-db.sh
#
# Cron quotidien (3 h du matin), à ajouter avec `crontab -e` :
#   0 3 * * * /opt/volunteer-apps/apps/bot-roles/deploy/backup-db.sh >> /var/log/bot-roles-backup.log 2>&1

set -euo pipefail

# Racine du déploiement = dossier parent de ce script.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

DB="${DB:-$ROOT/data/bot.db}"
DEST="${DEST:-$ROOT/backups}"
KEEP_DAYS="${KEEP_DAYS:-14}"

if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "sqlite3 est requis : sudo apt install -y sqlite3" >&2
    exit 1
fi
if [ ! -f "$DB" ]; then
    echo "base introuvable : $DB" >&2
    exit 1
fi

mkdir -p "$DEST"
stamp="$(date +%Y%m%d_%H%M%S)"
out="$DEST/bot-$stamp.db"

# .backup gère le WAL et produit un fichier autonome et cohérent.
sqlite3 "$DB" ".backup '$out'"
echo "$(date -Iseconds) sauvegarde créée : $out ($(du -h "$out" | cut -f1))"

# Purge des sauvegardes plus vieilles que KEEP_DAYS jours.
find "$DEST" -maxdepth 1 -name 'bot-*.db' -type f -mtime "+$KEEP_DAYS" -print -delete \
    | sed 's/^/  supprimée : /' || true

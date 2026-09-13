# deploy/ — helpers d'exploitation

Scripts d'exploitation propres à ce déploiement (GitHub + Hetzner). Ils ne font
pas partie du bot ; ils l'entourent.

## `reset-forum.py`

Supprime les **rôles périmés** laissés par un import au contenu différent, que
le bot ne détruit jamais lui-même (par sécurité). Peut aussi purger les cartes
des fils pour repartir de zéro.

Lit `DISCORD_TOKEN` et `DISCORD_GUILD_ID` dans le `.env` voisin. Aucune
dépendance : bibliothèque standard Python 3 uniquement.

```bash
cd /opt/volunteer-apps/apps/bot-roles

# Supprime seulement les rôles périmés listés dans le script (sans toucher aux cartes) :
python3 deploy/reset-forum.py

# Remise à zéro complète (purge des cartes des 4 fils EN PLUS des rôles) :
python3 deploy/reset-forum.py --purge
```

> ⚠️ Ne lance **jamais** `--purge` après un `/forum importer` réussi : il
> effacerait les cartes fraîchement publiées. La purge ne sert qu'à repartir
> d'un forum vide.

Les identifiants des fils et la liste des rôles à supprimer sont en tête du
script, à ajuster si le contenu évolue.

## `backup-db.sh`

Sauvegarde cohérente de `data/bot.db` (mode WAL) via `sqlite3 .backup`, à chaud,
sans arrêter le bot. Fait tourner les sauvegardes et supprime celles de plus de
`KEEP_DAYS` jours (14 par défaut).

Prérequis : `sudo apt install -y sqlite3`.

```bash
cd /opt/volunteer-apps/apps/bot-roles
deploy/backup-db.sh                       # data/bot.db -> backups/
DB=/chemin/bot.db DEST=/backups KEEP_DAYS=30 deploy/backup-db.sh
```

Cron quotidien (3 h), via `crontab -e` :

```cron
0 3 * * * /opt/volunteer-apps/apps/bot-roles/deploy/backup-db.sh >> /var/log/bot-roles-backup.log 2>&1
```

> Les sauvegardes vivent sur le même serveur : pour se prémunir d'une perte du
> serveur lui-même, copiez régulièrement le dossier `backups/` ailleurs.

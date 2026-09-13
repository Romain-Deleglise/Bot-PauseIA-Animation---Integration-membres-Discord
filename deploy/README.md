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

## Sauvegardes automatiques (conteneur `backup`)

**C'est le mécanisme en place.** Le `docker-compose.yml` inclut un service
`backup` : un petit conteneur qui monte le même volume `./data` que le bot et
prend un instantané cohérent de la base (`sqlite3 .backup`, compatible WAL, à
chaud) à intervalle régulier, dans `./backups/`, avec rotation.

Rien à programmer sur l'hôte : `docker compose up -d` lance aussi les
sauvegardes, et elles repartent toutes seules au redémarrage du serveur. La
boucle est dans [`backup-service.sh`](backup-service.sh) ; l'intervalle
(`INTERVAL`, en secondes) et la rétention (`KEEP_DAYS`, en jours) se règlent dans
la section `backup` du `docker-compose.yml` (défaut : 1 jour, 14 jours de
rétention).

Vérifier que ça tourne :

```bash
docker compose logs backup            # doit afficher « sauvegarde créée : … »
ls -lh ./backups/                     # les instantanés datés
```

> Les sauvegardes vivent sur le **même serveur** : elles protègent d'une
> corruption ou d'une suppression, pas d'une perte du serveur lui-même. Pour
> couvrir ce cas, copiez régulièrement `backups/` ailleurs.

## `backup-db.sh` (sauvegarde manuelle ponctuelle)

Même sauvegarde, mais lancée à la main sur l'hôte (utile pour un instantané
avant une opération risquée). Prérequis : `sudo apt install -y sqlite3`.

```bash
cd /opt/volunteer-apps/apps/bot-roles
deploy/backup-db.sh                       # data/bot.db -> backups/
DB=/chemin/bot.db DEST=/backups KEEP_DAYS=30 deploy/backup-db.sh
```

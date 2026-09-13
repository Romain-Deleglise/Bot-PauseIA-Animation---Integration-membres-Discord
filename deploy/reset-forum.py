#!/usr/bin/env python3
"""Remise à zéro du forum pour un ré-import propre.

Helper d'exploitation (pas partie du bot lui-même). Il corrige la situation
laissée par un premier import au contenu différent : il purge les cartes des
fils et supprime les rôles périmés que le bot, par sécurité, ne détruit jamais
lui-même.

Utilise l'API Discord avec le token du bot lu dans le .env voisin. Aucune
dépendance externe : uniquement la bibliothèque standard.

Séquence complète recommandée (voir le README de ce dossier) :
    docker compose down
    rm -f ./data/bot.db ./data/bot.db-wal ./data/bot.db-shm   # le bot oublie tout
    python3 deploy/reset-forum.py                              # purge fils + rôles périmés
    docker compose up -d
    # puis, dans Discord : /forum importer  (avec le nouveau forum.toml)
"""

import json
import os
import sys
import time
import urllib.error
import urllib.request

API = "https://discord.com/api/v10"

# Les quatre fils du forum, dans l'ordre : Projets, Équipes, Groupes locaux, Compétences.
CHANNELS = [
    "1548365905870721084",
    "1548365691038732368",
    "1548366084967760004",
    "1548365209050161284",
]

# Rôles créés par le premier import et absents du contenu à jour, à supprimer.
# Ajuste cette liste si besoin ; un nom absent du serveur est simplement ignoré.
STALE_ROLES = [
    "accueil-orientation",
    "partenariats-créateurs",
    "creation-artistique",
    "flyers",
]


def read_env(key: str) -> str:
    """Lit une variable dans le .env voisin, ou dans l'environnement."""
    if key in os.environ and os.environ[key].strip():
        return os.environ[key].strip()
    here = os.path.dirname(os.path.abspath(__file__))
    env_path = os.path.join(here, "..", ".env")
    try:
        with open(env_path, encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if line.startswith(f"{key}="):
                    return line.split("=", 1)[1].strip()
    except FileNotFoundError:
        pass
    sys.exit(f"{key} introuvable (ni dans l'environnement, ni dans ../.env)")


TOKEN = read_env("DISCORD_TOKEN")
GUILD = read_env("DISCORD_GUILD_ID")
HEADERS = {
    "Authorization": f"Bot {TOKEN}",
    "Content-Type": "application/json",
    "User-Agent": "bot-roles-reset/1.0",
}


def request(method: str, path: str, body=None):
    """Appel API avec respect basique du rate limit (retente sur 429)."""
    data = json.dumps(body).encode() if body is not None else None
    while True:
        req = urllib.request.Request(f"{API}{path}", data=data, headers=HEADERS, method=method)
        try:
            with urllib.request.urlopen(req) as resp:
                raw = resp.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as err:
            if err.code == 429:
                retry = 1.0
                try:
                    retry = float(json.loads(err.read()).get("retry_after", 1.0))
                except Exception:
                    pass
                time.sleep(retry + 0.2)
                continue
            if err.code == 404:
                return None  # déjà supprimé / introuvable : pas une erreur ici
            raise


def purge_channel(channel_id: str) -> int:
    """Supprime toutes les cartes du fil, en gardant son message d'amorce."""
    deleted = 0
    while True:
        messages = request("GET", f"/channels/{channel_id}/messages?limit=100") or []
        # Le message d'amorce porte le même identifiant que le fil : indestructible.
        ids = [m["id"] for m in messages if m["id"] != channel_id]
        if not ids:
            return deleted
        if len(ids) >= 2:
            request("POST", f"/channels/{channel_id}/messages/bulk-delete", {"messages": ids})
            deleted += len(ids)
        else:
            request("DELETE", f"/channels/{channel_id}/messages/{ids[0]}")
            deleted += 1
        time.sleep(0.5)


def delete_stale_roles() -> None:
    roles = request("GET", f"/guilds/{GUILD}/roles") or []
    by_name = {r["name"].lower(): r["id"] for r in roles}
    for name in STALE_ROLES:
        rid = by_name.get(name.lower())
        if rid:
            request("DELETE", f"/guilds/{GUILD}/roles/{rid}")
            print(f"  rôle supprimé : {name} ({rid})")
            time.sleep(0.5)
        else:
            print(f"  rôle absent, ignoré : {name}")


def main() -> None:
    # La purge des cartes n'est utile que pour repartir de zéro sans être passé
    # par `/forum fil supprimer`. Elle est donc explicite : la lancer par erreur
    # après un import effacerait les cartes fraîchement publiées.
    purge = "--purge" in sys.argv[1:]

    if purge:
        print("== Purge des cartes des fils ==")
        for channel_id in CHANNELS:
            count = purge_channel(channel_id)
            print(f"  fil {channel_id} : {count} message·s supprimé·s")

    print("== Suppression des rôles périmés ==")
    delete_stale_roles()

    print("\nTerminé.")
    if not purge:
        print("(Aucune carte touchée. Pour aussi purger les fils, relance avec --purge.)")


if __name__ == "__main__":
    main()

# Passation du Bot Rôles

> Document généré par Claude à partir de l'historique du projet, relu par Antoine. État au 12 septembre 2026, version 3.4.0.

## Le projet

Bot Discord qui fait vivre le forum PauseIA décrit dans [cdc.md](cdc.md) : 4 fils, 38 cartes, 40 rôles. Lever la main 🙋 sur une carte donne son rôle, plus le rôle parent du fil. Rust, SQLite, image Docker de 10 Mo sans shell.

**État** : fonctionnel, recetté sur un serveur de test, **pas encore en production**. Tout ce qui est décrit dans le CDC est couvert, avec les ajustements décidés en réunion et en démo (plus bas).

## Où lire quoi

| Document | Pour qui | Contenu |
|---|---|---|
| [README.md](README.md) | exploitation | fonctionnement, déploiement, configuration, sauvegarde |
| [docs/guide.md](docs/guide.md) | administrateurs Discord | créer le bot avec les droits minimaux, commandes |
| [MISE-EN-PLACE.md](MISE-EN-PLACE.md) | équipe PauseIA | décisions prises, préparation du serveur, format du fichier de contenu |
| [CLAUDE.md](CLAUDE.md) | développeurs | architecture, invariants à respecter, pièges Discord |
| [docs/questions-ouvertes.md](docs/questions-ouvertes.md) | décideurs | ce qui reste à trancher |
| [docs/demo.md](docs/demo.md) | archive | déroulé de la démo du 12 septembre |
| [cdc.md](cdc.md) | tous | cahier des charges d'origine |

## Ce qui dépend de l'infrastructure actuelle

À reprendre ou remplacer. Aucun secret n'est versionné : les `.env` sont ignorés par Git.

| Élément | Aujourd'hui | À faire |
|---|---|---|
| Dépôt et CI | GitLab `git.kher.nl` | La CI n'utilise que les variables standard de GitLab (`CI_REGISTRY*`) : elle fonctionne telle quelle sur tout GitLab avec registry et runner Docker-in-Docker. |
| Images | `registry.kher.nl/pause-ia/bot-roles` | Remplacer ce nom dans `docker-compose.yml`, `.env.example` et `deploy/vps/docker-compose.yml`. |
| Éditeur web | GitLab Pages, job `pages` | Automatique sur la branche principale. La page démarre vide et ne publie jamais le contenu. |
| Déploiement | `deploy/vps/` : compose propre au VPS d'Antoine, avec une étiquette Traefik | Le `docker-compose.yml` racine est la version générique. |
| Bot et serveur de test | Application Discord et serveur sandbox d'Antoine | Non transmis. Créez les vôtres avec [docs/guide.md](docs/guide.md). |
| `data/` | Base et TOML de la sandbox, en local | Non versionné, rien à récupérer. |

## Mettre en production

1. Créer l'application Discord et inviter le bot : [docs/guide.md](docs/guide.md) §1. Glisser son rôle **tout en haut** de la liste des rôles.
2. Créer à la main le salon forum et ses 4 fils, relever leurs identifiants.
3. Dans `contenu/forum.toml` : reporter les identifiants dans `salon`, retirer `nom` et `description` si les fils ont déjà leur nom et leur message d'ouverture, écrire les référents avec leur **nom d'utilisateur** Discord.
4. Remplir le `.env` (`DISCORD_TOKEN`, `DISCORD_GUILD_ID`, `MANAGE_ROLE_IDS`), puis `docker compose up -d`.
5. `/forum importer`, puis recette : une réaction par fil, en vérifiant les deux rôles d'un groupe local et d'une équipe.
6. Passer `ENABLE_IMPORT=false` et redémarrer : la commande d'import disparaît.

## Décisions prises pendant la démo

- `/forum message supprimer` n'efface que la carte : le rôle reste sur le serveur et ses porteurs le gardent.
- Les `@pseudo` des textes deviennent des mentions cliquables.
- `nom` et `description` sont facultatifs dans le TOML, pour des fils créés à la main.
- Les listes longues sont découpées en plusieurs messages.

## Limites connues

- **Réaction orpheline** : un rôle retiré à la main laisse la 🙋 sur la carte (question ouverte).
- **Message privé** : une seule fois par personne et par fil, pas par ville (question ouverte).
- **Pseudos** : reconnus sur le nom d'utilisateur, pas sur le pseudo affiché sur le serveur.
- **Carte supprimée** : son rôle, resté sur le serveur, ne justifie plus le rôle parent. Un membre qui le garde peut perdre `@groupe-local` en quittant un autre groupe.
- **`/forum republier`** remet toutes les réactions à zéro. C'est le seul moyen de réordonner les cartes.
- **Couleur d'un fil** : répercutée sur les cartes depuis la 3.4.0. Ce correctif n'a pas encore été essayé sur le serveur de test.
- **Tests** : 86 tests automatiques, mais aucun ne parle à l'API Discord. Toute modification des commandes ou des réactions demande une recette manuelle sur un serveur de test.

## Faire évoluer

Lire [CLAUDE.md](CLAUDE.md) avant de toucher au code. Vérifier avec `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` et `cargo test`, exactement ce que fait la CI. Publier une version : changer `version` dans `Cargo.toml`, puis `git tag X.Y.Z && git push origin X.Y.Z` ; la CI construit et publie l'image.

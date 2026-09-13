# Bot Rôles

Bot Discord qui fait vivre le **forum PauseIA** : un salon forum avec quatre
fils — Projets, Équipes, Groupes locaux, Compétences — dans lesquels chaque
message présente une activité. Lever la main 🙋 sur un message donne les rôles
associés, et peut déclencher un message privé d'accueil.

Écrit en Rust, sans dépendance externe : une base SQLite dans un fichier, et une
image Docker qui ne contient qu'un binaire statique — ni interpréteur, ni shell,
ni bibliothèque partagée.

**Vous reprenez le projet ?** Commencez par [PASSATION.md](PASSATION.md).

## Sommaire

- [Ce que fait le bot](#ce-que-fait-le-bot)
- [Prérequis Discord](#prérequis-discord)
- [Déploiement](#déploiement)
- [Configuration](#configuration)
- [Amorçage du forum](#amorçage-du-forum)
- [Le fichier de contenu](#le-fichier-de-contenu)
- [Commandes](#commandes)
- [Exploitation](#exploitation)
- [Développement](#développement)

## Ce que fait le bot

**Des cartes qui donnent des rôles.** Chaque fil du forum contient des messages
publiés par le bot : un titre, un texte, une couleur. Lever la main sur une
carte accorde le rôle qu'elle représente ; retirer sa réaction le reprend.

**Un rôle parent par fil.** Un fil peut déclarer un rôle accordé par tous ses
messages : rejoindre Paris donne `@gl-paris` **et** `@groupe-local`. Ce rôle
parent n'est repris que lorsque plus aucun message du fil ne le justifie —
quitter Paris en restant à Lyon ne fait rien perdre.

**Des messages sans rôle.** Un message peut n'accorder aucun rôle : il ne porte
alors aucune réaction et sert de simple information.

**Un message privé d'accueil.** Un fil peut envoyer un message privé à la
première réaction d'un membre. Il n'est envoyé qu'une fois par membre et par
fil, quel que soit le délai entre les réactions.

Rendu d'un fil : [docs/maquette-fil.png](docs/maquette-fil.png).

## Prérequis Discord

Dans le [portail développeur](https://discord.com/developers/applications), sur
votre application :

- onglet **Bot** : activer l'intent privilégié **SERVER MEMBERS INTENT**. Le bot n'écoute pas les arrivées ni les départs, mais sans cet intent il ne voit pas les changements de rôles des membres, et déciderait du rôle parent sur des rôles périmés ;
- onglet **OAuth2** : inviter le bot avec les scopes `bot` et `applications.commands`, et exactement les 8 permissions listées dans [docs/guide.md](docs/guide.md) (entier `275146435648`), vérifiées une par une sur un serveur de test.

> **Le point qui coince le plus souvent.** Dans *Paramètres du serveur >
> Rôles*, le rôle du bot doit être **au-dessus** de tous les rôles qu'il gère.
> Discord interdit à quiconque de toucher à un rôle situé plus haut que le
> sien : sans cela, chaque commande échouera avec une erreur de permissions,
> alors même que la permission « Gérer les rôles » est bien accordée.

## Déploiement

Sur le VPS, avec Docker seul :

```bash
git clone <ce-dépôt> bot-roles && cd bot-roles
cp .env.example .env && $EDITOR .env

# Le conteneur tourne en utilisateur 65534 : le répertoire de données doit
# lui appartenir, sinon le bot ne pourra pas créer sa base.
mkdir -p ./data && sudo chown -R 65534:65534 ./data

docker compose pull
docker compose up -d
docker compose logs -f
```

Au démarrage, le bot applique ses migrations puis enregistre ses commandes sur
le serveur — la propagation est immédiate.

### Mettre à jour

```bash
docker compose pull && docker compose up -d
```

Les images sont publiées par la CI à chaque tag Git, sous `<registry>:X.Y.Z` et
`:latest`. Le nom complet est affiché à la fin du job `build` ; reportez-le dans
`docker-compose.yml`, ou passez-le par `BOT_IMAGE` dans le `.env`. Pour revenir
en arrière, remplacez `:latest` par la version voulue.

### Publier une version

```bash
git tag 3.0.0 && git push origin 3.0.0
```

Le pipeline vérifie le formatage, la qualité et les tests, puis construit et
publie l'image. Un push sur une branche ne publie rien.

## Configuration

Sept variables, toutes décrites dans [`.env.example`](.env.example). Le bot les
valide au démarrage et signale **toutes** celles qui manquent d'un coup, plutôt
que de s'arrêter à la première.

| Variable | Rôle |
|---|---|
| `DISCORD_TOKEN` | Jeton du bot |
| `DISCORD_GUILD_ID` | Serveur géré. Une instance = un serveur |
| `MANAGE_ROLE_IDS` | Rôles autorisés à utiliser les commandes, séparés par des virgules |
| `REACTION_EMOJI` | Emoji des cartes : `🙋` ou `<:nom:123>` |
| `DATABASE_PATH` | Fichier de la base. Défaut `/data/bot.db` |
| `LOG_LEVEL` | `error`, `warn`, `info`, `debug`, `trace`. Défaut `info` |
| `ENABLE_IMPORT` | Propose `/forum importer`. À passer à `false` une fois le contenu importé. Défaut `true` |

## Amorçage du forum

**1. Créer le salon forum et ses quatre fils** dans Discord, à la main. Le bot
publie dans des fils existants, il n'en crée pas.

**2. Relever les identifiants des fils** : clic droit sur chaque fil > Copier
l'identifiant, avec le mode développeur activé.

**3. Reporter ces identifiants** dans [`contenu/forum.toml`](contenu/forum.toml),
à la place des valeurs factices `111111111111111111` et suivantes. Ce fichier
contient déjà les 38 messages du cahier des charges ; son en-tête liste ce qui
reste à compléter.

**4. Importer**, en glissant le fichier dans la commande :

```
/forum importer fichier:[forum.toml]
```

Le bot vérifie d'abord que les quatre fils existent et sont accessibles — si
l'un manque, l'import s'arrête sans avoir rien écrit. Puis il crée les 40 rôles
absents, publie les cartes et rend un compte rendu chiffré.

**5. Vérifier** en levant la main sur un message de chaque fil, comme le
demande le cahier des charges, et en contrôlant les rôles obtenus.

## Le fichier de contenu

Le contenu du forum est décrit dans un fichier TOML versionné, qui se relit et
se corrige comme un texte. Un éditeur web accompagne le dépôt — publié sur
GitLab Pages par le job `pages`, source dans [`web/index.html`](web/index.html) —
qui en montre l'aperçu carte par carte, signale ce qui manque et le réexporte.

**Cet éditeur démarre vide et le fichier de contenu n'est jamais publié avec
lui** : celui-ci nomme les référents de chaque projet et les villes de chaque
groupe local, et une page consultable n'est pas un endroit pour ça. On y importe
son fichier depuis son poste ; tout se passe dans le navigateur, rien n'est
envoyé nulle part. L'import est **rejouable** : il crée ce qui manque,
met à jour ce qui a changé, et **ne supprime jamais rien**. Les messages
présents en base mais absents du fichier sont signalés dans le compte rendu, à
retirer explicitement — une suppression silencieuse reprendrait des rôles à des
membres.

```toml
[[fils]]
nom          = "Groupes locaux"   # facultatif : absent, le nom du fil sur Discord est repris
salon        = "1234567890123456789"
couleur      = "#F2994A"
role_parent  = "groupe-local"   # accordé par tous les messages du fil
description  = "PauseIA près de chez vous…"
mp           = "Bienvenue dans votre groupe local !"

[[fils.messages]]
slug  = "paris"        # identité stable, ne pas changer après un import
titre = "Paris"
role  = "gl-paris"     # créé s'il n'existe pas ; absent = message d'information
couleur = "#99AAB5"    # facultatif, prime sur celle du fil : grise la carte
texte = """
Référent : @pseudo_discord
Un groupe très actif : tractage, manifestations…"""
```

Un `@pseudo` dans un texte devient une mention cliquable si la personne est membre du serveur ; les pseudos introuvables restent en texte et sont signalés dans le compte rendu. Pour un fil créé à la main, `description` s'omet : son message d'ouverture sert d'introduction.

Éditer une carte depuis le fichier ou depuis Discord revient au même : dans les
deux cas le message est modifié sur place, et les réactions comme les rôles déjà
accordés survivent.

Passé l'amorçage, **c'est Discord qui fait foi**. Le fichier reste utile comme
trace lisible et comme filet de secours.

## Commandes

Toutes les commandes sont réservées aux rôles listés dans `MANAGE_ROLE_IDS` et
répondent en message éphémère, visible du seul appelant.

### `/forum message` — les cartes

| Commande | Effet |
|---|---|
| `créer` | Ajoute un message à un fil et publie sa carte |
| `modifier` | Change titre, texte, rôle, couleur, fil ou rang. Édite la carte sur place |
| `éditer` | Ouvre une fenêtre (modal) pré-remplie pour corriger titre et texte en multiligne, sans passer par des `\n` |
| `supprimer` | Supprime la carte. Le rôle reste sur le serveur, ses porteurs le gardent |
| `liste` | Liste les messages d'un fil, ou de tout le forum |

Le message se choisit par autocomplétion, sous la forme `Fil › Titre`.

### `/forum fil` — les fils

| Commande | Effet |
|---|---|
| `créer` | Rattache au bot un fil Discord existant, avec ses réglages |
| `modifier` | Change nom, salon, couleur, illustration, introduction, rôle parent |
| `mp` | Configure le message privé d'accueil, ou l'affiche |
| `supprimer` | Retire le fil du bot et efface ses cartes. Les rôles restent |
| `liste` | Affiche les fils et leur configuration |

Sur `modifier`, un paramètre omis reste inchangé et la valeur `-` efface le
champ. `mp` accepte un texte, ou l'identifiant d'un message à recopier comme
modèle — pratique pour un texte long, rédigé tranquillement dans un salon
d'administration.

### Contenu et ordre

| Commande | Effet |
|---|---|
| `/forum importer` | Importe le fichier de contenu, en pièce jointe |
| `/forum republier` | Réécrit un fil, ou tous, pour rétablir l'ordre |

## Exploitation

**Republier fait perdre les réactions.** `/forum republier` republie toutes les
cartes : les membres gardent leurs rôles, mais devront lever la main de nouveau
s'ils veulent pouvoir se les retirer. C'est le seul moyen de corriger l'ordre
d'affichage, l'ordre des messages Discord ne pouvant changer qu'en les
republiant. Toutes les autres commandes modifient la carte concernée sur place.

**Supprimer un message garde son rôle.** `/forum message supprimer` efface la carte et rien d'autre : le rôle reste sur le serveur et ses porteurs le gardent. Un rôle règle souvent l'accès à des salons, sa suppression se fait donc à la main dans Discord. La commande demande une confirmation explicite.

**Fils archivés.** Discord archive un fil après trois jours sans message, ce qui bloque les réactions. Le bot le rouvre aussitôt, ainsi qu'à son démarrage. Un fil verrouillé reste fermé.

**Sauvegarder.** Tout tient dans `./data`. Le bot arrêté, une copie du
répertoire suffit. À chaud, depuis l'hôte — l'image ne contient pas de shell :

```bash
sqlite3 ./data/bot.db ".backup './sauvegarde.db'"
```

**Diagnostiquer.** `LOG_LEVEL=debug` détaille les décisions prises sur chaque
réaction. `RUST_LOG` permet de cibler un module, par exemple
`RUST_LOG=bot_roles::events=debug`.

## Développement

```bash
cargo test           # tests unitaires, base SQLite en mémoire
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run            # lit .env à la racine
```

Prérequis : Rust 1.94 ou plus, et un compilateur C (`gcc` et `libc6-dev` sous
Debian) — SQLite et les primitives cryptographiques sont compilées depuis leurs
sources.

L'architecture, les invariants et les pièges à connaître sont documentés dans
[CLAUDE.md](CLAUDE.md).

# CLAUDE.md — notes pour agents

Contexte technique du dépôt. Le [README](README.md) couvre l'usage et
l'exploitation ; ce fichier couvre le code et ce qu'il faut savoir avant d'y
toucher.

## Le projet en deux phrases

Bot Discord qui publie et pilote un forum de rôles auto-assignables, décrit par
le cahier des charges [`cdc.md`](cdc.md). Une instance sert un serveur, l'état
tient dans un fichier SQLite, l'image finale est un binaire statique sur
`scratch`.

Il a d'abord été réécrit en Rust depuis une version Python qui dépendait d'une
base Directus distante, puis adapté au CDC — c'est cette seconde étape qui a
fait passer le modèle du rôle au message.

## Pile

| | |
|---|---|
| Langage | Rust 2024, MSRV 1.94 |
| Discord | `poise` 0.6 sur `serenity` 0.12 |
| Base | `sqlx` 0.9 + SQLite, requêtes en API dynamique |
| Contenu | `serde` + `toml` |
| Journalisation | `tracing` + `tracing-subscriber` |
| Erreurs | `anyhow` de bout en bout, `state::Error` en est l'alias |

**Pas de macros `sqlx::query!`.** Elles exigeraient un `DATABASE_URL` à la
compilation ou un répertoire `.sqlx` à maintenir. On utilise `query_as` avec
`#[derive(FromRow)]`. En contrepartie, sqlx 0.9 refuse les chaînes SQL
construites dynamiquement (`SqlSafeStr`) : **le SQL doit être un littéral**, et
la liste des colonnes est écrite en toutes lettres dans chaque requête.

## Carte du code

```
src/
  main.rs          amorçage, intents, framework poise, arrêt sur SIGTERM
  config.rs        lecture et validation de l'environnement
  state.rs         Data : pool + configuration + caches
  rules.rs         règles métier pures, sans Discord ni base
  emoji.rs         pont entre emoji saisi, emoji Gateway et emoji stocké
  ids.rs           conversions u64 (Discord) <-> i64 (SQLite)
  mentions.rs      @pseudo → <@identifiant>, d'après la liste des membres
  db/              une opération nommée par besoin métier, aucun SQL au-dessus
  discord/
    embed.rs       construction de la carte d'un message
    channel.rs     publication, modification, purge, réécriture d'un fil
  commands/
    threads.rs     /forum fil …
    posts.rs       /forum message …
    import.rs      /forum importer et /forum republier
  events/          aiguillage Gateway, réactions, réouverture des fils archivés
migrations/        SQL embarqué dans le binaire par sqlx::migrate!
contenu/forum.toml contenu du forum, transcrit du CDC
web/index.html     éditeur du fichier de contenu, publié sur GitLab Pages
deploy/vps/        compose du déploiement de référence
docs/              guide des commandes, questions ouvertes, déroulé de la démo
```

## Invariants

À vérifier avant toute modification. Les trois premiers ont été violés par la
version précédente.

### 1. Un message porte son identité, pas son rôle

`posts.title` est le titre affiché, indépendant du nom du rôle : « Fresque de
l'IA » n'est pas `@fresque-ia`.

Un message sans `role_id` **dans un fil à `parent_role_id`** accorde tout de même
ce rôle parent : la réaction 🙋 est posée et le donne. Un vrai message
d'information porte le marqueur explicite `posts.information` (colonne ajoutée par
la migration `0002`) : aucune réaction, il n'accorde rien, même dans un fil à
rôle parent. Un message sans rôle **et** sans parent reste informatif de fait.

Une carte peut aussi **renoncer** au rôle parent de son fil (`posts.grants_parent`,
migration `0003`) et porter son **propre** message privé (`posts.dm_text`). C'est
l'exception née de PauseAction : l'engagement le plus léger, dans un fil dont
toute carte accorde le rôle des bénévoles actifs. Elle porte la main levée,
n'accorde rien, et envoie en privé le lien qu'on ne veut pas afficher
publiquement. Ce renoncement ne choisit pas un rôle, il abandonne le seul que la
carte recevrait : la décision d'équipe qui interdit un réglage de rôle par
message tient toujours.

Conséquences : une carte est indexée si elle accorde un rôle **ou** si elle a son
propre message privé ; `state::PostRef.parent_role_id` vaut `None` quand la carte
y renonce, ce qui neutralise d'un coup l'attribution et la reprise ; et une main
levée n'est enregistrée dans `reactions` que si elle justifie réellement un rôle
parent, sans quoi PauseAction maintiendrait `@portail-équipe` sur le dos des
autres cartes du fil.
Conséquence : `state::PostRef.role_id` est un `Option`, et l'index du chemin
chaud (`post_by_message`) contient toute carte qui accorde quelque chose (rôle
nominatif **ou** parent), hors messages `information`.

Comme un message sans rôle ne laisse aucune trace dans les rôles portés, la
justification du rôle parent au retrait d'une réaction combine deux sources : un
autre rôle nominatif du fil encore porté (`rules::parent_still_justified`, pur),
**ou** une autre main levée du fil enregistrée dans la table `reactions`
(`db::reactions`, alimentée à chaque réaction sur un message sans rôle).

Le second rôle n'appartient pas au message : c'est `categories.parent_role_id`,
le même pour tous les messages du fil. L'équipe a explicitement écarté un
réglage par message, et les statuts avec.

`posts.slug` est l'identité stable pour le réimport : `role_id` peut être `NULL`
et le titre peut être corrigé, ni l'un ni l'autre ne peut servir de clé.

Une carte peut enfin nommer un·e **référent·e** (`posts.referent_id`) et un
**salon d'annonce** (`posts.notify_channel_id`, migration `0005`), son fil n'en
offrant qu'un de recours (`categories.notify_channel_id`) — chaque projet a son
salon, une équipe en a parfois plusieurs, et une annonce tombée dans un salon
commun ne serait lue par personne. Sans
eux, une main levée ne prévenait personne : le message privé annonçait qu'on
prendrait contact, et la seule trace était la liste des réactions, que personne
ne consulte.

### 2. Toute écriture est suivie de `Data::reload_caches`

Les caches de `state.rs` sont des données dérivées, rechargées en bloc plutôt
qu'invalidées champ par champ : impossible d'en oublier une. Le coût — quelques
requêtes — est payé sur un chemin froid, celui des commandes d'administration.

Corollaire : **rien n'est lu à l'initialisation d'un module**. La version Python
appelait `get_categories()` au moment de l'import, ce qui imposait un
redémarrage après chaque création de fil.

### 3. Le rôle parent ne se reprend que s'il n'est plus justifié

`rules::parent_still_justified` est le cœur du comportement, et il est pur
pour être testable sans serveur. Un membre inscrit à Paris et à Lyon porte
`@groupe-local` pour deux raisons : quitter Paris ne doit pas lui faire perdre
l'accès aux salons communs, quitter le dernier groupe, si.

Elle s'applique au retrait d'une réaction (`events::reactions::on_remove`). La suppression d'un message, elle, ne touche à aucun rôle : le rôle reste sur le serveur et ses porteurs le gardent. Décision prise à la démo, un rôle réglant souvent l'accès à des salons.

### 4. `posts.message_id` est la clé du chemin chaud

Une réaction se résout en message par une lecture de cache, sans appel réseau.
En échange, tout ce qui touche à une carte doit tenir cette colonne à jour :
`publish` l'écrit, `remove` la remet à `NULL`, `clear_message_ids` la vide avant
une réécriture complète.

C'est aussi ce qui permet de tenir l'exigence du CDC : « éditer les descriptions
des messages sans perdre le comportement ni supprimer des rôles aux
utilisateurs ». Toute opération qui republierait une carte au lieu de l'éditer
efface les réactions des membres — à n'employer que pour réordonner.

Deux index unique partiels gardent le modèle cohérent : un message ne désigne
jamais deux cartes, et un rôle n'est le rôle d'un seul message — sans quoi la
règle du rôle parent n'aurait pas de réponse unique.

### 5. Le message privé se réserve avant de s'envoyer

Serenity traite les réactions en parallèle : deux clics rapprochés liraient tous
deux une absence de ligne et enverraient chacun leur message. `db::dm::claim`
s'appuie donc sur l'atomicité d'un `INSERT OR IGNORE` et sur `rows_affected`.
Une carte qui a son propre message privé se réserve à part, dans `dm_post_sent` :
partager la ligne du fil ferait que le premier parti empêcherait l'autre.
En cas d'échec d'envoi, la réservation n'est libérée que si l'échec est
passager — réseau, limite de débit, panne de Discord. Un refus de Discord
lui-même (4xx : messages privés fermés, bot bloqué, compte supprimé) vaudra
encore demain : la réservation reste posée, sans quoi chaque réaction du fil
rejouerait le même appel perdu. Le rôle, lui, est accordé dans tous les cas, et
les liens du message privé figurent aussi dans la description du fil : une part
des membres ferme les messages privés de serveur, et rien ne permet de les
joindre autrement.

### 6. La clé d'un emoji ignore son nom

Discord n'inclut pas toujours le nom d'un emoji custom dans un événement de
réaction. `Emoji::key()` ne retient donc que l'identifiant (`custom:123`), et
`Emoji::matches` compare sur cette base.

Comparer deux `ReactionType` avec `==` échouerait par intermittence.

## Pièges Discord

- **Hiérarchie des rôles.** Le bot ne peut rien sur un rôle placé plus haut que
  le sien. C'est la première cause d'erreur 403, avant les permissions.
- **Rôles à position égale.** Des rôles créés en masse peuvent partager la même position ; Discord les départage alors par ancienneté, et le rôle du bot passe sous tous ceux créés avant lui. Symptôme : `Missing Permissions` sur les anciens rôles seulement. Remède : glisser le rôle du bot tout en haut dans les paramètres du serveur, ce qui réécrit toutes les positions.
- **`GUILD_MEMBERS` est requis, mais pas pour écouter les membres.** Sans lui, le cache serenity ne reçoit pas `GUILD_MEMBER_UPDATE` : `member_roles` lirait des rôles périmés au retrait d'une réaction, et le rôle parent serait décidé à tort.
- **Gardes de cache et `.await`.** Tenir un garde du cache serenity au travers
  d'un point d'attente bloque le cache entier. Clippy le surveille
  (`await_holding_lock`), ce n'est pas une raison pour s'en remettre à lui.
- **Identifiants en paramètre.** Un paramètre entier de commande slash est
  plafonné à 2^53 par Discord ; un snowflake le dépasse. Ils passent donc par
  une chaîne, lue avec `commands::parse_snowflake` — y compris dans le fichier
  TOML, où `salon` est une chaîne.
- **Suppression groupée.** Interdite au-delà de deux semaines et hors de la
  plage 1–100 messages. `channel::purge_channel` répartit selon l'âge, calculé
  depuis l'identifiant du message sans dépendre d'une bibliothèque de dates.
- **Message d'amorce d'un fil.** Son identifiant est celui du fil et il est
  indestructible : la purge l'ignore.
- **Un salon forum n'accepte pas de message.** Seuls ses fils en reçoivent, d'où
  le filtre de `commands::is_writable`.
- **Fenêtre de trois secondes.** Toute commande commence par `commands::begin`,
  qui diffère la réponse en éphémère.
- **Le contrôle d'accès tourne aussi pendant l'autocomplétion.** poise appelle
  `command_check` sur les requêtes d'autocomplétion comme sur les invocations,
  or `ctx.say` et `ctx.send` **paniquent** dans ce contexte. Toute réponse
  depuis un check doit donc être gardée par `commands::is_autocomplete`.

## Conventions

- **Commentaires** : expliquer le *pourquoi*, jamais le *quoi*. Un commentaire
  qui paraphrase la ligne suivante est du bruit.
- **Langue** : commentaires, documentation et messages destinés aux
  utilisateurs en français ; identifiants en anglais.
- **Noms de commandes** : écrits en dur dans les attributs `#[poise::command]`.
  Ils venaient de vingt variables d'environnement, dont une seule oubliée
  empêchait le démarrage.
- **Recherche par nom** : toujours via `db::categories::name_key`, qui replie en
  minuscules côté Rust. La collation `NOCASE` de SQLite ne replie que l'ASCII et
  considère « ÉQUIPES » et « Équipes » comme distincts.
- **Tests** : ils décrivent un comportement, pas une fonction. Préférer
  `another_group_still_justifies_it` à `test_rule_2`. La base de test est en
  mémoire, une connexion unique — au-delà, chaque connexion ouvrirait sa propre
  base.

## Ajouter une commande

1. L'écrire dans le fichier thématique de `commands/`, avec
   `#[poise::command(slash_command)]`.
2. L'ajouter à la liste `subcommands(…)` du groupe parent.
3. Commencer par `commands::begin(ctx).await?`.
4. Si elle écrit en base, terminer par `ctx.data().reload_caches().await?`.

Le contrôle d'accès est posé une fois pour toutes dans
`FrameworkOptions::command_check` : rien à ajouter par commande.

Discord n'autorise que deux niveaux d'imbrication — commande, groupe,
sous-commande — soit exactement `/forum message créer`. Pas plus.

## Faire évoluer le schéma

Ajouter un fichier `migrations/000N_description.sql`. Ne jamais modifier une
migration déjà déployée : sqlx enregistre la somme de contrôle de chaque
fichier appliqué et refuse de démarrer si elle a changé.

Les `PRAGMA` ne vont pas dans les migrations — SQLite refuse de changer le mode
de journal dans une transaction, et sqlx en ouvre une par migration. Ils sont
posés à l'ouverture de la connexion, dans `db::connect`.

## Le fichier de contenu

`contenu/forum.toml` sert à l'amorçage, pas de source de vérité continue : le
CDC demande de pouvoir éditer un message en quelques secondes
depuis Discord. L'import ne supprime donc jamais rien, il signale.

Il écrase en revanche : ce que le fichier décrit remplace ce que Discord
affiche. D'où `/forum exporter`, qui rend l'état réel dans le même format. Le
cycle sûr est exporter, comparer, fusionner, importer. Sans lui, corriger une
carte depuis Discord et réimporter plus tard efface la correction sans le dire.

Un test (`commands::import::tests::the_shipped_content_file_is_valid`) charge le
fichier livré et vérifie qu'il décrit bien quatre fils et 38 messages, chacun
doté d'un message privé d'accueil : une coquille dans le contenu échoue en CI,
pas en production.

L'introduction d'un fil, elle, n'appartient plus au bot. Dans un salon forum,
le premier message est celui de la personne qui a créé le fil : il est en tête,
indestructible, et se corrige d'un clic. Une `description` en aurait publié une
seconde en dessous, et la changer aurait coûté toutes les mains levées du fil,
faute de mémoriser l'identifiant de ce message.

## Vérifier

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

C'est exactement ce que fait l'étape `check` de la CI. Le build musl statique
peut se reproduire en local :

```bash
rustup target add x86_64-unknown-linux-musl
CC_x86_64_unknown_linux_musl=musl-gcc \
  cargo build --release --locked --target x86_64-unknown-linux-musl
```

Ce que les tests ne couvrent pas : tout ce qui parle à l'API Discord. Les
appels réseau ne sont pas simulés. Une modification de `discord/channel.rs`,
`commands/` ou `events/reactions.rs` doit être essayée sur un serveur de test —
le README décrit le parcours d'amorçage, et le CDC sa recette : « tester au
moins un message de chaque salon, et vérifier tous les rôles donnés ».

## Ce dont il ne reste rien

Directus, `requests`, `disnake`, les cogs et les quarante variables
d'environnement. La quarantaine d'accueil et les liaisons réaction-message,
héritées du serveur précédent, ont été retirées avec l'adaptation au CDC : elles
restent consultables dans l'historique Git.

# Bot Rôles — guide

> Généré par Claude, relu par Antoine. Permissions vérifiées une par une sur un serveur de test.

## 1. Créer le bot

1. Sur [discord.com/developers/applications](https://discord.com/developers/applications) : **New Application**.
2. Onglet **Bot** → **Reset Token**. Copiez le jeton : il n'est affiché qu'une fois.
3. Onglet **Bot** → *Privileged Gateway Intents* : activez **Server Members Intent**, puis enregistrez. Sans lui, le bot ne démarre pas.
4. Onglet **OAuth2 → URL Generator** : cochez les scopes `bot` et `applications.commands`, puis exactement ces 8 permissions :

| Permission | Sert à |
|---|---|
| View Channels | voir le forum et recevoir les réactions |
| Send Messages | rouvrir un fil archivé |
| Send Messages in Threads | publier les cartes dans les fils |
| Embed Links | afficher les cartes |
| Add Reactions | poser la 🙋 sous chaque carte |
| Read Message History | réagir et relire un fil |
| Manage Messages | vider un fil lors de `/forum republier` |
| Manage Roles | créer, donner et reprendre les rôles |

5. Ouvrez l'URL générée et invitez le bot sur votre serveur.
6. *Paramètres du serveur → Rôles* : glissez le rôle du bot **tout en haut**, même s'il semble déjà bien placé : à rang égal, Discord le classe sous les rôles plus anciens. Sinon, les attributions échouent.

Raccourci pour les étapes 4 et 5, en remplaçant l'identifiant par celui de votre application :

```
https://discord.com/oauth2/authorize?client_id=VOTRE_ID&scope=bot+applications.commands&permissions=275146435648
```

## 2. Valeurs à transmettre pour le `.env`

| Variable | Valeur |
|---|---|
| `DISCORD_TOKEN` | le jeton de l'étape 2 — **par un canal privé** |
| `DISCORD_GUILD_ID` | clic droit sur le serveur → Copier l'identifiant (mode développeur activé) |
| `MANAGE_ROLE_IDS` | identifiants des rôles autorisés à utiliser les commandes, séparés par des virgules |
| `REACTION_EMOJI` | facultatif, 🙋 par défaut |
| `ENABLE_IMPORT` | `true` pour le premier import, `false` ensuite : la commande disparaît |

## 3. Commandes

Réservées aux rôles de `MANAGE_ROLE_IDS`. Les réponses ne sont visibles que de
vous.

**Cartes**

| Commande | Effet |
|---|---|
| `/forum message créer` | Publie une carte : titre, texte, rôle, couleur, rang. Sans rôle, elle accorde quand même le rôle parent de son fil. |
| `/forum message modifier` | Modifie titre, texte, rôle, couleur, fil ou rang. La carte est éditée sur place : réactions et rôles conservés. |
| `/forum message éditer` | Ouvre une fenêtre pré-remplie avec le titre et le texte, éditables en multiligne. Pour le rôle, la couleur, le fil ou le rang, passer par `modifier`. |
| `/forum message supprimer` | Supprime la carte. Le rôle reste sur le serveur et ses porteurs le gardent. Demande `confirmer: True`. |
| `/forum message liste` | Liste les cartes d'un fil avec leur identifiant et ce qu'elles accordent réellement. |

**Fils**

| Commande | Effet |
|---|---|
| `/forum fil créer` | Rattache un fil Discord existant : nom, couleur, illustration, introduction, rôle parent. |
| `/forum fil modifier` | Modifie ces réglages. La valeur `-` efface un champ ; `sans_role_parent: True` retire le rôle parent. |
| `/forum fil mp` | Définit le message privé envoyé à la première réaction dans le fil, ou le recopie d'un message existant avec `depuis_message`. 2 000 caractères maximum. `texte: -` le désactive. |
| `/forum fil supprimer` | Retire le fil du bot et efface ses cartes. Les rôles restent. Demande `confirmer: True`. |
| `/forum fil liste` | Liste les fils et leurs réglages. |

**Contenu**

| Commande | Effet |
|---|---|
| `/forum exporter` | Rend le contenu actuel du bot sous forme de `forum.toml`, à comparer au vôtre avant de réimporter. |
| `/forum importer` | Importe un fichier `forum.toml`. Rejouable, ne supprime rien, édite les cartes sur place. Masquée si `ENABLE_IMPORT=false`. |
| `/forum republier` | Réécrit un fil entier pour rétablir l'ordre des cartes. **Efface toutes les mains levées du fil.** Demande `confirmer: True` ; sans lui, elle annonce seulement ce qu'elle ferait. |

### Les champs du fichier de contenu

Un **fil** accepte `nom`, `salon`, `couleur`, `illustration`, `role_parent`,
`mp`, plus deux réglages plus récents :

- `confirmations = true` : un message privé confirme chaque entrée et chaque
  sortie de ce fil. À réserver aux fils où l'on rejoint un groupe ; dix
  compétences cochées d'affilée feraient dix messages.
- `salon_notifications` : l'identifiant du salon où annoncer les mains levées,
  quand la carte n'en a pas à elle.

Un **message** accepte `slug`, `titre`, `texte`, `role`, `couleur`, plus :

- `information = true` : carte affichée sans réaction, qui n'accorde rien et
  n'envoie rien, même dans un fil à rôle parent.
- `mp` : son message privé propre, envoyé à la place de celui du fil.
- `sans_role_parent = true` : la carte renonce au rôle parent de son fil. Avec
  un `mp`, elle porte une main levée qui n'accorde rien et se contente de
  répondre en privé.
- `referent` : le `@pseudo` de la personne mentionnée dans l'annonce.
- `salon_notifications` : le salon où annoncer, qui l'emporte sur celui du fil.

## 4. À savoir

- **Introduction d'un fil** : c'est le premier message du fil, écrit et corrigé à la main par l'équipe. Le bot n'y touche pas, ce qui permet de la changer sans rien republier.
- **Rôle parent** : accordé par chaque carte d'un fil, repris seulement quand la personne n'a plus aucune carte de ce fil.
- **Carte grisée** : couleur `#99AAB5`.
- **Fils archivés** : Discord archive un fil après 3 jours sans message, ce qui bloque les réactions. Le bot le rouvre aussitôt. Un fil verrouillé reste fermé.
- **Rôle retiré à la main** : la 🙋 reste sur la carte. Retirez-la puis remettez-la pour récupérer le rôle.
- **Message privé** : envoyé une seule fois par personne et par fil, et une
  seule fois par personne et par carte pour une carte qui a le sien. Un membre
  qui refuse les messages privés du serveur ne reçoit rien, et le bot n'y peut
  rien : les liens importants doivent aussi figurer dans l'introduction du fil.
- **Annonce aux référents** : chaque main levée est annoncée dans le salon de la
  carte, en mentionnant son référent. Le bot doit pouvoir écrire dans ce salon,
  sinon l'annonce échoue en silence et seuls les journaux le disent.
- **Référents** : écrivez `@pseudo` dans le texte d'une carte. S'il correspond à un membre du serveur, le bot en fait une mention cliquable ; sinon il le laisse en texte et vous le signale.

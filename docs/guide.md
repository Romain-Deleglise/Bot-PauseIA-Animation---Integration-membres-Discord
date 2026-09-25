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
vous. Une commande qui détruit quelque chose demande `confirmer: True` : lancée
sans, elle décrit ce qu'elle ferait et ne fait rien.

Trois notions reviennent partout :

- une **carte**, le message encadré qu'on publie pour un projet, une équipe, un groupe local ;
- un **fil**, le salon de forum qui rassemble des cartes et leur donne des réglages communs ;
- le **MP d'accueil**, le message privé envoyé à la première main levée.

### Je veux…

| … | Commande |
|---|---|
| ajouter un projet ou une équipe | `/forum message créer` |
| corriger un titre ou un texte | `/forum message éditer` |
| mettre une carte en pause sans l'effacer | `/forum message modifier`, `desactiver: True` |
| retirer une carte pour de bon | `/forum message supprimer` |
| changer le message privé d'accueil | `/forum fil mp`, ou `/forum message mp` pour une seule carte |
| changer ce qu'on écrit quand on lève la main | `/forum fil confirmation` |
| remettre les cartes dans l'ordre | `/forum republier` |
| savoir ce que le bot contient vraiment | `/forum exporter` |

### Les cartes

| Commande | Ce qu'elle fait |
|---|---|
| `/forum message créer` | Publie une carte : titre, texte, rôle, couleur, rang. Sans rôle, elle accorde celui du fil. |
| `/forum message éditer` | Ouvre une fenêtre pré-remplie pour reprendre le titre et le texte en multiligne. |
| `/forum message modifier` | Change les réglages : rôle, couleur, fil, rang (voir les options ci-dessous). |
| `/forum message mp` | Ouvre une fenêtre pré-remplie avec le message privé propre à cette carte. |
| `/forum message supprimer` | Efface la carte du fil et de la base. |
| `/forum message liste` | Liste les cartes d'un fil, leur identifiant et ce qu'elles accordent. |

Options de `/forum message modifier` :

- `desactiver: True` met la carte en pause. Elle reste affichée, sa 🙋 disparaît, elle n'accorde plus rien. `desactiver: False` la réveille.
- `retirer_rôle: True` détache le rôle de la carte. Le rôle Discord et ceux qui le portent ne bougent pas.
- Le titre et le texte se corrigent sur place : les mains levées et les rôles déjà donnés sont conservés.

Options de `/forum message supprimer` :

- `confirmer: True` est toujours exigé.
- Par défaut le rôle survit à la carte, et ses porteurs le gardent.
- `supprimer_rôle: True` détruit aussi le rôle Discord. Comme ses porteurs perdraient alors l'accès à des salons sans retour possible, le bot réclame en plus `confirmer_rôle: True`.

### Les fils

| Commande | Ce qu'elle fait |
|---|---|
| `/forum fil créer` | Rattache au bot un fil Discord existant : nom, couleur, illustration, rôle parent. |
| `/forum fil modifier` | Change ces réglages. `-` efface un champ, `sans_role_parent: True` retire le rôle parent. |
| `/forum fil mp` | Ouvre une fenêtre pré-remplie avec le message privé d'accueil du fil. |
| `/forum fil confirmation` | Affiche ou règle ce que le bot écrit quand on lève ou baisse la main. |
| `/forum fil supprimer` | Retire le fil du bot et efface ses cartes. Les rôles restent. Demande `confirmer: True`. |
| `/forum fil liste` | Liste les fils et leurs réglages. |

`/forum fil mp` et `/forum message mp` s'utilisent de la même façon : elles
ouvrent une fenêtre déjà remplie avec le texte actuel, à corriger directement.
Videz-la, ou laissez un simple `-`, pour retirer le message. 2 000 caractères
maximum.

Retirer le message d'une carte la rend à celui de son fil. Retirer celui d'un
fil laisse ses mains levées sans aucun accueil.

`/forum fil mp` sans fil n'ouvre rien : elle affiche **tous** les messages
d'accueil, ceux des fils comme ceux des cartes.

`/forum fil confirmation` règle les deux messages envoyés à chaque main levée ou
baissée, `moment: arrivée` ou `moment: départ`. Sans `texte`, le message
s'affiche. Deux marqueurs sont remplacés à l'envoi :

- `{carte}` par le titre de la carte, `{rôles}` par les rôles concernés ;
- tout autre marqueur est refusé à la saisie, pour qu'une coquille ne parte pas à chaque membre ;
- `texte: -` rétablit le texte d'origine.

Ces messages ne partent que si le fil a `confirmations` activé. Le bot vous le
rappelle si ce n'est pas le cas.

### Le contenu

| Commande | Ce qu'elle fait |
|---|---|
| `/forum exporter` | Renvoie le contenu réel du bot au format `forum.toml`. |
| `/forum importer` | Applique un `forum.toml`. Rejouable, ne supprime rien, édite les cartes sur place. Masquée si `ENABLE_IMPORT=false`. |
| `/forum republier` | Réécrit un fil pour rétablir l'ordre des cartes. **Efface toutes les mains levées du fil.** Demande `confirmer: True`. |

L'import **écrase** : ce que le fichier décrit remplace ce que Discord affiche.
Le bon ordre est donc toujours le même : `/forum exporter`, comparer au fichier
qu'on s'apprête à importer, fusionner les deux, puis `/forum importer`. Sans
cette comparaison, une carte corrigée depuis Discord est effacée sans un mot.

Le fichier accepte quelques champs qui n'ont pas de commande. Pour un fil :
`confirmations` et `salon_notifications`. Pour un message : `information`,
`sans_role_parent`, `mp`, `referent` et `salon_notifications`.

## 4. À savoir

- **Rôle parent** : accordé par chaque carte d'un fil, repris seulement quand la personne n'a plus aucune carte de ce fil.
- **Introduction d'un fil** : c'est le premier message du fil, écrit à la main. Le bot n'y touche pas, donc la changer ne coûte rien.
- **Carte grisée** : couleur `#99AAB5`.
- **Fils archivés** : Discord archive un fil après 3 jours sans message, ce qui bloque les réactions. Le bot le rouvre aussitôt. Un fil verrouillé reste fermé.
- **Rôle retiré à la main** : la 🙋 reste sur la carte. Retirez-la puis remettez-la pour récupérer le rôle.
- **Message d'accueil ou confirmation ?** L'accueil part une seule fois, à la première main levée dans le fil (ou sur la carte, si elle a le sien). Les confirmations partent à chaque main levée ou baissée. Un membre qui refuse les MP de serveur ne reçoit ni l'un ni l'autre, mais garde ses rôles.
- **Annonce aux référents** : chaque main levée est annoncée dans le salon de la carte. Le bot doit pouvoir y écrire, sinon l'annonce échoue en silence.
- **Référents** : écrivez `@pseudo` dans le texte d'une carte. S'il correspond à un membre, le bot en fait une mention ; sinon il le signale à l'import.

# Piloter le forum Contribuer

Le bot publie les cartes du forum **Contribuer** et distribue les rôles quand
quelqu'un lève la main. Cette page liste ce que vous pouvez faire depuis
Discord. Aucune installation, aucun compte à créer : tout se tape dans un salon.

Les réponses du bot ne sont visibles que de vous.

## En trente secondes

Tapez `/forum` dans n'importe quel salon, la liste s'affiche. Trois familles :

- `/forum message …` agit sur **une carte** — un projet, une équipe, un groupe local, une compétence.
- `/forum fil …` agit sur **un fil entier** et ses réglages communs.
- `/forum exporter` rend une copie de tout le contenu.

Rien ne s'efface sans que vous le confirmiez. Une commande lancée sans
`confirmer: True` décrit ce qu'elle ferait, et s'arrête là.

## Je veux…

| … | La commande |
|---|---|
| corriger un titre ou un texte | `/forum message éditer` |
| ajouter un projet, une équipe, un groupe | `/forum message créer` |
| changer la couleur, le rôle ou le rang d'une carte | `/forum message modifier` |
| mettre une carte en pause sans l'effacer | `/forum message modifier`, `desactiver: True` |
| voir ce qu'une carte accorde vraiment | `/forum message liste` |
| changer le message privé d'accueil d'un fil | `/forum fil mp` |
| changer ce que reçoit quelqu'un qui lève la main | `/forum fil confirmation` |
| changer l'annonce lue par les référents | `/forum fil annonce` |
| retirer une carte pour de bon | `/forum message supprimer` |

## Corriger une carte

`/forum message éditer` ouvre une fenêtre avec le titre et le texte actuels.
Vous corrigez, vous validez. Les mains levées et les rôles déjà donnés sont
conservés : c'est l'opération la plus sûre du lot, utilisez-la sans crainte.

`/forum message modifier` sert au reste : la couleur (une palette s'affiche
quand vous tapez le paramètre), le rôle accordé, le fil, le rang dans le fil.
Deux options valent d'être connues :

- `desactiver: True` met la carte en pause. Elle reste affichée, sa 🙋 disparaît, elle n'accorde plus rien. `desactiver: False` la réveille. À préférer à la suppression dans presque tous les cas.
- `retirer_rôle: True` détache le rôle de la carte. Le rôle Discord et les gens qui le portent ne bougent pas.

## Les messages privés

Le bot envoie trois sortes de messages privés. Ne les confondez pas.

**Le message d'accueil du fil**, envoyé une seule fois, à la première main levée
dans ce fil. Il y en a un par fil, soit quatre en tout. C'est celui qu'on
modifie presque toujours :

```
/forum fil mp    fil: Projets
```

Une fenêtre s'ouvre avec le texte actuel. Vider le champ supprime l'accueil de
tout le fil. Sans indiquer de fil, la commande affiche tous les messages
d'accueil du forum d'un coup.

**L'exception d'une carte**, quand une carte doit dire autre chose que ses
voisines. Une seule en a une aujourd'hui : Veille, dont le projet est en pause
et qui doit expliquer pourquoi elle n'accorde aucun rôle.

```
/forum message mp    message: Veille
```

Sur toutes les autres cartes la fenêtre s'ouvre vide, et c'est normal : elles
suivent leur fil. Vider le champ supprime l'exception.

**Les confirmations**, envoyées à chaque main levée ou baissée :

```
/forum fil confirmation    fil: Projets    moment: arrivée
```

Deux marqueurs y sont remplacés à l'envoi : `{carte}` par le titre de la carte,
`{rôles}` par les rôles concernés. Tout autre marqueur est refusé à la saisie.
`texte: -` rétablit le texte d'origine.

## Prévenir les référents

Quand quelqu'un lève la main, le bot l'annonce dans le salon du projet et
mentionne son référent. Ce texte se règle aussi :

```
/forum fil annonce    fil: Projets    moment: arrivée
```

Marqueurs : `{carte}` et `{membre}`. Le rappel au référent s'ajoute tout seul
en dessous quand la carte en a un.

## Ce qui demande de la prudence

| Commande | Ce qu'elle coûte |
|---|---|
| `/forum message supprimer` | La carte disparaît. Le rôle reste sur le serveur, sauf avec `supprimer_rôle: True`, qui exige un second accord. |
| `/forum fil supprimer` | Le fil et toutes ses cartes. Les rôles restent. |
| `/forum republier` | Réécrit un fil pour remettre les cartes dans l'ordre. **Efface toutes les mains levées du fil.** |
| `/forum importer` | Applique un fichier de contenu, qui **remplace** ce que Discord affiche. |

Ces quatre commandes sont réservées aux responsables du forum.

Avant tout import, lancez `/forum exporter` et comparez : le fichier écrase ce
qui a été corrigé depuis Discord, sans prévenir.

## Bon à savoir

- **Rôle parent** : chaque carte d'un fil accorde aussi le rôle du fil. Il n'est repris que lorsque la personne n'a plus aucune carte de ce fil — quitter Paris ne fait pas perdre l'accès aux salons des groupes locaux si on est aussi à Lyon.
- **Introduction d'un fil** : c'est le premier message du fil, écrit à la main. Le bot n'y touche pas, la corriger ne coûte rien.
- **Rôle retiré à la main** : la 🙋 reste sur la carte. Retirez-la puis remettez-la pour récupérer le rôle.
- **Messages privés fermés** : un membre qui refuse les MP de serveur ne reçoit rien, mais garde ses rôles. Les liens importants figurent aussi dans les cartes.
- **Une commande a disparu de la liste ?** Faites Ctrl+R : Discord garde les commandes en cache.

---

Un problème, une question : contactez l'équipe Tech sur le Discord.
L'installation et l'exploitation du bot sont décrites dans le [guide technique](guide.md).

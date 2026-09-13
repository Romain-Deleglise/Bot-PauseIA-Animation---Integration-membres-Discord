# Forum PauseIA — mise en place

> **Document généré par Claude** à partir du cahier des charges et de nos
> échanges, relu par Antoine avant envoi. Mis à jour le 27 août 2026 avec vos
> retours : rôle parent au niveau du fil, statuts supprimés, rôles affichés sous
> chaque carte.

Ce document accompagne le bot qui fera vivre le forum décrit dans le cahier des
charges. Il couvre quatre choses :

1. [les décisions prises](#1-décisions-prises) là où le cahier des charges ne
   tranchait pas ;
2. [ce qu'il faut préparer sur le Discord](#2-à-préparer-sur-le-discord) ;
3. [les valeurs à transmettre](#3-valeurs-à-transmettre) pour configurer le bot ;
4. [comment remplir le fichier de contenu](#4-remplir-le-fichier-de-contenu),
   éventuellement en le faisant rédiger par une IA.

Rien n'est irréversible dans les décisions du point 1 : si l'une d'elles ne vous
convient pas, dites-le, elles se changent.

---

## Ce que fait le bot, en une minute

Le forum contient quatre fils : **Projets**, **Équipes**, **Groupes locaux**,
**Compétences**. Dans chaque fil, le bot publie une **carte** par activité : un
titre, un texte, une couleur.

Lever la main 🙋 sur une carte donne le ou les rôles associés. Retirer sa
réaction les reprend. Certaines cartes ne donnent aucun rôle : elles ne portent
alors aucune réaction et servent d'information.

Un fil peut envoyer un **message privé de bienvenue** à la première réaction
d'un membre, une seule fois.

Toute la gestion se fait par des commandes Discord, réservées à des rôles que
vous désignerez. Il n'y a aucune interface web à administrer.

---

## 1. Décisions prises

Le cahier des charges laissait plusieurs points ouverts. Voici ce qui a été
retenu, et pourquoi.

### 1.1 Un fil peut avoir un rôle parent

Un fil déclare zéro ou un **rôle parent** — `@groupe-local` pour les groupes
locaux, `@portail-équipe` pour les équipes. Tous les messages du fil l'accordent
en plus du leur, sans avoir à le préciser. Il n'y a pas d'exception possible
message par message.

Il se change par commande : `/forum fil modifier role_parent:@…`, ou
`sans_role_parent: True` pour l'enlever. Les cartes du fil sont mises à jour
dans la foulée.

### 1.2 Le rôle partagé n'est repris que s'il n'est plus justifié

Situation : un membre lève la main sur Paris, puis sur Lyon. Il a `@gl-paris`,
`@gl-lyon` et `@groupe-local`. S'il retire sa réaction sur Paris :

- `@gl-paris` lui est repris ;
- `@groupe-local` lui **reste**, puisqu'il est toujours à Lyon ;
- il ne le perdra qu'en quittant son dernier groupe.

Le CDC ne le précisait pas. L'autre choix — reprendre les deux — ferait perdre
l'accès aux salons communs à quelqu'un qui est toujours dans un groupe.

### 1.3 Retirer sa réaction retire le rôle

Comportement repris de l'ancien bot, que le CDC ne mentionnait pas. Un membre
peut donc quitter un projet ou un groupe tout seul, sans demander à personne.

### 1.4 Pas de statuts

Abandonnés à votre demande. Un projet inactif ou en structuration se signale
dans le titre ou le texte de sa carte, comme le fait déjà le CDC avec
`[Inactif]`, `[En structuration]`, `[À venir]` et `[Besoin d'aide]`. Ces
marqueurs sont repris tels quels dans le fichier de contenu.

Pour éditer un texte : `/forum message modifier`, en choisissant le message dans
les suggestions. La carte est modifiée sur place, les réactions et les rôles
déjà accordés ne bougent pas.

### 1.5 Supprimer un message garde son rôle

Supprimer la carte de Bordeaux l'efface du fil, et c'est tout : `@gl-bordeaux` reste sur le serveur, et ceux qui le portent le gardent. S'il n'a plus lieu d'être, supprimez-le à la main dans *Paramètres du serveur → Rôles*.

Décision prise lors de la démo, à la place de la lecture du CDC qui supprimait aussi le rôle : un rôle règle souvent l'accès à des salons, le détruire doit rester une décision humaine.

La commande demande une confirmation explicite avant d'agir.

### 1.6 Chaque carte affiche un titre, un texte et ses rôles

Une carte peut porter sa propre couleur, qui prime sur celle du fil : c'est
ainsi qu'on grise un groupe endormi, sans introduire de notion de statut. Les
deux groupes `[Inactif]` sont déjà grisés dans le fichier de contenu.

L'affichage des rôles sous la carte a été ajouté à votre demande. Voir le rendu
dans [docs/maquette-fil.png](docs/maquette-fil.png).

Deux champs de l'ancien bot restent retirés : la catégorie, qui répétait le nom
du fil, et le décompte de membres, figé au moment de la publication et donc faux
le lendemain. Si vous voulez indiquer un effectif, écrivez-le dans le texte,
comme le fait la maquette du CDC (« 4 personnes »).

Un message sans rôle n'affiche pas de rôles et ne porte pas de réaction.

### 1.7 L'ordre des cartes est explicite

L'ancien bot triait les cartes selon la position des rôles dans les paramètres
du serveur. Ça ne marche plus : un message d'information n'a pas de rôle, donc
pas de position.

L'ordre est désormais celui du fichier de contenu, et se modifie message par
message. Une commande permet de tout republier dans le bon ordre.

> **Attention** : republier un fil recrée toutes ses cartes. Les membres gardent
> leurs rôles, mais leurs réactions disparaissent — il faudra qu'ils lèvent la
> main de nouveau s'ils veulent pouvoir se retirer un rôle. C'est la seule façon
> de corriger un ordre : Discord ne permet pas de déplacer un message. Toutes les
> autres opérations (changer un texte, un rôle) modifient la carte sur
> place, sans rien perdre.

### 1.8 Un emoji unique pour tout le forum

🙋 par défaut, comme dans les textes du CDC. Il est configurable, mais il est le
même partout : un emoji différent par carte compliquerait la lecture sans rien
apporter.

### 1.9 Un rôle appartient à un seul message

Deux cartes ne peuvent pas donner le même rôle principal. Sans cette règle, la
réévaluation du rôle partagé (point 1.2) n'aurait pas de réponse unique. Les
rôles partagés, eux, sont bien utilisés par plusieurs messages — c'est leur
raison d'être.

### 1.10 Le message privé est stocké par le bot

Le CDC demande de pouvoir le modifier « sans que les utilisateurs qui ont réagi
soient notifiés ». Il n'est donc pas un message Discord mais un texte gardé par
le bot : le modifier ne notifie personne, par construction.

Il peut être tapé dans la commande, ou recopié depuis un message existant —
pratique pour un texte long, rédigé tranquillement dans un salon
d'administration. Le bot en fait une copie : le message modèle peut ensuite être
supprimé sans conséquence.

Un membre qui a fermé ses messages privés reçoit quand même son rôle ; le bot
réessaiera à sa prochaine réaction dans le fil.

### 1.11 Qui peut administrer

Le CDC dit « les admins du Discord (ou à minima, l'admin qui a créé le message
correspondant) ». Le bot retient la première formule : **vous désignez une liste
de rôles**, et quiconque porte l'un d'eux peut tout éditer.

Suivre l'auteur de chaque message aurait empêché un collègue de corriger une
faute de frappe pendant les vacances de son auteur.

### 1.12 Ce que le bot ne gère pas

- **La description du forum lui-même** : c'est le sujet du salon, à écrire à la
  main dans les paramètres Discord. Une commande n'aurait rien apporté.
- **La création des fils** : le bot publie dans des fils existants, il n'en crée
  pas. Vous gardez la main sur leur nom, leur ordre et leurs permissions.
- **L'illustration en tête de fil** : le bot la publie comme premier message du
  fil, pas comme image du fil lui-même — le message d'ouverture d'un fil de
  forum appartient à celui qui l'a créé et ne peut pas être modifié par le bot.

### 1.13 Corrections apportées au contenu du CDC

- **Les descriptions de « Groupes locaux » et « Compétences » étaient
  interverties** : Groupes locaux annonçait « les savoir-faire qui font avancer
  nos projets », et Compétences « les groupes Pause IA près de chez vous ». Elles
  ont été remises à leur place. Les textes d'introduction sous chaque tableau
  étaient, eux, corrects.
- **Lyon** n'avait que `TODO` pour tout texte : remplacé par le texte que vous
  avez fourni.
- **Melun** et **Colmar** n'avaient pas de titre, leur description commençant
  directement par `[Inactif]` : les titres ont été déduits du nom du rôle.
- **Relations associations** n'avait ni statut ni salon : le marqueur
  `[À venir]` de son titre est conservé, le salon reste à définir.
- `@com-vulga-ia` était bien une coquille : corrigé en `@comp-vulga-ia`.
- Le référent du projet « Accueil et orientation » est passé à
  `@pouetpouetaumarche`.

---

## 2. À préparer sur le Discord

Cochez dans l'ordre. Les points 2.1 à 2.3 demandent un compte administrateur.

### 2.1 Créer l'application et le bot

Sur [discord.com/developers/applications](https://discord.com/developers/applications) :

1. **New Application**, nommez-la (« Bot Forum » par exemple).
2. Onglet **Bot** → **Reset Token** → copiez le jeton, il ne sera plus jamais
   affiché. C'est la première valeur à transmettre (voir §3).
3. Toujours dans **Bot**, activez **SERVER MEMBERS INTENT**.

> Cet interrupteur ne sert pas à surveiller les membres. Sans lui, le bot ne voit pas les changements de rôles des membres, et risquerait de reprendre un rôle parent à tort.

### 2.2 Inviter le bot

Onglet **OAuth2 → URL Generator** :

- **Scopes** : `bot` **et** `applications.commands` (les deux, sans quoi les
  commandes n'apparaîtront pas) ;
- **Bot Permissions** : `Gérer les rôles`.

Ouvrez l'URL générée et invitez le bot sur le serveur.

### 2.3 Placer le rôle du bot tout en haut

*Paramètres du serveur → Rôles* : faites glisser le rôle du bot **au-dessus de
tous les rôles qu'il devra gérer** — les 38 rôles de projets, équipes, groupes
et compétences, plus `@groupe-local` et `@portail-équipe`.

> **C'est la cause d'incident numéro un.** Discord interdit à quiconque de
> toucher à un rôle situé plus haut que le sien. Si le rôle du bot est en
> dessous, chaque commande échouera avec une erreur de permissions, alors même
> que la permission « Gérer les rôles » est bien accordée.

### 2.4 Créer le salon forum et ses quatre fils

Créez un salon de type **Forum**, puis quatre publications dans ce forum :
**Projets**, **Équipes**, **Groupes locaux**, **Compétences**.

Vérifiez que **tous les membres voient le salon et les quatre fils** : le CDC
demande qu'ils y aient accès dès leur arrivée.

Sur ce salon, le bot a besoin des permissions suivantes :

- Voir le salon
- Envoyer des messages
- Créer des fils / envoyer des messages dans les fils
- Intégrer des liens
- Ajouter des réactions
- Gérer les messages
- Voir les messages précédents

Écrivez la description du forum dans le sujet du salon. Texte proposé par le
CDC :

> Bienvenue dans le forum !
>
> Vous voulez aider Pause IA mais ne savez pas où commencer ? Vous êtes au bon
> endroit.
>
> Ici vous pouvez explorer notre structure et nos projets : qui fait quoi, où,
> et ce que vous pouvez rejoindre. Parcourez les différentes catégories et levez
> la main (🙋), vous recevrez les accès associés.
>
> N'hésitez pas à contacter les référents de chaque projet / groupe en message
> privé si vous avez des questions.

### 2.5 Relever les identifiants des quatre fils

Activez le mode développeur : *Paramètres utilisateur → Avancés → Mode
développeur*.

Puis, sur chaque fil : clic droit → **Copier l'identifiant**. Notez les quatre
nombres, ils iront dans le fichier de contenu (§4).

### 2.6 Les rôles : rien à faire

Aucun rôle n'est à créer à la main. L'import crée les 38 rôles de messages **et**
les deux rôles parents `groupe-local` et `portail-équipe`.

Si certains existent déjà, le bot les réutilise dès lors que le nom correspond,
sans créer de doublon.

### 2.7 Désigner qui peut administrer le bot

Choisissez le ou les rôles autorisés à utiliser les commandes — typiquement
`@tech`, `@bureau`, ou un rôle dédié. Relevez leurs identifiants (clic droit sur
le rôle → Copier l'identifiant).

### 2.8 Créer les salons manquants

Plusieurs projets du CDC renvoient à un salon « à créer » ou « à définir ».
Créez-les et donnez-leur la bonne permission de lecture au rôle correspondant,
sans quoi rejoindre le projet ne donnera accès à rien :

| Projet | Salon |
|---|---|
| PauseAction | à définir : nouveau salon, ou réutiliser `#pause-action` |
| Accueil et orientation | `#onboarding`, à renommer |
| Animation Discord | à créer |
| Organisation des rencontres | à créer |
| Partenariats créateurs | à créer |
| Création artistique | à créer |
| Jeu vidéo et jeu de rôle | à créer |
| Création de flyers | à créer |
| Relations associations | à définir |

Ce raccordement salon ↔ rôle se fait dans les permissions de chaque salon, pas
dans le bot.

---

## 3. Valeurs à transmettre

Quatre valeurs suffisent à configurer le bot. Transmettez-les par un canal
privé — **le jeton donne le contrôle complet du bot**.

| Valeur | Où la trouver | Exemple |
|---|---|---|
| **Jeton du bot** | Portail développeur → Bot → Reset Token | `MTA5…` (~70 caractères) |
| **Identifiant du serveur** | Clic droit sur le nom du serveur → Copier l'identifiant | `837023715971039288` |
| **Rôles administrateurs** | Clic droit sur chaque rôle → Copier l'identifiant | `837023716016783412,912…` |
| **Emoji de réaction** | 🙋 par défaut ; sinon un emoji du serveur | `🙋` ou `<:meduse:872735843771633734>` |

À ajouter aussi, mais dans le fichier de contenu et non dans la configuration :
**les quatre identifiants de fils** relevés au point 2.5.

Si le jeton fuite : Reset Token dans le portail développeur invalide
immédiatement l'ancien.

---

## 4. Remplir le fichier de contenu

Le contenu du forum est décrit dans un fichier `forum.toml`, fourni **déjà
rempli** avec les 38 messages du cahier des charges. Il reste à :

1. remplacer les quatre identifiants de fils factices par les vrais ;
2. compléter le texte de Lyon ;
3. ajouter les illustrations, si vous en avez ;
4. rédiger les messages privés de bienvenue, si vous en voulez.

Le plus simple est de passer par **l'éditeur web** publié avec le projet. Vous y
déposez votre `forum.toml` : il affiche chaque carte telle qu'elle apparaîtra
dans Discord, signale ce qui manque, et le réexporte quand vous avez fini. Vous
en ressortez avec un fichier à déposer dans `/forum importer`.

L'éditeur démarre vide et ne conserve rien : tout se passe dans votre
navigateur, rien n'est envoyé ni enregistré en ligne. Le contenu du forum n'est
pas publié avec la page — il nomme des personnes et des villes.

Le reste de cette section décrit le format, pour qui préfère éditer le fichier
directement.

### 4.1 À quoi ressemble le fichier

Un bloc par fil, puis un bloc par message :

```toml
[[fils]]
nom             = "Groupes locaux"
salon           = "1234567890123456789"
couleur         = "#F2994A"
role_parent     = "groupe-local"
description     = """
PauseIA près de chez vous.
Levez la main 🙋 pour rejoindre le vôtre."""
mp              = """
Bienvenue dans votre groupe local !
Présentez-vous dans le salon du groupe."""

[[fils.messages]]
slug  = "paris"
titre = "Paris"
role  = "gl-paris"
texte = """
Référent : @pseudo_du_referent
Un groupe très actif : tractage, manifestations, conférences…"""
```

### 4.2 Les champs, un par un

**Pour un fil** (`[[fils]]`) :

| Champ | Obligatoire | Remarque |
|---|---|---|
| `nom` | non | Sert à désigner le fil dans les commandes. Absent : le nom du fil sur Discord est repris |
| `salon` | oui | Identifiant du fil, **entre guillemets** |
| `couleur` | non | `#RRGGBB`. Sans elle, les cartes sont sans couleur |
| `illustration` | non | URL `https` d'une image publiée en tête de fil |
| `description` | non | Texte d'introduction publié en tête de fil. À omettre pour un fil créé à la main : son message d'ouverture sert d'introduction |
| `role_parent` | non | Rôle accordé en plus du sien par chaque message du fil |
| `mp` | non | Message privé envoyé à la première réaction |

**Pour un message** (`[[fils.messages]]`) :

| Champ | Obligatoire | Remarque |
|---|---|---|
| `slug` | oui | Identifiant interne. **À ne plus changer après le premier import** |
| `titre` | oui | Titre affiché sur la carte |
| `role` | non | Nom du rôle accordé. **Absent = message d'information, sans réaction** |
| `couleur` | non | `#RRGGBB` propre à la carte. `#99AAB5` la grise |
| `texte` | non | Corps de la carte |

### 4.3 Les règles à respecter

- **`salon` est entre guillemets.** Les identifiants Discord sont trop grands
  pour être écrits comme des nombres.
- **Les noms de rôles s'écrivent sans `@`** — `gl-paris`, pas `@gl-paris`. La
  casse n'a pas d'importance. Un rôle qui n'existe pas est créé automatiquement.
- **Un `slug` est unique dans son fil**, et ne se change plus une fois importé :
  c'est lui qui permet au bot de reconnaître un message qu'il a déjà publié. Le
  changer créerait un doublon.
- **Un rôle ne peut être le rôle principal que d'un seul message** dans tout le
  fichier. Les rôles partagés, eux, peuvent servir partout.
- **Les textes longs s'écrivent entre triples guillemets** `"""`, sur plusieurs
  lignes.
- **Pour mentionner quelqu'un**, écrivez simplement `@pseudo` : le bot le transforme en mention cliquable si la personne est membre du serveur, et signale dans le compte rendu les pseudos qu'il n'a pas trouvés. Un salon (`<#123456789>`) ou un rôle (`<@&123456789>`) s'écrit toujours avec son identifiant.

Le bot refuse un fichier invalide en expliquant ce qui ne va pas, **sans avoir
rien modifié**. Vous pouvez donc essayer sans crainte.

### 4.4 Faire rédiger le fichier par une IA

Utile pour convertir des textes en série. Pour quelques retouches, l'éditeur web
est plus rapide.

Pour ajouter des messages en série ou refondre un fil, vous pouvez copier le
prompt suivant dans une IA, en y joignant vos textes :

```
Tu écris un fichier de configuration TOML pour un bot Discord.

FORMAT
Un bloc [[fils]] par fil du forum, suivi d'autant de blocs
[[fils.messages]] que de messages dans ce fil. Les blocs de messages
d'un fil doivent suivre immédiatement le bloc de leur fil.

CHAMPS D'UN FIL
  nom              (facultatif)  nom du fil ; absent, celui de Discord est repris
  salon            (obligatoire) identifiant Discord, ENTRE GUILLEMETS
  couleur          (facultatif)  "#RRGGBB"
  illustration     (facultatif)  URL https d'une image
  description      (facultatif)  texte d'introduction du fil ; omets-le si le fil a déjà son message d'ouverture
  role_parent      (facultatif)  nom d'un rôle accordé par tous ses messages
  mp               (facultatif)  message privé de bienvenue

CHAMPS D'UN MESSAGE
  slug             (obligatoire) identifiant en minuscules, sans espace,
                                 unique dans son fil
  titre            (obligatoire) titre affiché
  role             (facultatif)  nom du rôle accordé, SANS le @.
                                 Omets-le pour un message d'information.
  couleur          (facultatif)  "#RRGGBB" propre à la carte, prime sur le fil
  texte            (facultatif)  corps du message, entre """ si multiligne

RÈGLES
- Les valeurs de `salon` sont toujours entre guillemets.
- Les noms de rôles n'ont jamais de @ devant.
- Un même nom de rôle ne peut apparaître dans `role` qu'une seule fois
  dans tout le fichier. Un `role_parent` peut se répéter.
- Deux messages d'un même fil ne peuvent pas avoir le même slug.
- Une personne se mentionne par son pseudo, @pseudo : le bot le convertit lui-même. Un salon s'écrit <#identifiant> et un rôle <@&identifiant>. N'invente jamais d'identifiant.
- N'ajoute aucun champ qui ne figure pas dans la liste ci-dessus.
- Ne réponds qu'avec le TOML, sans commentaire autour.

CONTENU À CONVERTIR
[collez ici vos textes]
```

Relisez toujours le résultat : une IA invente volontiers des identifiants
plausibles. Les seuls chiffres du fichier qui comptent sont ceux que vous avez
relevés vous-même dans Discord.

---

## 5. Le jour de la mise en route

1. Le bot est démarré et apparaît en ligne sur le serveur.
2. Déposez le fichier dans Discord : `/forum importer` puis glissez `forum.toml`.
3. Le bot vérifie d'abord que les quatre fils existent et qu'il peut y écrire.
   **Si l'un manque, il s'arrête sans rien modifier** et vous dit lequel.
4. Sinon il crée les rôles absents, publie les 38 cartes et rend un compte
   rendu : tant de fils, tant de messages, tant de rôles créés.
5. **Recette**, comme le demande le CDC : levez la main sur au moins un message
   de chaque fil et vérifiez les rôles obtenus. Vérifiez en particulier qu'un
   groupe local donne bien **deux** rôles, et qu'une équipe aussi.
6. Retirez vos réactions de test et vérifiez que les rôles repartent.

Si quelque chose ne va pas, l'import est rejouable autant de fois que voulu : il
met à jour ce qui a changé et ne supprime jamais rien.

---

## 6. Au quotidien

| Ce que vous voulez faire | Commande |
|---|---|
| Corriger un texte, ou griser une carte | `/forum message modifier` |
| Ajouter un projet ou un groupe | `/forum message créer` |
| Retirer un projet (son rôle reste) | `/forum message supprimer` |
| Voir ce qui est configuré | `/forum message liste` |
| Changer le rôle parent d'un fil | `/forum fil modifier` |
| Changer le message privé d'un fil | `/forum fil mp` |
| Remettre un fil dans l'ordre | `/forum republier` |

Les commandes apparaissent dans la liste de tout le monde, mais seuls les rôles
autorisés peuvent les lancer : toute autre personne reçoit un refus. Les réponses
du bot sont éphémères, visibles de vous seul.

Deux points à retenir :

- **modifier un texte ne fait rien perdre** : la carte est éditée
  sur place, les réactions et les rôles restent ;
- **`/forum republier` remet les réactions à zéro** : à n'utiliser que pour
  corriger un ordre d'affichage.

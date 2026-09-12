# Démo du bot sur la sandbox

> Déroulé généré par Claude à partir des tests faits sur la sandbox, relu par Antoine.

## Avant

- Le bot tourne avec les 8 permissions minimales (pas Administrateur), avec `ENABLE_IMPORT=true` et une base vide.
- Le fichier `data/forum-sandbox.toml` est prêt à glisser en pièce jointe.
- Idéalement, la personne à qui on fait la démo rejoint la sandbox comme simple membre : elle fait les réactions elle-même et voit les salons apparaître.

## Déroulé

| # | Action | Ce qu'on montre |
|---|---|---|
| 1 | `/forum importer fichier: forum-sandbox.toml` | Compte rendu : 4 fils, 38 cartes, rôles manquants créés. |
| 2 | Parcourir les 4 fils | Couleur par fil, rôles affichés sous chaque carte, Colmar et Melun grisés, cartes d'information sans 🙋. |
| 3 | `/forum fil mp fil: Groupes locaux texte: Bienvenue dans les groupes locaux ! Présentez-vous dans le salon de votre ville.` | Le message privé se règle depuis Discord, il n'est pas dans le TOML. |
| 4 | 🙋 sur **Paris** | `@gl-paris` + `@groupe-local`, message privé reçu, salons #gl-paris et #groupe-local visibles. |
| 5 | 🙋 sur **Lyon** | `@gl-lyon` ajouté, pas de second message privé. |
| 6 | Retirer la 🙋 de Paris | `@gl-paris` repris, `@groupe-local` conservé (Lyon le justifie encore). |
| 7 | Retirer la 🙋 de Lyon | `@gl-lyon` et `@groupe-local` repris, les salons disparaissent. |
| 8 | 🙋 sur une carte d'**Équipes** | Le rôle de l'équipe + `@portail-équipe`. |
| 9 | `/forum message modifier message: Groupes locaux › Paris texte: …` | Carte éditée sur place : réactions et rôles des membres conservés. |
| 10 | `/forum message modifier message: … couleur: #99AAB5`, puis `couleur: -` | Carte grisée, puis retour à la couleur du fil. |
| 11 | Créer le rôle `@gl-nantes` dans Discord, puis `/forum message créer fil: Groupes locaux titre: Nantes texte: … role: @gl-nantes` | Nouvelle carte avec sa 🙋, qui donne aussi `@groupe-local`. |
| 12 | 🙋 sur Nantes, puis `/forum message supprimer message: Groupes locaux › Nantes confirmer: False` | Refus, avec l'effet annoncé : le rôle sera conservé. |
| 13 | Même commande avec `confirmer: True` | Carte supprimée, `@gl-nantes` et ses porteurs intacts. Le rôle se supprime à la main s'il n'a plus lieu d'être. |
| 14 | `/forum message liste` et `/forum fil liste` | Vue d'ensemble du contenu et des réglages. |

## À expliquer sans exécuter

- `/forum fil créer` : l'import l'a déjà fait. Sert à rattacher un nouveau fil.
- `/forum fil modifier` : nom, couleur, illustration, introduction, rôle parent. Couleur et rôle parent sont répercutés sur les cartes, sur place.
- `/forum fil supprimer` : retire le fil du bot, les rôles restent.
- `/forum republier` : remet toutes les réactions à zéro et duplique l'introduction. À éviter hors réorganisation.
- `/forum importer` disparaît une fois `ENABLE_IMPORT=false`.
- Un fil archivé par Discord après 3 jours d'inactivité est rouvert automatiquement.

## Questions à poser

Détail dans [questions-ouvertes.md](questions-ouvertes.md).

1. Un message privé par ville plutôt qu'un seul par fil ?
2. ~~L'introduction d'un fil : écrite à la main, ou le bot crée les fils ?~~ Tranché : fils créés à la main, `nom` et `description` facultatifs dans le TOML.
3. Rôle retiré à la main : le bot doit-il retirer la 🙋 restée sur la carte ?
4. ~~Référents en `@pseudo` texte ou en vraies mentions ?~~ Tranché : mentions cliquables, converties par le bot.
5. Qui crée les salons « à créer » avant la mise en production ?
6. Quels rôles ont accès aux commandes (`MANAGE_ROLE_IDS`) ?

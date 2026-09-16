# Le bot en bref

Synthèse pour qui découvre le projet : ce que fait le bot, et pourquoi il est
fait comme ça. Le [guide](guide.md) détaille les commandes, le
[CDC](cdc.md) le besoin d'origine, et [CLAUDE.md](../CLAUDE.md) le code.

## Ce que fait le bot

- Il publie le contenu du salon forum **#contribuer** sous forme de **cartes**
  (des embeds), réparties en quatre fils : Projets, Équipes, Groupes locaux,
  Compétences.
- **Lever la main 🙋 sur une carte attribue un ou des rôles Discord.** Retirer
  la réaction les reprend. Rien d'autre à faire pour le membre.
- Il envoie un **message privé** d'accueil expliquant la suite : ce que le rôle
  ouvre, où aller, qui contacter.
- Il **confirme en privé** les entrées et les sorties, sur les fils où l'on
  rejoint un groupe d'humains.
- Tout se pilote depuis Discord, en commandes `/forum …` : créer un fil, éditer
  le texte d'une carte, changer le message privé. Aucun redémarrage.
- Le contenu initial se rédige dans un fichier `contenu/forum.toml`, versionné,
  importable d'un glisser-déposer, et éditable dans une **page web** sans rien
  installer.

## Le modèle

- **Une carte porte son identité, pas son rôle.** « Fresque de l'IA » est le
  titre affiché ; `@fresque-ia` est le rôle. Les deux se corrigent séparément.
- **Deux rôles possibles par carte** : le sien, et le **rôle parent** du fil,
  accordé par toutes les cartes de ce fil. Groupes locaux donne
  `@groupe-local` en plus de `@gl-paris`.
- **Le rôle parent n'est repris que s'il n'est plus justifié.** Inscrit à Paris
  et à Lyon, on quitte Paris sans perdre les salons communs ; on les perd en
  quittant le dernier groupe.
- **Trois sortes de cartes** : celle qui accorde un rôle, celle qui n'accorde
  rien et ne porte aucune réaction (`information`), et celle qui porte la main
  levée sans rien accorder, juste pour répondre en privé (PauseAction).

## Les choix retenus, et pourquoi

- **Une réaction plutôt qu'un bouton.** C'est ce que demandait le cahier des
  charges, et c'est plus lisible sur mobile. Le prix à payer : Discord ne permet
  aucune réponse à l'écran après un clic, d'où les confirmations en privé.
- **Le message est la clé, pas le rôle.** Mémoriser l'identifiant du message
  publié permet de **corriger un texte sans republier**, donc sans effacer les
  réactions ni reprendre les rôles. C'était l'exigence n°1 du CDC.
- **Un rôle parent par fil, jamais par carte.** Décision explicite de l'équipe :
  un réglage par carte multipliait les cas sans rien résoudre.
- **Le fichier de contenu sert à l'amorçage, pas de source de vérité.** Passé le
  démarrage, c'est Discord qui fait foi : le CDC demandait de pouvoir corriger
  un message en quelques secondes. L'import ne supprime donc jamais rien, il
  signale.
- **Un message privé n'est jamais envoyé deux fois**, ni par fil ni par carte,
  même après des années. La réservation précède l'envoi, parce que Discord
  traite deux clics rapprochés en parallèle.
- **Le rôle est accordé quoi qu'il arrive.** Si le message privé échoue, le
  membre garde son accès. C'est pourquoi les liens vitaux figurent aussi dans
  l'introduction du fil, visible sans messages privés.
- **Rien n'est supprimé sans confirmation.** `/forum fil supprimer` et
  `/forum republier` demandent `confirmer: True`, la seconde parce qu'elle
  efface les mains levées de tout un fil.
- **Une instance, un serveur, un fichier.** SQLite plutôt qu'une base distante,
  binaire statique plutôt qu'une pile à maintenir. La version précédente
  dépendait d'un Directus externe et de quarante variables d'environnement.

## Ce que le bot ne fait pas

- Il **n'attribue pas** le rôle PauseAction : cette question reste dans
  l'onboarding natif de Discord.
- Il **ne gère pas** les accès Notion. Il oriente, un humain ouvre l'accès.
- Il **ne supprime jamais** un rôle Discord ni ses porteurs, même quand la carte
  correspondante disparaît.
- Il **n'a pas d'interface de pilotage en direct** : la page web édite le
  fichier de contenu, pas l'état du bot.

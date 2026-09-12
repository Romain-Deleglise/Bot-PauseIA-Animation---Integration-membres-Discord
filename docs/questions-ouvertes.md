# Questions à trancher en réunion

> Note générée par Claude à partir des tests sur la sandbox, relue par Antoine.

## Un message privé par ville ?

Aujourd'hui, le message privé d'un fil part une seule fois par personne, à sa première réaction dans ce fil, comme le demande le CDC. Quitter tous les groupes locaux puis en rejoindre un nouveau ne renvoie rien.

Faut-il plutôt un message par groupe rejoint, par exemple avec le contact du référent de la ville ? Côté bot, le changement est petit : mémoriser l'envoi par message plutôt que par fil, et permettre un texte propre à chaque message.

## Qui écrit l'introduction d'un fil ?

Le message d'ouverture d'un fil de forum est posté par celui qui crée le fil, et ne peut plus être supprimé. Le bot, lui, publie sa propre introduction quand on republie un fil, juste en dessous : on obtient deux introductions.

Tranché à la démo : les fils sont créés à la main. Dans le TOML, `nom` et `description` peuvent être omis : le bot reprend le nom du fil sur Discord, et le message d'ouverture sert d'introduction, sans doublon.

## Un rôle retiré à la main laisse la réaction

Si quelqu'un retire un rôle directement dans Discord, la réaction 🙋 reste sur la carte : la personne doit la retirer puis la remettre pour récupérer son rôle. Le bot pourrait retirer lui-même la réaction devenue orpheline. Utile, ou négligeable ?

## Référents : texte ou mentions ?

Tranché à la démo : pseudo cliquable. Le bot convertit chaque `@pseudo` d'une carte en mention, à l'import comme dans `créer` et `modifier`, si la personne est membre du serveur. Les pseudos introuvables restent en texte et sont signalés.

## Qui crée les salons « à créer » ?

Plusieurs lignes du CDC renvoient à un salon « à créer » ou « à définir ». Le rôle est bien accordé, mais il ne donne accès à rien tant que le salon n'existe pas et n'est pas réservé à ce rôle.

## Qui administre le bot ?

Les commandes sont réservées aux rôles listés dans `MANAGE_ROLE_IDS`. Lesquels ?

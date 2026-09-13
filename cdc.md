# Cahier des charges “Forum PauseIA”

Créée le: July 20, 2026 2:40 PM
Éditée le: August 26, 2026 6:53 PM
Statut: Brouillon
Projets: Refonte Accueil et orientation (https://app.notion.com/p/Refonte-Accueil-et-orientation-39128fc94b7780a783a5cf1198c99d7b?pvs=21)

# Intro

---

Dans le discord de PauseIA, les membres n’ont pas assez de visibilité sur les projets en cours, et il y a beaucoup de friction pour s’ajouter à un projet existant. Le projet “Forum PauseIA” vise à résoudre ce problème.

Le système que nous souhaitons est celui-ci:

![image.png](Cahier%20des%20charges%20%E2%80%9CForum%20PauseIA%E2%80%9D/image.png)

Nous avons décidé d’utiliser un salon de type “forum” avec 4 fils, un pour chaque catégorie.

Nous avons également décidé d’utiliser un bot pour que les messages dans ces salons permettent de mettre en place les rôles.

# Cahier des charges

Le cahier des charges suivant explique comment le “Forum” dans son ensemble devrait fonctionner. Certains des critères concernent le Forum en lui-même, d’autres concernent le bot.

## Parcours utilisateur

L’utilisateur doit avoir initialement accès au salon et aux 4 fils. Il n’y aura pas d’autre catégorie que les 4 présentées à court terme.

Chaque fil doit avoir une illustration qui peut si possible être changée.

L’utilisateur doit voir les N messages dans chaque fil. Il peut cliquer sur une réaction, ce qui lui donnera le rôle associé. Un message peut ne pas avoir de rôle associé. Dans ce cas, pas d’emoji visible.

Dans certains salons,  chaque réaction à un message attribue automatiquement un rôle donné.  Par exemple, nous avons une catégorie “Groupes locaux” qui contient “Paris” et “Lyon”. Chaque groupe local doit donner accès à un groupe local particulier (gl-paris, gl-lyon), et attribue également le rôle groupe-local plus général.

Dans certains fils, réagir à un des messages doit provoquer l’envoi d’un message privé à l’utilisateur concerné. Si l’utilisateur réagit à plusieurs messages d’un même fil, il reçoit seulement une fois le message, peu importe le délai entre les réactions.

## Possibilité d’édition

Les admins du Discord (ou à minima, l’admin qui a créé le message correspondant) doivent pouvoir:

- éditer les descriptions des messages sans perdre le comportement ni supprimer des rôles aux utilisateurs
- ajouter des nouveaux messages et configurer un ou deux rôles associés à ce message
- supprimer des messages, puis supprimer les rôles qui étaient associés aux messages
- modifier le “statut” d’un message simplement et rapidement (disons 10s). Soit par une commande dédiée qui change le style (et uniquement le style) du message, soit en éditant facilement le texte du message lui même (en ajoutant [Inactif] au début par exemple)
- éditer la description du forum
- configurer le message associé à un fil (celui envoyé en MP par le bot). Lors de la modification du message, les utilisateurs qui ont réagi avec un des messages du fil ne sont pas notifiés.

## Contenu

Le Forum doit initialement contenir les 4 fils, chacun avec une description.

Une présentation générale du forum doit être disponible, soit dans le forum lui-même, soit ailleurs.

- Présentation du forum
    
    > 
    > 
    > 
    > Bienvenu dans le forum!
    > 
    > Vous voulez aider Pause IA mais ne savez pas où commencer? Vous êtes au bon endroit. 
    > 
    > Ici vous pouvez explorer notre structure et nos projets : qui fait quoi, où, et ce que vous pouvez rejoindre. Parcourez les différentes catégories et levez la main (🙋), vous recevrez les accès associés.
    > 
    > N'hésitez pas à contacter les référents de chaque projet / groupe en message privé si vous avez des questions.
    > 
    

Les 4 fils doivent contenir les messages suivants. Remarque: dans le tableau, les colonnes en orange sont des informations utiles pour notre équipe en interne, elles ne sont pas utiles pour la création du forum.

- **Projets**
    
    Description: `Les projets de PauseIA, en cours et à venir. Levez la main pour rejoindre celui qui vous parle, ou pour signaler que vous en faites déjà partie.`
    
    > Un projet vous parle et vous souhaitez y contribuer, ou vous en êtes déjà membre ? Levez la main 🙋 pour le rejoindre.
    Si vous avez des idées qui ne sont pas encore listées ici, vous pouvez en discuter dans #idées
    > 
    
    | Texte               | Rôle | Statut | Salon associé |
    | ------------------- | ---- | ------ | ------------- |
    | **Fresque de l’IA** |
    Référent**:** @doweee1
    ****Notre version de la Fresque de l'IA, adaptée de celle du CeSIA et inspirée de la Fresque du climat : 36 cartes pour animer un atelier collaboratif où les participants apprennent et relient les sujets.
    Il reste à créer les documents pour animateurs et une formation. Une fois déployé, le projet continuera de vivre grâce aux retours d'expérience et aux mises à jour.
    Coordonné par l’équipe *communication* | @fresque-ia | Actif | #fresque-ia |
    | **PauseAction**
    Référent: @antonin7067
    ****Chaque semaine, nous trouvons une action simple, signer, partager ou commenter, réalisable en moins de cinq minutes, sur notre groupe WhatsApp.
    Coordonné par l’équipe *communication* | @pauseaction | Actif | À définir: un nouveau salon pause-action ? Ou supprimer le pause-action actuel ? |
    | **Idées reçues**
    Référents**:** @jeannebazard @lady_mircalla
    ****Nous rassemblons les objections fréquentes, les idées reçues et les fausses promesses sur les apports et les dangers de l'IA, puis nous y répondons afin d’outiller le mouvement.
    Coordonné par l’équipe *communication* | @idées-reçues | Actif | #idées-reçues |
    | **Entraide plaidoyer**
    Référent**:** @zedioum
    ****Porter la position de PauseIA auprès des institutions et des élus. 
    Dans ce projet, vous aurez accès à des ressources et des formations pour participer au plaidoyer de l’association dans votre département.
    Coordonné par l’équipe *plaidoyer* de l’asso. Vous pouvez d’ailleurs rejoindre cette équipe si vous êtes très motivé. | @entraide-plaidoyer | Actif | #entraide-plaidoyer |
    | **Accueil et orientation**
    Référent: @rambip
    Accueillir les nouveaux membres sur le Discord et les orienter vers les projets et équipes qui leur correspondent.
    On essaie de proposer des visios à chaque nouveau membre, et c’est très motivant et enrichissant!
    Coordonné par le bureau | @accueil-orientation | Actif | #onboarding (renommer refonte-onboarding) |
    | [En structuration] **Animation Discord**
    Référent: @gyrodiot
    ****Organiser et animer des ateliers, soirées de réflexion, discussions détente, lectures et soirées de soutien.
    
    Ce groupe a pour vocation d’effectuer aussi une veille sur le bien-être et la santé mentale des membres (canal #soutien) et d’apporter des ressources spécifiques. 
    
    Toutes les autres idées sont bienvenues.
    Coordonné par le bureau | @animation-discord | Actif | #animation-discord (à créer) |
    | **Organisation des rencontres pauseIA
    Référent**: @clemence_coqcinelle
    Rejoignez ce projet pour aider à organiser des rencontres dans la vraie vie. Prochain objectif: week-end de rencontre et de formation de Pause IA.
    Coordonné par le bureau | @rencontres-pauseia | Actif | À créer |
    | [En structuration] **Partenariats créateurs de contenu
    Référent**: @flavien0525
    Identifier et solliciter des streamers et youtubeurs pour inviter Maximes Fournes sur leurs chaînes. | @partenariats-créateurs | Actif | À créer |
    | [À venir] **Création artistique humaine (Pause I-Art)**
    Rejoignez ce projet si vous avez envie de produire des illustrations humaines pour les réseaux sociaux et la communication de Pause IA.
    Coordonné par l’équipe *communication*. | @creation-artistique | Actif | À créer |
    | [À venir] **Jeu vidéo et jeu de rôle
    Référent:** @dyson4282. Il a développé un jeu multijoueur super, n’hésitez pas à lui demander le lien.
    Concevoir des jeux vidéo et jeux de rôle au service de la sensibilisation. Plusieurs personnes ont manifesté leur intérêt, mais tout reste à faire! | @jeu-pauseia | Inactif | À créer |
    | [Besoin d’aide] **Création de flyers**
    Créer des supports visuels (flyers, affiches, pancartes, pins …) pour aider les groupes locaux. Contacter @rambip ou @clemence_coqcinelle si vous voulez donner un coup de main.
    Coordonné par l’équipe *initiatives locales*. | @flyers | Actif | À créer |
    | [À venir] **Relations associations**
    Prendre contact avec différentes associations dont la mission est proche de celle de PauseIA. Leur présenter notre argumentaire, proposer différentes activités, et aussi surveiller leurs événements pour que d’autres membres puissent les rencontrer et créer de liens. | @relation-associations |  |  |
- **Équipes**
    
    Description: `Les équipes qui font tourner l'association, et les membres qui s'y investissent régulièrement. Rejoindre une équipe, c'est s'engager environ cinq heures par semaine. Levez la main pour rejoindre celle où vous voulez contribuer.`
    
    > Le cœur de l'action : nos équipes. Leurs membres s'y investissent de l'ordre de cinq heures par semaine. Explorer les équipes et levez la main 🙋 sur celle où vous voulez contribuer.
    > 
    
    | Texte             | Rôle 1 | Rôle 2 | Membres |
    | ----------------- | ------ | ------ | ------- |
    | **Communication** |
    Référent**:** @jeannebazard
    L'équipe suit l'actualité, rédige les communiqués de presse, les posts pour les réseaux sociaux, la newsletter et le blog Substack. Poste à pourvoir : responsable YouTube. | @communication | @portail-équipe | @antonin7067
    @clemence_coqcinelle
    @doweee1
    
    @marcial1572
    @flavien0525
    @hugothrow |
    | **Plaidoyer**
    Référent: @zedioum
    Porter la position de PauseIA auprès des institutions et des élus. Exemples : rédiger une prise de position, courriers aux élus, rencontrer un député… | @plaidoyer | @portail-équipe | @clemence_coqcinelle
    @anmacha
    @hugothrow
     |
    | **Tech**
    Référent: @.romain73
    L'équipe assure le support technique sur Notion, Discord et le site internet, ainsi que leurs mises à jour. | @tech | @portail-équipe | @martinkunev
     |
    | **Initiatives locales**
    Référent: @rambip
    Organiser les temps forts et mobiliser les membres autour des actions de terrain. Gère les groupes locaux. | @initiatives | @portail-équipe | @clemence_coqcinelle
    @rampip |
    | **Financement & Partenariats**
    Référent: @.romain73
    Rechercher des financements et nouer des partenariats pour soutenir l'association. | @financement | @portail-équipe | 
    @clemence_coqcinelle |
    
- **Groupes locaux**
    
    Description:`Les savoir-faire qui font avancer nos projets. Levez la main pour signaler ceux que vous pouvez mettre à disposition pour Pause IA.`
    
    > PauseIA près de chez vous. 
    Les membres des groupes locaux se retrouvent pour discuter, tracter, former et faire vivre le mouvement sur le terrain. Levez la main 🙋 pour rejoindre le vôtre et rencontrer les membres de votre région.
    Si vous ne trouvez pas de groupe local près de chez vous, vous pouvez remplir ce formulaire: [https://pauseia.notion.site/2e128fc94b7780fd94b6d35c35b2f0ac?pvs=105](https://app.notion.com/p/2e128fc94b7780fd94b6d35c35b2f0ac?pvs=21). L’équipe en charge des groupes locaux vous contactera dès qu’un groupe peut être créé près de chez vous.
    > 
    
    | Description  | Rôle 1 | Rôle 2 | Statut |
    | ------------ | ------ | ------ | ------ |
    | **Bordeaux** |
    
    Référent: @olivierloisel
    
    Pour l’instant en sous-effectif (un membre actif, deux membres potentiels). Mais ils sont motivés pour lancer des projets ! | @gl-bordeaux | @groupe-local |  |
    | **Colmar**
    [Inactif] Groupe inactif, mais n’hésitez pas si vous voulez le relancer !
    Ancien référent: @method75019 | @gl-colmar | @groupe-local | Inactif |
    | **Dordogne**
    Référente: @clemence_coqcinelle
    
    Groupe sur le département car nous sommes dans différentes villes (Périgueux, Bergerac, Sarlat). On a fait une manif et un tractage et on a fait signé quelques chartes aux candidat-es pendant la campagne pour les municipales. On aimerait organiser une fresque de la sécurité de l’IA en septembre ! | @gl-dordogne | @groupe-local |  |
    | **Grenoble**
    Référent: @nothing.45
    ****
    Groupe créé en début 2026, on a été super nombreux (5), et on a fait plein de trucs: premier mai, municipales, sessions de tractage, discussion avec la mairie, fresque. Mais après de nombreux départs il ne reste que deux membres. Rejoignez vite pour qu’on recommence ! | @gl-grenoble | @groupe-local |  |
    | **Lille**
    Référents: @anmacha
    @pogzo
    
    Ils ne sont que 3, mais ils se sont rencontrés et sont très motivés! En particulier, ils ont participé à l’anti-sommet sur l’IA | @gl-lille | @groupe-local |  |
    | TODO
    Référents: @bayes8
    @jeannebazard
    Plusieurs membres très impliqués dans PauseIA sont présents dans ce groupe, mais pas encore de fonctionnement bien défini pour le groupe local. | @gl-lyon | @groupe-local |  |
    | [Inactif] Groupe inactif (une seule personne présente), mais n’hésitez pas si vous voulez le relancer !
    Seul membre: @Flavien | @gl-melun | @groupe-local |  |
    | **Paris**
    Référent: @martinkunev
    
    Un groupe très actif: tractage, manifestations, organisation de conférences …
     | @gl-paris | @groupe-local |  |
    | **Poitiers**
    Référent: @Armelle86
    Ce groupe n’a pas encore eu l’occasion d’organiser des évènements locaux, mais il y a déjà 3 membres. | @gl-poitiers | @groupe-local |  |
    | **Toulouse**: 
    Référents: @bizouche @totoss0901 @Eloise_b_
    
    Groupe créé en début 2026, on a des membres sur le discord et de membre en dehors. On a créé un groupe WhatsApp qui est plus accessible que discord pour les événements. On a déjà organisé un débat, un tractage et une fresque de la sécurité de l'IA. | @gl-toulouse | @groupe-local |  |
    | **Marseille**
    
    Groupe en pleine construction. Vous pouvez contacter @11mishka11. qui habite vers aix les bains. | @gl-marseille | @groupe-local |  |
    
- **Compétences**
    
    Description: `Les groupes Pause IA près de chez vous. Levez la main pour rejoindre le vôtre et rencontrer les membres de votre ville.`
    
    > Vous souhaitez porter les valeurs de PauseIA en mettant une de vos compétences à disposition ? Nul besoin d'un gros engagement. Levez la main 🙋 : vous serez rattaché à cette compétence, prêt à donner un coup de main quand vous le pouvez.
    > 
    
    | Description                                                                                                                                                                                                                                 | Rôle                  |
    | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
    | **Aide à la création**                                                                                                                                                                                                                      |
    | Vous voulez donner un coup de main pour créer du contenu militant (relecture, sous-titrage, communication, conseils techniques…).                                                                                                           | @comp-aide-creation   |
    | **Artivisme**                                                                                                                                                                                                                               |
    | Vous pratiquez un art et voulez le mettre au service de la cause. Exemples : photo, illustration, mise en scène, tournage vidéo, chant, motion design, sculpture, poésie, danse…                                                            | @comp-artivisme       |
    | **Créa graphisme**                                                                                                                                                                                                                          |
    | Vous savez créer, choisir et agencer des éléments graphiques. Vous avez une bonne culture de l'image et des bases en maquette, mise en page ou retouche photo. Exemples : concevoir un logo, un flyer, proposer une initiation à Photoshop… | @comp-graphisme       |
    | **Créa jeux vidéo**                                                                                                                                                                                                                         |
    | Vous savez programmer ou concevoir des jeux vidéo. Exemples : penser un level design, manier Unreal Engine, Unity, RPG Maker…                                                                                                               | @comp-jeux-vidéo      |
    | **Créa photo /vidéo**                                                                                                                                                                                                                       |
    | Vous savez manier un appareil photo et/ou une caméra. Exemples : portraits, clichés pris lors d'un événement, enregistrement d’une conférence…                                                                                              | @comp-photo-video     |
    | **Créa son**                                                                                                                                                                                                                                |
    | Vous savez composer des musiques, réaliser des bruitages et mixer. Vous avez une bonne culture musicale ou des bases en composition et prise de son. Exemples : habillage sonore d'une vidéo, jingle, montage audio…                        | @comp-crea-son        |
    | **Sensibilisation**                                                                                                                                                                                                                         |
    | Vous savez animer et présenter pour faire connaître les enjeux de l'IA. Vous avez déjà organisé des ateliers ou évènements similaires.                                                                                                      | @comp-sensibilisation |
    | **Support et vulgarisation scientifique en IA**                                                                                                                                                                                             |
    | Vous pouvez apporter une expertise sur le fonctionnement de l'IA et savez l'expliquer simplement.                                                                                                                                           | @com-vulga-ia         |
    | **Support et vulgarisation scientifique en Sécurité de l’IA**                                                                                                                                                                               |
    | Vous pouvez apporter une expertise sur les risques et la sécurité de l'IA.                                                                                                                                                                  | @comp-vulga-sécurité  |
    | **Support technique en IA**                                                                                                                                                                                                                 |
    | Vous savez mettre en œuvre des outils et des solutions techniques liés à l'IA.                                                                                                                                                              | @comp-tech            |
    

## Tests

Tester au moins un message de chaque salon, et vérifier tous les rôles donnés.

## Questions d’organisation

Est-ce que tous les rôles doivent être créés avant la configuration du bot ?
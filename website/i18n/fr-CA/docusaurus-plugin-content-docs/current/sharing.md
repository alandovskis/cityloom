---
id: sharing
title: Partager un aménagement
sidebar_position: 3
---

# Partager un aménagement

Un lien de partage permet à quelqu’un d’autre de consulter l’un de vos projets tel qu’il est maintenant, sans compte et
sans pouvoir le modifier. Cette page explique comment en créer un et en ouvrir un, et ce que CityLoom conserve, pour qui
et pendant combien de temps.

## Créer un lien

Sur la page d’accueil, choisissez **Partager** dans la rangée d’un projet. La page de partage
(`share.html?project=<id>`) explique ce qu’un lien montre; choisissez **Créer un lien de partage**.

- Vous devez être connecté : le serveur conserve la copie partagée pour votre compte. Sans connexion, la page le dit et
  ne crée aucun lien.
- Le lien s’affiche dans un champ que vous pouvez sélectionner, avec **Copier le lien** et le jour où il cessera de
  fonctionner. Les liens que vous avez créés y sont de nouveau listés sur le même appareil, du plus récent au plus ancien.
- **Supprimer le lien** supprime la copie partagée du serveur avant son expiration. Quiconque ouvre le lien ensuite
  apprend que la personne qui l’a partagé l’a supprimé.
- Un compte peut avoir au plus 50 liens qui fonctionnent encore et créer au plus 100 liens en 24 heures (les liens
  supprimés comptent aussi), et une copie partagée pèse au plus 1 Mo.

## Ouvrir un lien

Un lien ressemble à `https://…/share.html#<jeton>`. L’ouvrir ne demande aucun compte : la page récupère la copie
partagée, la vérifie, la conserve dans ce navigateur comme une copie à part et ouvre sa carte en lecture seule
(`view=1`), comme tout aménagement ouvert en lecture seule : tout peut être consulté, rien ne peut être modifié. La copie
n’apparaît pas parmi vos projets, n’est jamais envoyée au serveur avec votre travail et est remplacée si vous rouvrez le
même lien. Un navigateur conserve au plus dix de ces copies; la plus ancienne part en premier.

Pour modifier un aménagement partagé, choisissez **Modifier une copie** dans l’avis sous la barre du haut. Aucun compte n’est nécessaire : l’aménagement devient un nouveau projet à vous, avec le même nom, la même
zone, les mêmes modifications et les mêmes règles de zone, affiché sur la page d’accueil et synchronisé avec votre
travail comme tout autre projet, et la page propose **Ouvrir votre copie**. Votre projet est indépendant : ce que vous y
changez n’atteint jamais l’aménagement partagé, la personne qui l’a créé ni quiconque ouvre le lien, et rouvrir le lien
remplace la copie en lecture seule, pas votre projet.

Quand un lien ne peut pas être ouvert, la page dit pourquoi et ne conserve rien : le lien est incomplet, aucun
aménagement partagé n’a ce lien, il a expiré, la personne qui l’a créé l’a supprimé, il a été créé par une version plus
récente de CityLoom, il est endommagé, le réseau ou le serveur est injoignable, ou l’espace de stockage du navigateur a
refusé la copie.

## Confidentialité et conservation

- **Ce qui est partagé** : un instantané du projet entier au moment où le lien a été créé : son nom, sa zone, les
  modifications de ses rues et de ses jonctions, et ses règles de zone. Ni sa chronologie, ni rien de votre compte, de
  vos réglages ou de vos autres projets. Les modifications apportées au projet ensuite ne sont pas dans le lien : créez-en
  un nouveau pour les partager.
- **Qui peut le consulter** : quiconque a le lien, et personne d’autre. Le secret du lien (256 bits aléatoires) se trouve
  dans la partie de l’adresse qui suit `#`, que les navigateurs n’envoient jamais à un serveur dans une adresse, ni à un
  autre site dans un `Referer`. Pour récupérer l’aménagement, la page de partage envoie le secret au seul serveur de
  CityLoom, dans un en-tête de requête (`Share-Token`) plutôt que dans une adresse, que les mandataires en chemin
  peuvent journaliser; le serveur s’en sert pour retrouver et déchiffrer l’instantané, et ne le conserve pas. Traitez
  un lien comme une clé : la personne à qui vous le donnez peut le transmettre.
- **Ce que le serveur conserve** : l’instantané, chiffré avec une clé tirée du secret du lien, de sorte que le serveur ne
  détient rien qu’il puisse lire sans le lien; une empreinte du secret pour le retrouver; votre compte, comme créateur;
  et les dates de création et d’expiration. Le secret lui-même n’est jamais conservé ni inscrit dans les journaux du
  serveur, qui n’enregistrent aucun en-tête de requête.
- **Pendant combien de temps** : 90 jours à partir de la création du lien. L’instantané est alors supprimé, et le lien
  indique qu’il a expiré. Un lien supprimé ou expiré continue de le dire pendant un an; ensuite, il indique qu’aucun
  aménagement partagé n’a ce lien.
- **Suppression** : vous pouvez supprimer un lien à tout moment depuis la page de partage de l’appareil où vous l’avez
  créé. Supprimer un projet sur la page d’accueil supprime d’abord les liens qui en ont été créés sur cet appareil (vous
  devez être connecté avec le compte qui les a créés); si l’un d’eux ne peut pas l’être, la page dit pourquoi et
  propose de supprimer le projet quand même, ses liens fonctionnant alors, et comptant parmi les 50 liens qu’un compte peut
  avoir, jusqu’à leur expiration : ils ne sont plus listés nulle part. Un projet supprimé sur un autre appareil laisse fonctionner les liens qui en ont été créés sur
  celui-ci jusqu’à leur expiration. Supprimer votre compte supprime d’un coup tous les liens que vous avez créés; ces liens indiquent alors qu’aucun
  aménagement partagé ne les a.
- **Sur l’appareil de la personne qui l’ouvre** : la copie est conservée dans l’espace de stockage de ce navigateur
  seulement, jusqu’à ce qu’elle soit remplacée, que la limite de dix copies la pousse dehors ou que les données du site
  soient effacées.

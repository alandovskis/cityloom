---
id: projects
title: Projets
sidebar_position: 2
---

# Projets

Votre travail dans CityLoom est conservé dans des projets. Un projet a un nom et un lieu : sa carte montre les rues et
les jonctions de ce lieu, et ses modifications de rues, ses modifications de jonctions et ses règles de zone
n’appartiennent qu’à lui. Deux projets peuvent modifier la même rue de deux façons différentes.

## La liste des projets

La page d’accueil (`web/index.html`) présente vos projets sous **Vos projets**, le plus récemment modifié en premier.
Chaque ligne indique le nom du projet, son lieu et le jour de sa dernière modification.

- **Nouveau projet** : tapez un nom sous **Nom du projet** et choisissez **Créer le projet**. Un nom compte de 1 à
  80 caractères. Le lieu d’un nouveau projet est le Plateau-Mont-Royal, à Montréal, le seul lieu offert pour l’instant;
  le lieu d’un projet ne change pas.
- **Ouvrir** : choisissez le nom du projet. Sa carte s’ouvre (`map.html?project=<id>`), et les rues et les jonctions
  que vous ouvrez à partir de celle-ci sont modifiées dans ce projet.
- **Renommer** : choisissez **Renommer**, tapez le nouveau nom et choisissez **Enregistrer le nom**. Échap ou
  **Annuler** le laisse tel quel.
- **Supprimer** : choisissez **Supprimer**. La ligne demande d’abord une confirmation; **Supprimer le projet** retire
  le projet et tout ce qu’il contient. Cette action est irréversible.

La carte et les éditeurs portent le projet dans leur adresse. Une page ouverte sans projet ouvre celui que vous avez
ouvert en dernier. Si ce projet n’est plus là, par exemple parce qu’il a été supprimé sur un autre appareil, la page
revient à la liste, qui indique **Ce projet n’est plus ici.**

## Le travail d’avant les projets

Le travail conservé dans le navigateur par une version de CityLoom sans projets est déplacé, à la première ouverture de
la liste, dans un projet nommé **Premier projet**, dans le même lieu qu’avant. Il s’ouvre comme avant : vous n’avez
rien à faire.

## Les projets sur plusieurs appareils

Si vous êtes connecté et avez choisi **Stocker mon travail sur le serveur** sur la page Compte, vos projets suivent
votre travail : chaque appareil sur lequel vous vous connectez reçoit tous les projets, chacun avec ses propres
modifications et règles. Un projet créé, renommé ou supprimé sur un appareil l’est aussi sur les autres à leur
prochaine synchronisation. Quand vous choisissez le stockage sur le serveur, le message qui suit la fusion compte les
lieux (rues, jonctions, règles de zone), et non les projets eux-mêmes.

Le travail qu’une version antérieure de CityLoom a stocké sur le serveur appartient au premier projet.

## L’archive de conception

**Exporter la conception**, sur la page Compte, enregistre le travail du projet que vous avez ouvert en dernier.
**Importer une conception** ajoute le travail d’une archive à ce projet.

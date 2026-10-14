---
id: intro
slug: /
title: Introduction
sidebar_position: 1
---

# Introduction

CityLoom est un éditeur de rues, de jonctions et de cartes de ville. Il est écrit en Rust, compilé en WebAssembly et
dessiné avec [Leptos](https://leptos.dev/) 0.8 (rendu côté client). Il s’exécute dans le navigateur; il n’a pas de
serveur d’applications : un serveur de fichiers statiques sert ses fichiers.

Les rues viennent d’[OpenStreetMap](https://www.openstreetmap.org/). Les règles, les taux et les seuils du catalogue (ce
à quoi une vérification compare une rue) sont des valeurs provisoires synthétiques, non une norme.

L’application est offerte en anglais et en français canadien.

## Les quatre pages

| Page     | Fichier                 | À quoi elle sert                                                                                                                                               |
| -------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Accueil  | `web/index.html`        | Chercher un lieu : trouver une ville ou un quartier et charger ses rues.                                                                                       |
| Carte    | `web/map.html`          | Les rues et les jonctions de la ville sur un fond de carte. Ouvrir un lieu pour le modifier et déclarer des zones avec des règles (le panneau Règles de zone). |
| Rue      | `web/street.html`       | Modifier la coupe transversale d’une rue : réorganiser ses éléments, lire les vérifications et la voir en 2,5D.                                                |
| Jonction | `web/intersection.html` | Modifier une jonction en plan : ses branches, ses coins, ses passages pour piétons et sa signalisation.                                                                   |

Les pages de la rue et de la jonction s’ouvrent depuis la carte : la carte donne à chaque lieu une adresse comme
`?street=7` ou `?junction=3`. Une page ouverte ainsi modifie un lieu de la ville, et ses modifications sont conservées
dans le stockage du navigateur.

## Pour poursuivre

- [Démarrage](getting-started.md) construit et sert l’application sur votre machine.
- [Architecture](architecture.md) explique comment le code est organisé.

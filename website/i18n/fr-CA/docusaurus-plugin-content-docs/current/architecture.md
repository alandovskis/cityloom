---
id: architecture
title: Architecture
sidebar_position: 3
---

# Architecture

## Tranches verticales

Le code est organisé par fonctionnalité, un dossier par tranche, et non par couche. Le nouveau code va dans la tranche
qui possède la fonctionnalité.

| Tranche | Contenu |
| --- | --- |
| `street/` | L’éditeur de rue. |
| `junction/` | L’éditeur de jonction. |
| `map/` | La carte de la ville sur un fond de carte MapLibre, et le panneau Règles de zone. |
| `shell/` | Le balisage commun aux pages : menu, changement de langue, raccourcis. |
| `city/` | La ville : sa disposition, son stockage, sa projection sur la Terre et le document des modifications. |
| `place/` | L’origine d’une ville : la recherche de lieu, ainsi que la récupération et l’importation des routes d’une zone. |
| `restriction/` | Les règles déclarées sur une zone de la carte. |
| `shared/` | Le noyau : catalogue, unités, ports, traductions. Il ne doit dépendre d’aucune tranche. |

Une tranche peut utiliser le `model` d’une autre tranche, `city::store`, `city::binding`, `city::projection`,
`restriction::store`, `shell::Target` et `shared`. Elle ne doit jamais accéder au modèle de vue, aux vues ni aux textes
d’une autre tranche.

## MVVM dans une tranche

Chaque tranche d’édition contient :

- `model.rs` : les règles d’édition. Aucun DOM, aucun signal, aucune entrée-sortie. Une modification renvoie un `bool`,
  et un refus en donne la raison.
- `vm.rs` : le modèle de vue. Il expose des accesseurs réactifs prêts à présenter et reçoit des commandes.
- `text.rs` : les mots dits à un lecteur d’écran.
- Les vues Leptos, qui se lient à un modèle de vue et lui transmettent les événements. Elles ne décident de rien.
- `tests.rs`.

Le dessin est produit par des fonctions pures qui renvoient des chaînes SVG, placées avec `inner_html`. La logique du
pointeur et du clavier est faite de machines à états pures et testées. La page Carte fait exception : elle dessine sur un
fond de carte MapLibre, sans SVG.

## Ports

Un modèle de vue n’atteint le navigateur que par des ports, définis dans `shared/ports.rs` avec des doublures pour les
tests : `Announcer`, `Storage`, `Scheduler`, `Mapper`, `Fetcher` (HTTP), `Importer` (le lecteur OpenStreetMap) et
`Navigator`. `shared/platform.rs` en est le côté navigateur. C’est ainsi que les modèles de vue sont testés en natif,
avec `test_ports()`, sans navigateur.

## Traduction

Le modèle ne contient jamais de texte. Il exprime les choses par des données `Said` (une clé de message et des
arguments) que la langue courante met en mots; un historique suit donc un changement de langue. Chaque tranche qui
possède des messages a `i18n/en.ftl` et `i18n/fr.ftl` (Fluent), et un test échoue quand les deux diffèrent. Les termes
français sont dans `docs/glossary-fr-ca.md`.

## Modules WebAssembly

L’espace de travail compte trois caisses en plus de l’éditeur lui-même :

- `cityloom_editor` (la caisse racine) est l’éditeur, construit en WebAssembly et piloté par `wasm-bindgen`.
- `osm_network` est le type de réseau simple (serde seulement).
- `osm_import` exécute [osm2streets](https://github.com/a-b-street/osm2streets) et remplit le réseau. Il est construit
  comme son propre module WebAssembly, `web/pkg/osm_import*`, que `web/city.js` charge à la première ouverture d’un
  lieu. Cela garde osm2streets hors du module de l’éditeur.

`city/import.rs` transforme un réseau en disposition de la ville : les routes deviennent des rues, et la rencontre de
trois à cinq branches que l’éditeur de jonction sait dessiner devient une jonction.

## JavaScript

La logique appartient à Rust. Les scripts de page dans `web/` (`app.js`, `junction.js`, `map.js`) se contentent de
charger le module, d’appeler `mount_*` et de monter le shell. La seule exception est `web/basemap.js`, l’adaptateur qui
possède la carte MapLibre.

## Persistance

Une page ouverte depuis la carte modifie un lieu de la ville. Ses modifications sont réécrites par `CityStore`, avec un
délai antirebond de 250 ms, et enregistrées quand la page est masquée. Les modifications de toutes les zones tiennent
dans un seul document, `cityloom-edits`, dont les enregistrements sont indexés d’après ce dont le lieu a été fait
(identifiants de nœuds et de voies OpenStreetMap); une modification suit donc son lieu dans une autre zone qui le
contient.

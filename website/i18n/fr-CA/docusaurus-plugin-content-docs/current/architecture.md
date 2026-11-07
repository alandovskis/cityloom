---
id: architecture
title: Architecture
sidebar_position: 3
---

# Architecture

## Tranches verticales

Le code est organisé par fonctionnalité, un dossier par tranche, et non par couche. Le nouveau code va dans la tranche
qui possède la fonctionnalité.

| Tranche        | Contenu                                                                                                                           |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `street/`      | L’éditeur de rue.                                                                                                                 |
| `junction/`    | L’éditeur de jonction.                                                                                                            |
| `map/`         | La carte de la ville sur un fond de carte MapLibre, le panneau Règles de zone et les vérifications du réseau.                     |
| `shell/`       | Le balisage commun aux pages : menu, changement de langue, raccourcis, le menu Se connecter.                                      |
| `account/`     | La page Compte : connexions associées, lieu de conservation du travail, suppression du compte.                                    |
| `city/`        | La ville : sa disposition, son stockage, sa projection sur la Terre et le document des modifications.                             |
| `place/`       | L’origine d’une ville : la recherche de lieu, ainsi que la récupération et l’importation des routes d’une zone.                   |
| `restriction/` | Les règles déclarées sur une zone de la carte.                                                                                    |
| `sync/`        | Conserve le travail d’une personne connectée sur le serveur, quand elle l’a choisi, et le fusionne entre ses appareils.           |
| `archive/`     | L’archive de conception : exportation et importation des modifications et des règles de zone d’une personne dans un seul fichier. |
| `shared/`      | Le noyau : catalogue, unités, ports, traductions. Il ne doit dépendre d’aucune tranche.                                           |

Une tranche peut utiliser le `model` d’une autre tranche, `city::store`, `city::binding`, `city::projection`,
`restriction::store`, `shell::Target` et `shared`; les magasins et les éditeurs utilisent aussi les types de `sync::model` et `mark`, et tout le reste accède à la synchronisation par `sync::hub`. Elle ne doit jamais accéder au modèle de vue, aux vues ni aux textes
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
tests : `Announcer`, `Storage`, `Scheduler`, `Mapper`, `Fetcher` (HTTP), `Importer` (le lecteur OpenStreetMap), `Account` (l’état de connexion et l’API du compte), `WorkSync` (le travail stocké sur le serveur) et
`ArchiveFiles` (une archive de conception remise sous forme de fichier, et un fichier choisi) et `Navigator`. `shared/platform.rs` en est le côté navigateur. C’est ainsi que les modèles de vue sont testés en natif,
avec `test_ports()`, sans navigateur.

## Traduction

Le modèle ne contient jamais de texte. Il exprime les choses par des données `Said` (une clé de message et des
arguments) que la langue courante met en mots; un historique suit donc un changement de langue. Chaque tranche qui
possède des messages a `i18n/en.ftl` et `i18n/fr.ftl` (Fluent), et un test échoue quand les deux diffèrent. Les termes
français sont dans `docs/glossary-fr-ca.md`.

## Modules WebAssembly

L’espace de travail compte quatre caisses :

- `cityloom_editor` (la caisse racine) est l’éditeur, construit en WebAssembly et piloté par `wasm-bindgen`.
- `osm_network` est le type de réseau simple (serde seulement).
- `osm_import` exécute [osm2streets](https://github.com/a-b-street/osm2streets) et remplit le réseau. Il est construit
  comme son propre module WebAssembly, `web/pkg/osm_import*`, que `web/city.js` charge à la première ouverture d’un
  lieu. Cela garde osm2streets hors du module de l’éditeur.
- `cityloom_server` est le serveur : il sert `web/`, connecte les gens avec OpenStreetMap, Google, Microsoft ou Apple, et
  conserve les comptes dans Postgres. Il n’est pas construit en WebAssembly.

`city/import.rs` transforme un réseau en disposition de la ville : les routes deviennent des rues, et la rencontre de
trois à cinq branches que l’éditeur de jonction sait dessiner devient une jonction.

## Comptes et serveur

On peut se connecter avec OpenStreetMap, Google, Microsoft ou Apple (OpenID Connect, code d’autorisation avec PKCE).
Une identité est le fournisseur et son sujet; un courriel ne sert jamais à trouver un compte ni à en joindre deux. Les
fournisseurs s’associent à la main, depuis la page Compte.

Tout ce que le serveur conserve sur une personne est chiffré : chaque champ est scellé avec AES-256-GCM sous une clé
propre à cette personne, elle-même scellée sous une clé maîtresse que détient l’environnement du serveur, et liée à sa
table, à sa colonne et à sa ligne. Les identités se retrouvent par un HMAC du fournisseur et du sujet. La suppression
d’un compte en retire aussitôt les lignes; les copies dans les sauvegardes disparaissent à l’expiration de celles-ci.
Sans connexion, le site fonctionne exactement comme avant, et il fonctionne encore quand le volet des comptes du
serveur est indisponible.

## Modifications dans le nuage

Une personne qui a choisi « Stocker mon travail sur le serveur » voit ses modifications de rues et de jonctions, ses
règles de zone et la zone choisie conservées sous son compte, et chaque appareil sur lequel elle se connecte les reçoit.
Le stockage du navigateur reste la copie de travail : les éditeurs fonctionnent hors ligne et rattrapent leur retard au
retour du réseau. Chaque lieu est un enregistrement dont la révision vient du compteur du compte; un appareil demande ce
qui dépasse la dernière révision qu’il a vue et envoie ce qu’il a modifié. Quand deux appareils ont modifié le même
lieu, la modification la plus récente l’emporte sur les deux, et l’appareil dont la modification a perdu le dit. Le
contenu des enregistrements est chiffré comme tout ce que le serveur conserve, lié à son genre, à sa clé et à son
compte; les clés des lieux (identifiants OpenStreetMap), les identifiants de règles et l’heure des modifications ne le
sont pas, parce que le serveur les compare et les trie. Cesser le stockage supprime la copie sur le serveur et garde
celle du navigateur; supprimer le compte supprime aussi la copie sur le serveur. Les préférences (thème, langue,
panneaux) restent propres à chaque appareil.

## JavaScript

La logique appartient à Rust. Les scripts de page dans `web/` (`home.js`, `map.js`, `app.js` pour la page de la rue et
`junction.js`) servent au démarrage : ils chargent le module, appellent `mount_*` et montent le shell. Les scripts de la
rue et de la jonction préparent aussi les motifs de hachures et enregistrent les modifications de la page à `pagehide`,
et `map.js` crée le fond de carte. La seule exception est `web/basemap.js`, l’adaptateur qui possède la carte MapLibre.

## Persistance

Une page ouverte depuis la carte modifie un lieu de la ville. Ses modifications sont réécrites par `CityStore`, avec un
délai antirebond de 250 ms, et enregistrées quand on quitte la page. Les modifications de toutes les zones tiennent
dans un seul document, `cityloom-edits`, dont les enregistrements sont indexés d’après ce dont le lieu a été fait
(identifiants de nœuds et de voies OpenStreetMap); une modification suit donc son lieu dans une autre zone qui le
contient.

## L’archive de conception

La page Compte peut exporter le travail de conception d’une personne dans un seul fichier JSON et importer un tel
fichier. Le format est décrit dans `src/archive/model.rs`; il porte un `format_version` entier (1) et un `kind` valant
`cityloom-design`, la version de l’application, l’heure, ce sur quoi la conception a été faite (zone, somme de
contrôle de la tuile, version de l’importeur), le document `cityloom-edits` (lu à partir de la version 4 et écrit à la
version que son contenu exige) et le document `cityloom-restrictions`. Il ne contient ni le document de
synchronisation, ni rien du compte, ni la langue et le thème, ni les réseaux de rues téléchargés. Le fichier se nomme
`*.cityloom-design.json`, jamais un simple `.cityloom`, qui est l’archive de zone que construit `cityloom_prepare`.

Une importation vérifie tout le fichier avant d’écrire quoi que ce soit. Un fichier qui n’est pas du JSON, qui n’est pas
une archive de conception CityLoom, qui est endommagé, qui dépasse 10 Mo, qui vient d’un format plus récent ou qui
contient des modifications ou des règles d’une version plus récente est refusé avec un message qui le dit et qui précise
que rien n’a été modifié. Les lieux absents de l’appareil sont ajoutés; un lieu présent qui diffère n’est remplacé que si
la personne choisit « Utiliser ceux de l’archive » pour cette importation. Des modifications ou des règles de l’appareil
qui sont endommagées ou d’une version plus récente arrêtent l’importation. Les champs inconnus sont ignorés à la
lecture, et `notes` et `scenarios` sont des noms réservés que la version 1 n’écrit ni ne lit. Les enregistrements
importés sont marqués pour la synchronisation comme toute modification.

---
id: project-layout
title: Organisation du dépôt
sidebar_position: 4
---

# Organisation du dépôt

Cette page indique où se trouve chaque chose dans le dépôt. [Architecture](architecture/index.md) explique comment le code
qu’ils contiennent s’assemble.

## Le premier niveau

| Dossier    | Contenu                                                                                                                                                                                                                                                                                                                |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/`  | Toutes les caisses Rust (voir [Les caisses](#les-caisses)). Le `Cargo.toml` de la racine n’est que l’espace de travail : il énumère `crates/*` et fixe le profil de publication.                                                                                                                                       |
| `web/`     | Ce que le serveur envoie au navigateur : les pages, leurs scripts de démarrage, les feuilles de style, les polices et les icônes (voir [`web/`](#web)).                                                                                                                                                                |
| `e2e/`     | Les tests Playwright dans le navigateur : les spécifications dans `tests/`, de petits extraits OpenStreetMap dans `fixtures/`, et `tiers.json`, qui place chaque spécification dans un seul niveau (interface, intégration ou bout en bout).                                                                           |
| `scripts/` | Les scripts shell et Node que lancent le `justfile` et l’intégration continue : construction, déploiement, vérification de la version. `scripts/ci/` contient le sélecteur de changements de l’intégration continue et ses règles (`spec-map.json`); `scripts/include/tests/` les teste.                               |
| `website/` | Cette documentation (Docusaurus). Les pages anglaises sont dans `website/docs/`, les françaises dans `website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`.                                                                                                                                                     |
| `docs/`    | Des documents de travail absents de ce site : le glossaire français (`glossary-fr-ca.md`) et les spécifications de conception et les plans de mise en œuvre dans `superpowers/`. Le modèle de données est dessiné sur ce site : [Modèle de données : rues et jonctions](architecture/data-model-streets-junctions.md). |
| `deploy/`  | Le `Dockerfile` de l’image que `just deploy` téléverse vers Railway. Rien n’y est compilé.                                                                                                                                                                                                                             |
| `.github/` | Les flux de travail de l’intégration continue, les formulaires de billets, les étiquettes (`labels.yml`), les règles qui changent les réponses d’un billet en étiquettes, les réglages de Dependabot et l’action `browser-tests` que partagent tous les niveaux de tests dans le navigateur.                           |
| `.claude/` | Les agents qu’utilise Claude Code, comme celui qui fusionne et déploie les demandes de fusion.                                                                                                                                                                                                                         |

Les fichiers du premier niveau :

- `Cargo.toml` et `Cargo.lock` : l’espace de travail et ses dépendances verrouillées.
- `justfile` : toutes les tâches; lancez `just` pour les lister.
- `package.json` : ce que les pages chargent de npm à l’exécution (MapLibre), copié dans `web/vendor/` par la
  construction.
- `.env.example` : toutes les variables que lit le serveur, avec la façon de créer chaque clé.
- `README.md`, et `CLAUDE.md`, avec un autre dans la plupart des dossiers : des notes pour les personnes et les agents de
  programmation qui travaillent dans cette partie du code.
- `DESIGN.md`, `PRODUCT.md` et `.impeccable/` : le système de conception et la description du produit que lisent les
  outils de conception.
- `rustfmt.toml`, `.prettierrc.json` et `.prettierignore` : les règles de mise en forme (`just format`).
- `sonar-project.properties` : les réglages de SonarCloud.

### Produits par les outils, absents de git

| Chemin                    | Produit par         | Contenu                                                                                  |
| ------------------------- | ------------------- | ---------------------------------------------------------------------------------------- |
| `target/`                 | `cargo`             | Les produits de la construction.                                                         |
| `.tools/`                 | `just setup`        | `wasm-bindgen-cli`, à la version fixée dans `crates/cityloom_editor/Cargo.toml`.         |
| `node_modules/`           | `just setup`        | MapLibre, d’après `package.json`.                                                        |
| `web/pkg/`                | `just build`        | Les modules WebAssembly et leur code de liaison JavaScript.                              |
| `web/vendor/`             | `just build`        | Le script, le travailleur et la feuille de style de MapLibre, copiés de `node_modules/`. |
| `web/data/metro/`         | `just prepare`      | Les tuiles du réseau de la région métropolitaine (`<x>_<y>.json`) et leur `index.json`.  |
| `data/`                   | `just prepare`      | L’extrait du Québec de Geofabrik et l’archive de zone empaquetée, `montreal.citymodel`.  |
| `e2e/fixtures/generated/` | `just e2e-fixtures` | Les données de test changées en tuiles du réseau.                                        |
| `website/build/`          | `just docs-build`   | Ce site, construit.                                                                      |

## Les caisses

L’espace de travail compte sept caisses, toutes dans `crates/`. Celles qui s’exécutent dans le navigateur sont
construites en WebAssembly.

| Caisse                | S’exécute dans                  | Contenu                                                                                                                                                                                                                                                                                             |
| --------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cityloom_editor`     | Le navigateur                   | L’éditeur : la logique et les vues de chaque page. Son `Cargo.toml` porte la version du produit, que chaque demande de fusion qui modifie l’application augmente.                                                                                                                                   |
| `cityloom_ui`         | Le navigateur                   | Les commandes que partagent les pages de l’éditeur : le groupe de boutons radio, les boutons plus et moins et leur champ numérique, la section des détails et la question posée avant de faire quelque chose. Elle ne contient aucun texte et n’utilise aucune autre caisse de l’espace de travail. |
| `cityloom_model`      | Le navigateur, le constructeur  | Le réseau de rues simple que produit `citymodel_builder` et que lit l’éditeur (serde seulement).                                                                                                                                                                                                    |
| `cityloom_report`     | Le navigateur                   | Le rapport d’examen de la conception sous forme de données simples (serde seulement). L’éditeur le remplit et le remet au générateur de PDF.                                                                                                                                                        |
| `cityloom_report_pdf` | Le navigateur                   | Le générateur de PDF. C’est un module distinct, `web/pkg/cityloom_report_pdf*`, que `web/report.js` charge la première fois qu’un rapport est demandé, pour que sa lourde dépendance (krilla) reste hors du module de l’éditeur. La construction échoue si l’éditeur le lie.                        |
| `cityloom_server`     | Le serveur                      | Le serveur : il envoie `web/`, connecte les gens et conserve les comptes et le travail stocké dans Postgres. `migrations/` contient le schéma de la base de données, `tests/` ses tests d’intégration.                                                                                              |
| `citymodel_builder`   | Le poste d’un développeur, l’IC | La préparation des données derrière `just prepare` et `just pack` : il télécharge l’extrait, l’importe une seule fois avec osm2streets (`src/importer/`), découpe le réseau en tuiles et les empaquette dans une archive `.citymodel` (`src/archive/`), que le serveur décompresse.                 |

## Les dossiers de l’éditeur

Le code source de l’éditeur, `crates/cityloom_editor/src/`, compte un dossier par fonctionnalité (une tranche).
[Architecture](architecture/index.md) indique quelle tranche peut utiliser laquelle.

| Dossier        | Contenu                                                                                                                                           |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `street/`      | L’éditeur de rue.                                                                                                                                 |
| `junction/`    | L’éditeur de jonction, avec sa géométrie, son dessin en 2,5D (`iso/`), ses arrêts et ses feux.                                                    |
| `map/`         | La carte de la ville dessinée par MapLibre, le panneau Règles de zone et la section Vérifications du réseau.                                      |
| `shell/`       | Ce que partagent toutes les pages : le menu, le changement de langue, les raccourcis et le menu Se connecter.                                     |
| `account/`     | La page Compte.                                                                                                                                   |
| `city/`        | La ville : sa disposition, son stockage, sa projection sur la Terre, le document des modifications et l’importation d’un réseau.                  |
| `place/`       | L’origine d’une ville : la recherche de lieu et la récupération des rues d’une zone.                                                              |
| `restriction/` | Les règles déclarées sur une zone de la carte.                                                                                                    |
| `project/`     | Les projets : la liste de la page d’accueil et le projet dans lequel une page est ouverte.                                                        |
| `standards/`   | Les normes de conception propres à un projet : des seuils locaux que les vérifications de la page de rue utilisent à la place des seuils de base. |
| `timeline/`    | La chronologie de conception : les jalons nommés du travail d’un projet.                                                                          |
| `sync/`        | Conserve le travail d’une personne connectée sur le serveur, quand elle l’a choisi, et le fusionne entre ses appareils.                           |
| `archive/`     | L’archive de conception : exportation et importation du travail d’une personne dans un seul fichier.                                              |
| `shared/`      | Le noyau : catalogue, unités, ports, traductions, outils de test. Il ne dépend d’aucune tranche.                                                  |

Dans une tranche d’édition, `model.rs` contient les règles, `vm.rs` le modèle de vue, `text.rs` les mots destinés à un
lecteur d’écran, `tests.rs` les tests, et `i18n/en.ftl` et `i18n/fr.ftl` les messages; les vues sont à côté. Dans les
trois plus grandes tranches, `street/`, `junction/` et `map/`, chaque couche a son propre dossier, `model/`, `vm/` et
`view/` (avec `model/mod.rs`, `vm/mod.rs` et `view/mod.rs` à la place de `model.rs`, `vm.rs` et `view.rs`, et les tests
dans `view/tests.rs`); seuls `mod.rs`, `i18n/` et `CLAUDE.md` restent à la racine de la tranche. Cinq fichiers se
trouvent au sommet de `src/` :

- `lib.rs` : les points d’entrée qu’appellent les pages (`mount_*`) et le registre des messages de toutes les tranches.
- `i18n_guard.rs` : un test qui échoue quand un fichier traduit contient ses propres mots au lieu d’un message.
- `layout_guard.rs` : un test qui échoue quand un fichier est laissé à la racine de `street/`, `junction/` ou
  `map/` au lieu d’être dans l’une de ses couches.
- `ui_guard.rs` : un test qui échoue quand une tranche dessine elle-même un groupe de boutons radio, des boutons plus et
  moins, une section des détails avec son titre ou une question, au lieu d’utiliser ceux de `cityloom_ui`.
- `atlas_contract.rs` : un test qui garde accordés ce qui est construit et la liste des mesures du tableau Atlas.

## `web/`

Le serveur envoie ces fichiers. Une page charge son script et ses feuilles de style depuis `v/<build>/`, une adresse
liée à la construction, pour qu’un navigateur ne mélange jamais les fichiers de deux déploiements.

| Page                | Script        | Contenu                                                                              |
| ------------------- | ------------- | ------------------------------------------------------------------------------------ |
| `index.html`        | `home.js`     | L’accueil : les projets de la personne.                                              |
| `about.html`        | `about.js`    | À quoi sert CityLoom.                                                                |
| `map.html`          | `map.js`      | La carte de la ville. `mapcanvas.js` est l’adaptateur qui possède la carte MapLibre. |
| `street.html`       | `app.js`      | L’éditeur de rue.                                                                    |
| `intersection.html` | `junction.js` | L’éditeur de jonction.                                                               |
| `account.html`      | `account.js`  | La page Compte.                                                                      |
| `timeline.html`     | `timeline.js` | La chronologie de conception.                                                        |
| `compare.html`      | `compare.js`  | Deux conceptions d’une rue ou d’une jonction côte à côte.                            |
| `credits.html`      |               | Les mentions, en anglais seulement.                                                  |

Les scripts de page ne servent qu’au démarrage : ils chargent le module WebAssembly et appellent sa fonction `mount_*`.
Les autres fichiers :

- `boot.js` : le message affiché quand le code d’une page ne peut pas être chargé ou démarré. C’est le seul script
  hors de `v/<build>/`.
- `city.js` : lit, dans l’adresse d’une page, le lieu pour lequel elle a été ouverte (`?street=7`, `?junction=3`).
- `report.js` : charge le générateur de PDF et ses polices la première fois qu’un rapport est demandé.
- `app.css` (toutes les pages), `editor.css` (les éditeurs de rue et de jonction), `home.css` (l’accueil et À propos), et
  `about.css`, `account.css`, `compare.css` et `timeline.css` pour leurs pages. La page de la carte utilise la feuille de style de
  MapLibre, dans `vendor/`.
- `fonts/` : les polices Noto Sans qu’intègre le rapport PDF. `glyphs/` : les glyphes des étiquettes de la carte.
  `licenses/` : les licences de ce qui est livré d’autres auteurs.
- Le logo, les icônes et `favicon.svg`.

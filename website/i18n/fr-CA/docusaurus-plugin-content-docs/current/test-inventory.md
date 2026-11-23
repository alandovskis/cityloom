---
id: test-inventory
title: Inventaire des tests
---

# Inventaire des tests

Cette page compte les tests de CityLoom par palier, par caisse et par niveau, et par fonctionnalité, et dit quels tests
n’appartiennent à aucune fonctionnalité. Les paliers eux-mêmes sont décrits dans [Tests](testing.md). Les nombres sont
ceux du commit `0d9cb8b9` (version 0.113.0). Aucun script ne produit ni ne vérifie ces nombres : ils restent donc
ceux de ce commit jusqu’au prochain décompte.

Il y a 5 385 tests : 4 552 tests Rust, 607 tests Playwright et 226 tests des scripts. Ce sont les tests qu’énumèrent ces
commandes :

```sh
cargo test --workspace --features cityloom_server/test-provider -- --list   # Rust : 4 552
cd e2e && npx playwright test --list                                        # Playwright : 607
python3 -m unittest discover -s scripts/include/tests                       # scripts : 226 (elle les exécute)
```

La liste de Rust n’exige pas de base de données. Celle de Playwright exige `npm ci` dans `e2e/` et le serveur que
`scripts/pin-server.sh` copie dans `target/e2e/cityloom_server` : `e2e/playwright.config.ts` lève une erreur sans ce
fichier.

Les chemins partent de la racine du dépôt, ou de `crates/cityloom_editor/src/` là où le texte nomme l’éditeur.

## Comment les tests sont comptés

### Un test

- **Rust** : une ligne de la liste qui se termine par `: test`. Les deux tests `#[ignore]` de
  `crates/cityloom_editor/src/street/view/iso/mod.rs` (`markup_size` et `preview`) sont dans la liste, donc comptés.
  Elle ne contient aucun test de documentation.
- **Playwright** : une ligne de la liste, c’est-à-dire un test d’un projet d’`e2e/playwright.config.ts`. Un test déclaré
  dans une boucle compte une fois par tour. Avec `E2E_ARCHIVE=1`, le projet de bout en bout énumère aussi les
  spécifications d’`archive_also` dans `e2e/tiers.json` : `map` (38 tests) et `street` (71 tests). Ces 109 tests sont
  ceux du niveau de l’interface exécutés de nouveau, et ne sont pas comptés une seconde fois.
- **Script** : une méthode de test de `scripts/include/tests/test_*.py`.

La [vérification de la construction wasm32](testing.md#la-vérification-de-la-construction-wasm32) n’exécute aucun test :
elle n’est donc pas ici.

### Le fichier d’un test

Le fichier d’un test Rust est le fichier le plus profond que nomme le chemin de son module sous le `src/` de sa caisse :
`a/b.rs`, `a/b/mod.rs` ou `a/tests.rs`, et `lib.rs` quand le chemin n’en nomme aucun. Un test de
`crates/*/tests/<nom>.rs` a ce fichier. Les tests de l’importateur forment une seule caisse de tests
(`crates/citymodel_builder/tests/importer/main.rs`), et chacun a le fichier de son module, à côté de `main.rs`. Le
fichier d’un test Playwright est sa spécification. Un fichier sans test n’est pas énuméré.

### Le palier d’un test

Le palier d’un test Rust vient de l’endroit où se trouve son fichier. La première règle qui correspond décide :

1. **Garde** : `i18n_guard.rs`, `layout_guard.rs`, `atlas_contract.rs` et `shared/contrast.rs` dans
   `crates/cityloom_editor/src/`, et `crates/cityloom_server/tests/no_plaintext.rs`.
2. **Serveur** : `crates/cityloom_server/`.
3. **`citymodel_builder` et `cityloom_model`** : `crates/citymodel_builder/` et `crates/cityloom_model/`.
4. **Rapport et PDF** : `crates/cityloom_report/` et `crates/cityloom_report_pdf/`.
5. **Composant** : dans l’éditeur, un fichier nommé `tests.rs`. `sync/tests.rs`, qui teste la
   synchronisation, est donc ici.
6. **Modèle de vue** : dans l’éditeur, un fichier sous un dossier `vm/`, ou nommé `vm.rs`.
7. **Modèle** : tout autre test unitaire de l’éditeur. Il comprend les fonctions pures qui rendent le SVG dans `view/`
   et `shared/` (comme `street/view/iso/`, `street/view/drawing.rs` et `shared/iso/`), et les autres fichiers de
   `shared/`.

Le palier d’un test Playwright est le niveau de sa spécification dans `e2e/tiers.json` : interface, intégration ou bout
en bout. Le palier d’un test des scripts est Script.

### Les fonctionnalités d’un test

Les 45 fonctionnalités sont celles de `scripts/ci/features.json`.

- Un test Rust appartient à chaque fonctionnalité dont un motif de `code` correspond à son fichier. Dans un motif, `*`
  reste dans un seul dossier et `**` traverse les dossiers.
- Un test Playwright appartient à chaque fonctionnalité qui nomme sa spécification dans `specs`.
- Un test peut appartenir à plusieurs fonctionnalités, et compte dans chacune : la somme des nombres des
  fonctionnalités dépasse donc celui des tests.
- Un test qui n’appartient à aucune est dans [Non associés](#non-associés).

## Par palier

| Palier                                  | Exécuté par  | Fichiers                                                                                                                                      |     Tests |
| --------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------- | --------: |
| Modèle                                  | `cargo test` | Tout autre test unitaire de l’éditeur                                                                                                         |     2 328 |
| Modèle de vue                           | `cargo test` | Dans l’éditeur, un fichier sous un dossier `vm/` ou nommé `vm.rs`                                                                             |       664 |
| Composant                               | `cargo test` | Dans l’éditeur, un fichier nommé `tests.rs`                                                                                                   |       871 |
| Garde                                   | `cargo test` | Dans l’éditeur, `i18n_guard.rs`, `layout_guard.rs`, `atlas_contract.rs`, `shared/contrast.rs`, `crates/cityloom_server/tests/no_plaintext.rs` |        53 |
| Serveur                                 | `cargo test` | `crates/cityloom_server/`, sans `no_plaintext.rs`                                                                                             |       264 |
| `citymodel_builder` et `cityloom_model` | `cargo test` | `crates/citymodel_builder/`, `crates/cityloom_model/`                                                                                         |       348 |
| Rapport et PDF                          | `cargo test` | `crates/cityloom_report/`, `crates/cityloom_report_pdf/`                                                                                      |        24 |
| Script                                  | `unittest`   | `scripts/include/tests/`                                                                                                                      |       226 |
| Niveau de l’interface                   | Playwright   | Les spécifications de `ui` dans `e2e/tiers.json`                                                                                              |       552 |
| Niveau d’intégration                    | Playwright   | Les spécifications d’`integration` dans `e2e/tiers.json`                                                                                      |        34 |
| Niveau de bout en bout                  | Playwright   | Les spécifications d’`e2e` dans `e2e/tiers.json`                                                                                              |        21 |
| **Total**                               |              |                                                                                                                                               | **5 385** |

## Par caisse et par niveau

### Tests Rust par caisse

Une ligne par binaire de test, nommé par son fichier racine dans le dossier de la caisse. La bibliothèque de
`cityloom_report_pdf` et les programmes de `cityloom_server` et de `citymodel_builder` (`src/main.rs`) sont aussi des
binaires de test, sans aucun test.

| Caisse                  | Binaire de test          |     Tests |
| ----------------------- | ------------------------ | --------: |
| `cityloom_editor`       | `src/lib.rs`             |     3 915 |
| `citymodel_builder`     | `src/lib.rs`             |       101 |
| `citymodel_builder`     | `tests/archive.rs`       |        33 |
| `citymodel_builder`     | `tests/commands.rs`      |        14 |
| `citymodel_builder`     | `tests/ctrl_c.rs`        |         2 |
| `citymodel_builder`     | `tests/download.rs`      |        17 |
| `citymodel_builder`     | `tests/importer/main.rs` |        71 |
| `citymodel_builder`     | `tests/prepare.rs`       |         7 |
| `citymodel_builder`     | `tests/progress.rs`      |        14 |
| `citymodel_builder`     | `tests/tiles.rs`         |         9 |
| **`citymodel_builder`** | **Total**                |   **268** |
| `cityloom_server`       | `src/lib.rs`             |        96 |
| `cityloom_server`       | `tests/api.rs`           |        10 |
| `cityloom_server`       | `tests/areas.rs`         |         4 |
| `cityloom_server`       | `tests/auth_flow.rs`     |        25 |
| `cityloom_server`       | `tests/data_mount.rs`    |         9 |
| `cityloom_server`       | `tests/db.rs`            |        10 |
| `cityloom_server`       | `tests/no_plaintext.rs`  |         1 |
| `cityloom_server`       | `tests/providers.rs`     |         8 |
| `cityloom_server`       | `tests/rotate.rs`        |         4 |
| `cityloom_server`       | `tests/share_api.rs`     |        13 |
| `cityloom_server`       | `tests/share_db.rs`      |        10 |
| `cityloom_server`       | `tests/static_files.rs`  |        27 |
| `cityloom_server`       | `tests/unavailable.rs`   |         3 |
| `cityloom_server`       | `tests/work_api.rs`      |        20 |
| `cityloom_server`       | `tests/work_db.rs`       |        25 |
| **`cityloom_server`**   | **Total**                |   **265** |
| `cityloom_model`        | `src/lib.rs`             |        80 |
| `cityloom_report_pdf`   | `tests/pdf.rs`           |        23 |
| `cityloom_report`       | `src/lib.rs`             |         1 |
| **Total**               |                          | **4 552** |

Les paliers de chaque caisse :

| Caisse                | Tests par palier                                                 |
| --------------------- | ---------------------------------------------------------------- |
| `cityloom_editor`     | 3 915 (modèle 2 328, modèle de vue 664, composant 871, garde 52) |
| `citymodel_builder`   | 268 (`citymodel_builder` et `cityloom_model`)                    |
| `cityloom_server`     | 265 (garde 1, serveur 264)                                       |
| `cityloom_model`      | 80 (`citymodel_builder` et `cityloom_model`)                     |
| `cityloom_report_pdf` | 23 (rapport et PDF)                                              |
| `cityloom_report`     | 1 (rapport et PDF)                                               |

### Tests Playwright par niveau

Une ligne par projet Playwright, avec ses spécifications (`e2e/tests/<nom>.spec.ts`) et les tests de chacune.

| Projet        | Nombre de spécifications | Tests de chaque spécification                                                                                                                                                                                                                                                                                                                                                                                                                                                              |   Tests |
| ------------- | -----------------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------: |
| `ui`          |                       35 | `a11y` 21, `about` 16, `account` 7, `calming` 10, `compare` 8, `curb` 5, `design-archive` 4, `editors` 26, `edits` 6, `focus-touch` 10, `footer` 11, `green` 7, `home` 5, `impact` 11, `junction` 81, `junction-a11y` 21, `language` 21, `map` 38, `network` 8, `projects` 12, `read-only` 5, `report` 7, `restrictions` 18, `scenarios` 9, `search` 13, `shell` 47, `speed` 10, `standards` 7, `stops` 6, `street` 71, `street-iso` 14, `templates` 5, `timeline` 7, `webgl` 1, `world` 4 |     552 |
| `integration` |                        3 | `account-server` 12, `share` 9, `sync` 13                                                                                                                                                                                                                                                                                                                                                                                                                                                  |      34 |
| `e2e`         |                        3 | `archive` 3, `delivery` 14, `serve` 4                                                                                                                                                                                                                                                                                                                                                                                                                                                      |      21 |
| **Total**     |                   **41** |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | **607** |

## Par fonctionnalité

Les fonctionnalités sont dans l’ordre de `scripts/ci/features.json`. Chacune mène à ses fichiers de test et à ses
spécifications.

| Fonctionnalité                                            | Nom                                                                                                                                                                                                            | Tests Rust | Tests Playwright |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------: | ---------------: |
| [`street.pieces`](#streetpieces)                          | La coupe d’une rue se compose d’éléments qu’on ajoute, retire, redimensionne et réorganise, avec Annuler et Recommencer                                                                                        |        361 |               97 |
| [`street.drawing`](#streetdrawing)                        | Une rue est dessinée en 2,5D, et on choisit un élément en appuyant dessus dans le dessin                                                                                                                       |        240 |               14 |
| [`street.stretches`](#streetstretches)                    | Une rue se divise en tronçons, chacun avec sa propre disposition, dont on glisse les limites sur le plan de la rue                                                                                             |        118 |               92 |
| [`street.speed`](#streetspeed)                            | Une rue et chacun de ses tronçons ont une limite de vitesse, importée d’OpenStreetMap et modifiée par le concepteur                                                                                            |         15 |               10 |
| [`street.curbside`](#streetcurbside)                      | Les usages du bord de rue (stationnement, livraison, arbres, terrasses, stationnement pour vélos, quais) se placent le long de la bordure                                                                      |         68 |                5 |
| [`street.stops`](#streetstops)                            | Des arrêts d’autobus s’ajoutent le long d’une rue, en bordure, sur une avancée ou sur un îlot, avec ou sans abri                                                                                               |         52 |                6 |
| [`street.calming`](#streetcalming)                        | Des éléments surélevés apaisent la circulation : plateaux surélevés, passages piétons surélevés, chicanes et filtres modaux                                                                                    |         58 |               10 |
| [`street.green`](#streetgreen)                            | Des arbres et des infrastructures vertes (fosses d’arbre, plates-bandes, jardins de pluie, noues végétalisées) se placent dans une rue                                                                         |         64 |                7 |
| [`street.checks`](#streetchecks)                          | Les vérifications d’une rue disent si ses éléments tiennent et respectent les normes, et la carte Impact dit ce qu’a changé la dernière modification                                                           |         32 |               82 |
| [`street.templates`](#streettemplates)                    | Une rue se transforme d’un coup selon un modèle, comme une rue de transport en commun ou une rue scolaire, et Annuler le défait                                                                                |         50 |                5 |
| [`street.standards`](#streetstandards)                    | Un profil de normes remplace les seuils intégrés que lisent les vérifications d’une rue                                                                                                                        |         36 |                7 |
| [`junction.layout`](#junctionlayout)                      | Les branches, voies, virages, passages pour piétons et la signalisation d’une jonction sont dessinés et modifiés, et chaque partie s’atteint au clavier ou d’une touche                                        |        797 |              112 |
| [`junction.roundabout`](#junctionroundabout)              | Une jonction devient un carrefour giratoire ou un giratoire turbo                                                                                                                                              |         40 |               81 |
| [`junction.signal.phases`](#junctionsignalphases)         | Les phases d’une jonction à feux se choisissent parmi des modèles tout faits ou se règlent à la main, avec leur minutage                                                                                       |         46 |               81 |
| [`junction.signal.plans`](#junctionsignalplans)           | Des feux suivent des plans selon l’heure, d’après l’horaire de la semaine                                                                                                                                      |         14 |               86 |
| [`junction.signal.calls`](#junctionsignalcalls)           | Des feux servent les piétons sur appel, par bouton-poussoir ou par détection passive                                                                                                                           |         33 |               86 |
| [`junction.signal.actuation`](#junctionsignalactuation)   | Le vert des feux répond à la demande des véhicules (à la demande)                                                                                                                                              |         16 |               86 |
| [`junction.signal.bicycles`](#junctionsignalbicycles)     | Des feux détectent les vélos et leur donnent leur vert                                                                                                                                                         |         14 |               86 |
| [`junction.signal.transit`](#junctionsignaltransit)       | Des feux donnent la priorité au transport en commun par vert hâtif ou prolongation du vert                                                                                                                     |         16 |               86 |
| [`junction.signal.preemption`](#junctionsignalpreemption) | Les feux d’une jonction accordent la préemption aux appels d’urgence                                                                                                                                           |         19 |               86 |
| [`design.scenarios`](#designscenarios)                    | Une rue ou une jonction a des scénarios nommés à côté de sa référence, qu’on crée, renomme, duplique, supprime et entre lesquels on passe                                                                      |         10 |                9 |
| [`design.compare`](#designcompare)                        | Deux aménagements d’une même rue ou jonction se comparent côte à côte, avec ce qui diffère dans le second                                                                                                      |         65 |                8 |
| [`map.city`](#mapcity)                                    | La carte de la ville montre la zone du projet avec ses rues et ses jonctions, et ouvre chacune dans son éditeur                                                                                                |        197 |               39 |
| [`map.search`](#mapsearch)                                | Les lieux de la carte se trouvent par leur nom dans la boîte de recherche                                                                                                                                      |          2 |               13 |
| [`map.goals`](#mapgoals)                                  | Un objectif de planification choisi sur la carte se suit sur les pages de rue et de jonction                                                                                                                   |         13 |               38 |
| [`map.network-checks`](#mapnetwork-checks)                | Les vérifications du réseau repèrent culs-de-sac, passages pour piétons manquants, liens cyclables rompus et axes de transport en commun dans la zone                                                          |         34 |                8 |
| [`map.area-rules`](#maparea-rules)                        | Des règles se déclarent sur une zone tracée sur la carte (sans voitures, autobus et vélos seulement, limite de vitesse, stationnement et chargement interdits), et les rues qui les enfreignent sont signalées |        108 |               18 |
| [`map.report`](#mapreport)                                | Le rapport d’examen de l’aménagement se télécharge en PDF, dans la langue de la page                                                                                                                           |         13 |                7 |
| [`city.area`](#cityarea)                                  | La zone d’un projet est tirée de ses tuiles, découpée et transformée en une ville de rues et de jonctions, puis refaite quand sa tuile change                                                                  |        185 |                4 |
| [`city.edits`](#cityedits)                                | Les modifications d’une rue ou d’une jonction sont conservées dans le navigateur sous ses identifiants OpenStreetMap, et survivent au rechargement                                                             |        160 |               10 |
| [`data.importer`](#dataimporter)                          | Les routes d’OpenStreetMap deviennent le réseau de rues que lit l’éditeur                                                                                                                                      |        114 |                0 |
| [`data.archive`](#dataarchive)                            | Le réseau est découpé en tuiles et regroupé dans l’archive de la ville, que le serveur sert                                                                                                                    |         50 |                3 |
| [`projects.list`](#projectslist)                          | La page d’accueil liste les projets de la personne; un projet se crée dans une région métropolitaine et s’ouvre sur sa carte                                                                                   |        139 |               17 |
| [`projects.timeline`](#projectstimeline)                  | Les jalons d’un projet conservent son travail à un moment donné, et se comparent au travail actuel                                                                                                             |         43 |                7 |
| [`projects.design-archive`](#projectsdesign-archive)      | L’aménagement d’un projet s’exporte en un seul fichier et s’importe sur un autre appareil                                                                                                                      |         29 |                4 |
| [`projects.share`](#projectsshare)                        | Un projet se partage par un lien qui l’ouvre en lecture seule, et un aménagement partagé se copie dans ses propres projets                                                                                     |         60 |                9 |
| [`projects.read-only`](#projectsread-only)                | Un aménagement ouvert en lecture seule se consulte et on peut y choisir des éléments, mais aucune commande ne le modifie                                                                                       |          1 |                5 |
| [`account.sign-in`](#accountsign-in)                      | Une personne se connecte par un fournisseur, associe ou dissocie des connexions, choisit où conserver son travail et supprime son compte                                                                       |        105 |               19 |
| [`account.sync`](#accountsync)                            | Le travail d’une personne connectée est conservé sur le serveur et fusionné entre ses appareils, même après un travail hors ligne                                                                              |        113 |               13 |
| [`shell.settings`](#shellsettings)                        | Chaque page a la barre du haut, le menu des réglages (thème, unités, région, raccourcis clavier), le panneau des détails et le tiroir des notes                                                                |        118 |               84 |
| [`shell.language`](#shelllanguage)                        | Chaque page de l’application se lit en anglais ou en français canadien, et le choix est conservé                                                                                                               |         23 |               21 |
| [`shell.accessibility`](#shellaccessibility)              | Chaque page s’utilise au clavier et avec un lecteur d’écran, à tout zoom et toute largeur, et ses couleurs respectent les WCAG 2.2 AA                                                                          |         28 |               31 |
| [`pages.about`](#pagesabout)                              | La page À propos dit à quoi sert CityLoom, sur la ville du dernier projet ouvert                                                                                                                               |         14 |               16 |
| [`pages.credits`](#pagescredits)                          | La page des remerciements nomme les données et les logiciels dont CityLoom est fait                                                                                                                            |          0 |                0 |
| [`server.delivery`](#serverdelivery)                      | Le serveur livre rapidement le site et les données de la ville : adresses versionnées, compression et plages d’octets                                                                                          |         22 |               18 |
| **Somme sur les fonctionnalités**                         |                                                                                                                                                                                                                |  **3 735** |        **1 526** |
| **Tests associés distincts**                              |                                                                                                                                                                                                                |  **3 666** |          **607** |

Les sommes dépassent les tests associés distincts, parce que 9 fichiers Rust et 8 spécifications appartiennent à plusieurs
fonctionnalités :

| Fichier de test ou spécification                           | Tests | Fonctionnalités                                                                                                                                                                                                                        |
| ---------------------------------------------------------- | ----: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    |     9 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` |     8 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/view/turns.rs`        |     1 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/vm/compare.rs`        |    11 | `junction.layout`, `design.compare`                                                                                                                                                                                                    |
| `crates/cityloom_editor/src/project/copies.rs`             |    13 | `projects.list`, `projects.share`                                                                                                                                                                                                      |
| `crates/cityloom_editor/src/shell/session.rs`              |    20 | `account.sign-in`, `shell.settings`                                                                                                                                                                                                    |
| `crates/cityloom_model/src/catalogue.rs`                   |     2 | `street.pieces`, `data.importer`                                                                                                                                                                                                       |
| `crates/cityloom_model/src/corridor.rs`                    |     4 | `map.network-checks`, `data.importer`                                                                                                                                                                                                  |
| `crates/cityloom_model/src/street/speed.rs`                |     1 | `street.pieces`, `street.speed`                                                                                                                                                                                                        |
| `e2e/tests/a11y.spec.ts`                                   |    21 | `street.stretches`, `shell.accessibility`                                                                                                                                                                                              |
| `e2e/tests/editors.spec.ts`                                |    26 | `street.pieces`, `shell.settings`                                                                                                                                                                                                      |
| `e2e/tests/focus-touch.spec.ts`                            |    10 | `junction.layout`, `shell.accessibility`                                                                                                                                                                                               |
| `e2e/tests/junction.spec.ts`                               |    81 | `junction.layout`, `junction.roundabout`, `junction.signal.phases`, `junction.signal.plans`, `junction.signal.calls`, `junction.signal.actuation`, `junction.signal.bicycles`, `junction.signal.transit`, `junction.signal.preemption` |
| `e2e/tests/map.spec.ts`                                    |    38 | `map.city`, `map.goals`                                                                                                                                                                                                                |
| `e2e/tests/read-only.spec.ts`                              |     5 | `junction.signal.plans`, `junction.signal.calls`, `junction.signal.actuation`, `junction.signal.bicycles`, `junction.signal.transit`, `junction.signal.preemption`, `projects.read-only`                                               |
| `e2e/tests/street.spec.ts`                                 |    71 | `street.pieces`, `street.stretches`, `street.checks`                                                                                                                                                                                   |
| `e2e/tests/world.spec.ts`                                  |     4 | `city.area`, `city.edits`                                                                                                                                                                                                              |

### `street.pieces`

La coupe d’une rue se compose d’éléments qu’on ajoute, retire, redimensionne et réorganise, avec Annuler et Recommencer.

- Tests Rust : 361 (modèle 164, modèle de vue 178, `citymodel_builder` et `cityloom_model` 19)
- Tests Playwright : 97 (niveau de l’interface)

| Fichier de test ou spécification                      | Palier                                  | Tests |
| ----------------------------------------------------- | --------------------------------------- | ----: |
| `crates/cityloom_editor/src/shared/catalogue.rs`      | Modèle                                  |     6 |
| `crates/cityloom_editor/src/street/model/mod.rs`      | Modèle                                  |   109 |
| `crates/cityloom_editor/src/street/model/pick.rs`     | Modèle                                  |    37 |
| `crates/cityloom_editor/src/street/model/pieces.rs`   | Modèle                                  |     2 |
| `crates/cityloom_editor/src/street/view/inspector.rs` | Modèle                                  |     5 |
| `crates/cityloom_editor/src/street/view/page.rs`      | Modèle                                  |     5 |
| `crates/cityloom_editor/src/street/vm/add_menu.rs`    | Modèle de vue                           |     5 |
| `crates/cityloom_editor/src/street/vm/keys.rs`        | Modèle de vue                           |    16 |
| `crates/cityloom_editor/src/street/vm/mod.rs`         | Modèle de vue                           |   157 |
| `crates/cityloom_model/src/catalogue.rs`              | `citymodel_builder` et `cityloom_model` |     2 |
| `crates/cityloom_model/src/street/from_road.rs`       | `citymodel_builder` et `cityloom_model` |    10 |
| `crates/cityloom_model/src/street/mod.rs`             | `citymodel_builder` et `cityloom_model` |     6 |
| `crates/cityloom_model/src/street/speed.rs`           | `citymodel_builder` et `cityloom_model` |     1 |
| `e2e/tests/street.spec.ts`                            | Niveau de l’interface                   |    71 |
| `e2e/tests/editors.spec.ts`                           | Niveau de l’interface                   |    26 |

### `street.drawing`

Une rue est dessinée en 2,5D, et on choisit un élément en appuyant dessus dans le dessin.

- Tests Rust : 240 (modèle)
- Tests Playwright : 14 (niveau de l’interface)

| Fichier de test ou spécification                               | Palier                | Tests |
| -------------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/iso/format.rs`              | Modèle                |     2 |
| `crates/cityloom_editor/src/shared/iso/ground.rs`              | Modèle                |     6 |
| `crates/cityloom_editor/src/shared/iso/paint.rs`               | Modèle                |     7 |
| `crates/cityloom_editor/src/shared/iso/shading.rs`             | Modèle                |     1 |
| `crates/cityloom_editor/src/street/view/iso/calming.rs`        | Modèle                |    17 |
| `crates/cityloom_editor/src/street/view/iso/curbside.rs`       | Modèle                |    14 |
| `crates/cityloom_editor/src/street/view/iso/green.rs`          | Modèle                |    12 |
| `crates/cityloom_editor/src/street/view/iso/joints.rs`         | Modèle                |     8 |
| `crates/cityloom_editor/src/street/view/iso/markings.rs`       | Modèle                |    18 |
| `crates/cityloom_editor/src/street/view/iso/mod.rs`            | Modèle                |    67 |
| `crates/cityloom_editor/src/street/view/iso/order.rs`          | Modèle                |     4 |
| `crates/cityloom_editor/src/street/view/iso/paint.rs`          | Modèle                |     9 |
| `crates/cityloom_editor/src/street/view/iso/projection.rs`     | Modèle                |    15 |
| `crates/cityloom_editor/src/street/view/iso/raised.rs`         | Modèle                |     9 |
| `crates/cityloom_editor/src/street/view/iso/scene/hits.rs`     | Modèle                |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/markings.rs` | Modèle                |     3 |
| `crates/cityloom_editor/src/street/view/iso/scene/objects.rs`  | Modèle                |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/raised.rs`   | Modèle                |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/surface.rs`  | Modèle                |     3 |
| `crates/cityloom_editor/src/street/view/iso/sign.rs`           | Modèle                |     2 |
| `crates/cityloom_editor/src/street/view/iso/solids.rs`         | Modèle                |    11 |
| `crates/cityloom_editor/src/street/view/iso/stops.rs`          | Modèle                |    12 |
| `crates/cityloom_editor/src/street/view/iso/surface.rs`        | Modèle                |     4 |
| `crates/cityloom_editor/src/street/view/iso/symbols.rs`        | Modèle                |     7 |
| `crates/cityloom_editor/src/street/view/iso/taper.rs`          | Modèle                |     3 |
| `e2e/tests/street-iso.spec.ts`                                 | Niveau de l’interface |    14 |

### `street.stretches`

Une rue se divise en tronçons, chacun avec sa propre disposition, dont on glisse les limites sur le plan de la rue.

- Tests Rust : 118 (modèle 95, modèle de vue 23)
- Tests Playwright : 92 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                | Tests |
| ------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/joint.rs`      | Modèle                |     9 |
| `crates/cityloom_editor/src/street/model/plan.rs`       | Modèle                |     9 |
| `crates/cityloom_editor/src/street/model/transition.rs` | Modèle                |    24 |
| `crates/cityloom_editor/src/street/model/zones.rs`      | Modèle                |    30 |
| `crates/cityloom_editor/src/street/view/plan/hit.rs`    | Modèle                |     7 |
| `crates/cityloom_editor/src/street/view/plan/mod.rs`    | Modèle                |    14 |
| `crates/cityloom_editor/src/street/view/plan/scale.rs`  | Modèle                |     2 |
| `crates/cityloom_editor/src/street/vm/plan.rs`          | Modèle de vue         |    23 |
| `e2e/tests/street.spec.ts`                              | Niveau de l’interface |    71 |
| `e2e/tests/a11y.spec.ts`                                | Niveau de l’interface |    21 |

### `street.speed`

Une rue et chacun de ses tronçons ont une limite de vitesse, importée d’OpenStreetMap et modifiée par le concepteur.

- Tests Rust : 15 (modèle 14, `citymodel_builder` et `cityloom_model` 1)
- Tests Playwright : 10 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                                  | Tests |
| ------------------------------------------------------- | --------------------------------------- | ----: |
| `crates/cityloom_editor/src/street/model/zone_speed.rs` | Modèle                                  |    14 |
| `crates/cityloom_model/src/street/speed.rs`             | `citymodel_builder` et `cityloom_model` |     1 |
| `e2e/tests/speed.spec.ts`                               | Niveau de l’interface                   |    10 |

### `street.curbside`

Les usages du bord de rue (stationnement, livraison, arbres, terrasses, stationnement pour vélos, quais) se placent le long de la bordure.

- Tests Rust : 68 (modèle)
- Tests Playwright : 5 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                | Tests |
| ------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/curb.rs`       | Modèle                |    28 |
| `crates/cityloom_editor/src/street/model/curb_end.rs`   | Modèle                |     7 |
| `crates/cityloom_editor/src/street/model/curb_shown.rs` | Modèle                |    12 |
| `crates/cityloom_editor/src/street/model/map_uses.rs`   | Modèle                |    12 |
| `crates/cityloom_editor/src/street/view/curb_strip.rs`  | Modèle                |     9 |
| `e2e/tests/curb.spec.ts`                                | Niveau de l’interface |     5 |

### `street.stops`

Des arrêts d’autobus s’ajoutent le long d’une rue, en bordure, sur une avancée ou sur un îlot, avec ou sans abri.

- Tests Rust : 52 (modèle)
- Tests Playwright : 6 (niveau de l’interface)

| Fichier de test ou spécification                      | Palier                | Tests |
| ----------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/stop.rs`     | Modèle                |    37 |
| `crates/cityloom_editor/src/street/model/stop_end.rs` | Modèle                |    15 |
| `e2e/tests/stops.spec.ts`                             | Niveau de l’interface |     6 |

### `street.calming`

Des éléments surélevés apaisent la circulation : plateaux surélevés, passages piétons surélevés, chicanes et filtres modaux.

- Tests Rust : 58 (modèle)
- Tests Playwright : 10 (niveau de l’interface)

| Fichier de test ou spécification                           | Palier                | Tests |
| ---------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/calming.rs`       | Modèle                |    52 |
| `crates/cityloom_editor/src/street/model/calming_shown.rs` | Modèle                |     6 |
| `e2e/tests/calming.spec.ts`                                | Niveau de l’interface |    10 |

### `street.green`

Des arbres et des infrastructures vertes (fosses d’arbre, plates-bandes, jardins de pluie, noues végétalisées) se placent dans une rue.

- Tests Rust : 64 (modèle)
- Tests Playwright : 7 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                | Tests |
| ------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/green.rs`      | Modèle                |    59 |
| `crates/cityloom_editor/src/street/view/green_strip.rs` | Modèle                |     3 |
| `crates/cityloom_editor/src/street/view/green_tools.rs` | Modèle                |     2 |
| `e2e/tests/green.spec.ts`                               | Niveau de l’interface |     7 |

### `street.checks`

Les vérifications d’une rue disent si ses éléments tiennent et respectent les normes, et la carte Impact dit ce qu’a changé la dernière modification.

- Tests Rust : 32 (modèle 17, modèle de vue 15)
- Tests Playwright : 82 (niveau de l’interface)

| Fichier de test ou spécification                         | Palier                | Tests |
| -------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/check_fold.rs`        | Modèle                |     4 |
| `crates/cityloom_editor/src/street/model/profile.rs`     | Modèle                |     7 |
| `crates/cityloom_editor/src/street/model/unavailable.rs` | Modèle                |     6 |
| `crates/cityloom_editor/src/street/vm/impact.rs`         | Modèle de vue         |    15 |
| `e2e/tests/impact.spec.ts`                               | Niveau de l’interface |    11 |
| `e2e/tests/street.spec.ts`                               | Niveau de l’interface |    71 |

### `street.templates`

Une rue se transforme d’un coup selon un modèle, comme une rue de transport en commun ou une rue scolaire, et Annuler le défait.

- Tests Rust : 50 (modèle 27, modèle de vue 23)
- Tests Playwright : 5 (niveau de l’interface)

| Fichier de test ou spécification                       | Palier                | Tests |
| ------------------------------------------------------ | --------------------- | ----: |
| `crates/cityloom_editor/src/street/model/school.rs`    | Modèle                |    17 |
| `crates/cityloom_editor/src/street/model/templates.rs` | Modèle                |    10 |
| `crates/cityloom_editor/src/street/vm/transform.rs`    | Modèle de vue         |    23 |
| `e2e/tests/templates.spec.ts`                          | Niveau de l’interface |     5 |

### `street.standards`

Un profil de normes remplace les seuils intégrés que lisent les vérifications d’une rue.

- Tests Rust : 36 (modèle 23, composant 13)
- Tests Playwright : 7 (niveau de l’interface)

| Fichier de test ou spécification                | Palier                | Tests |
| ----------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/standards/model.rs` | Modèle                |    10 |
| `crates/cityloom_editor/src/standards/store.rs` | Modèle                |    12 |
| `crates/cityloom_editor/src/standards/tests.rs` | Composant             |    13 |
| `crates/cityloom_editor/src/standards/view.rs`  | Modèle                |     1 |
| `e2e/tests/standards.spec.ts`                   | Niveau de l’interface |     7 |

### `junction.layout`

Les branches, voies, virages, passages pour piétons et la signalisation d’une jonction sont dessinés et modifiés, et chaque partie s’atteint au clavier ou d’une touche.

- Tests Rust : 797 (modèle 469, modèle de vue 93, composant 235)
- Tests Playwright : 112 (niveau de l’interface)

| Fichier de test ou spécification                           | Palier                | Tests |
| ---------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/beyond.rs`      | Modèle                |    10 |
| `crates/cityloom_editor/src/junction/model/control.rs`     | Modèle                |     4 |
| `crates/cityloom_editor/src/junction/model/curbside.rs`    | Modèle                |     5 |
| `crates/cityloom_editor/src/junction/model/geometry.rs`    | Modèle                |     9 |
| `crates/cityloom_editor/src/junction/model/mod.rs`         | Modèle                |   163 |
| `crates/cityloom_editor/src/junction/model/read_model.rs`  | Modèle                |   137 |
| `crates/cityloom_editor/src/junction/model/stops.rs`       | Modèle                |    12 |
| `crates/cityloom_editor/src/junction/view/drawing.rs`      | Modèle                |     3 |
| `crates/cityloom_editor/src/junction/view/frame.rs`        | Modèle                |     3 |
| `crates/cityloom_editor/src/junction/view/inspector.rs`    | Modèle                |     8 |
| `crates/cityloom_editor/src/junction/view/iso/mod.rs`      | Modèle                |    59 |
| `crates/cityloom_editor/src/junction/view/iso_marks.rs`    | Modèle                |    31 |
| `crates/cityloom_editor/src/junction/view/notes.rs`        | Modèle                |     5 |
| `crates/cityloom_editor/src/junction/view/page.rs`         | Modèle                |     2 |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    | Modèle                |     9 |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` | Modèle                |     8 |
| `crates/cityloom_editor/src/junction/view/tests.rs`        | Composant             |   235 |
| `crates/cityloom_editor/src/junction/view/turns.rs`        | Modèle                |     1 |
| `crates/cityloom_editor/src/junction/vm/compare.rs`        | Modèle de vue         |    11 |
| `crates/cityloom_editor/src/junction/vm/keys.rs`           | Modèle de vue         |    11 |
| `crates/cityloom_editor/src/junction/vm/mod.rs`            | Modèle de vue         |    56 |
| `crates/cityloom_editor/src/junction/vm/text.rs`           | Modèle de vue         |    15 |
| `e2e/tests/junction.spec.ts`                               | Niveau de l’interface |    81 |
| `e2e/tests/junction-a11y.spec.ts`                          | Niveau de l’interface |    21 |
| `e2e/tests/focus-touch.spec.ts`                            | Niveau de l’interface |    10 |

### `junction.roundabout`

Une jonction devient un carrefour giratoire ou un giratoire turbo.

- Tests Rust : 40 (modèle)
- Tests Playwright : 81 (niveau de l’interface)

| Fichier de test ou spécification                            | Palier                | Tests |
| ----------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/turbo.rs`        | Modèle                |    22 |
| `crates/cityloom_editor/src/junction/model/turbo_ring.rs`   | Modèle                |     2 |
| `crates/cityloom_editor/src/junction/model/turbo_spiral.rs` | Modèle                |    16 |
| `e2e/tests/junction.spec.ts`                                | Niveau de l’interface |    81 |

### `junction.signal.phases`

Les phases d’une jonction à feux se choisissent parmi des modèles tout faits ou se règlent à la main, avec leur minutage.

- Tests Rust : 46 (modèle)
- Tests Playwright : 81 (niveau de l’interface)

| Fichier de test ou spécification                           | Palier                | Tests |
| ---------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/signal.rs`      | Modèle                |    28 |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    | Modèle                |     9 |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` | Modèle                |     8 |
| `crates/cityloom_editor/src/junction/view/turns.rs`        | Modèle                |     1 |
| `e2e/tests/junction.spec.ts`                               | Niveau de l’interface |    81 |

### `junction.signal.plans`

Des feux suivent des plans selon l’heure, d’après l’horaire de la semaine.

- Tests Rust : 14 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                     | Palier                | Tests |
| ---------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/plans.rs` | Modèle                |    14 |
| `e2e/tests/junction.spec.ts`                         | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                        | Niveau de l’interface |     5 |

### `junction.signal.calls`

Des feux servent les piétons sur appel, par bouton-poussoir ou par détection passive.

- Tests Rust : 33 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                     | Palier                | Tests |
| ---------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/calls.rs` | Modèle                |    33 |
| `e2e/tests/junction.spec.ts`                         | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                        | Niveau de l’interface |     5 |

### `junction.signal.actuation`

Le vert des feux répond à la demande des véhicules (à la demande).

- Tests Rust : 16 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                         | Palier                | Tests |
| -------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/actuation.rs` | Modèle                |    16 |
| `e2e/tests/junction.spec.ts`                             | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                            | Niveau de l’interface |     5 |

### `junction.signal.bicycles`

Des feux détectent les vélos et leur donnent leur vert.

- Tests Rust : 14 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                | Tests |
| ------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/bicycles.rs` | Modèle                |    14 |
| `e2e/tests/junction.spec.ts`                            | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                           | Niveau de l’interface |     5 |

### `junction.signal.transit`

Des feux donnent la priorité au transport en commun par vert hâtif ou prolongation du vert.

- Tests Rust : 16 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                       | Palier                | Tests |
| ------------------------------------------------------ | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/transit.rs` | Modèle                |    16 |
| `e2e/tests/junction.spec.ts`                           | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                          | Niveau de l’interface |     5 |

### `junction.signal.preemption`

Les feux d’une jonction accordent la préemption aux appels d’urgence.

- Tests Rust : 19 (modèle)
- Tests Playwright : 86 (niveau de l’interface)

| Fichier de test ou spécification                          | Palier                | Tests |
| --------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/preemption.rs` | Modèle                |    19 |
| `e2e/tests/junction.spec.ts`                              | Niveau de l’interface |    81 |
| `e2e/tests/read-only.spec.ts`                             | Niveau de l’interface |     5 |

### `design.scenarios`

Une rue ou une jonction a des scénarios nommés à côté de sa référence, qu’on crée, renomme, duplique, supprime et entre lesquels on passe.

- Tests Rust : 10 (modèle)
- Tests Playwright : 9 (niveau de l’interface)

| Fichier de test ou spécification              | Palier                | Tests |
| --------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/city/scenario.rs` | Modèle                |    10 |
| `e2e/tests/scenarios.spec.ts`                 | Niveau de l’interface |     9 |

### `design.compare`

Deux aménagements d’une même rue ou jonction se comparent côte à côte, avec ce qui diffère dans le second.

- Tests Rust : 65 (modèle 44, modèle de vue 21)
- Tests Playwright : 8 (niveau de l’interface)

| Fichier de test ou spécification                       | Palier                | Tests |
| ------------------------------------------------------ | --------------------- | ----: |
| `crates/cityloom_editor/src/junction/model/compare.rs` | Modèle                |    13 |
| `crates/cityloom_editor/src/junction/vm/compare.rs`    | Modèle de vue         |    11 |
| `crates/cityloom_editor/src/shared/compare.rs`         | Modèle                |     7 |
| `crates/cityloom_editor/src/shared/compare_view.rs`    | Modèle                |    11 |
| `crates/cityloom_editor/src/street/model/compare.rs`   | Modèle                |    13 |
| `crates/cityloom_editor/src/street/vm/compare.rs`      | Modèle de vue         |    10 |
| `e2e/tests/compare.spec.ts`                            | Niveau de l’interface |     8 |

### `map.city`

La carte de la ville montre la zone du projet avec ses rues et ses jonctions, et ouvre chacune dans son éditeur.

- Tests Rust : 197 (modèle 6, modèle de vue 191)
- Tests Playwright : 39 (niveau de l’interface)

| Fichier de test ou spécification                   | Palier                | Tests |
| -------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/map/model/editable.rs` | Modèle                |     6 |
| `crates/cityloom_editor/src/map/vm/camera.rs`      | Modèle de vue         |    11 |
| `crates/cityloom_editor/src/map/vm/mod.rs`         | Modèle de vue         |   105 |
| `crates/cityloom_editor/src/map/vm/overlay.rs`     | Modèle de vue         |    50 |
| `crates/cityloom_editor/src/map/vm/selection.rs`   | Modèle de vue         |     4 |
| `crates/cityloom_editor/src/map/vm/style.rs`       | Modèle de vue         |    21 |
| `e2e/tests/map.spec.ts`                            | Niveau de l’interface |    38 |
| `e2e/tests/webgl.spec.ts`                          | Niveau de l’interface |     1 |

### `map.search`

Les lieux de la carte se trouvent par leur nom dans la boîte de recherche.

- Tests Rust : 2 (modèle)
- Tests Playwright : 13 (niveau de l’interface)

| Fichier de test ou spécification              | Palier                | Tests |
| --------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/search.rs` | Modèle                |     2 |
| `e2e/tests/search.spec.ts`                    | Niveau de l’interface |    13 |

### `map.goals`

Un objectif de planification choisi sur la carte se suit sur les pages de rue et de jonction.

- Tests Rust : 13 (modèle)
- Tests Playwright : 38 (niveau de l’interface)

| Fichier de test ou spécification                  | Palier                | Tests |
| ------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/goals.rs`      | Modèle                |     8 |
| `crates/cityloom_editor/src/shared/goals_view.rs` | Modèle                |     5 |
| `e2e/tests/map.spec.ts`                           | Niveau de l’interface |    38 |

### `map.network-checks`

Les vérifications du réseau repèrent culs-de-sac, passages pour piétons manquants, liens cyclables rompus et axes de transport en commun dans la zone.

- Tests Rust : 34 (modèle 30, `citymodel_builder` et `cityloom_model` 4)
- Tests Playwright : 8 (niveau de l’interface)

| Fichier de test ou spécification                   | Palier                                  | Tests |
| -------------------------------------------------- | --------------------------------------- | ----: |
| `crates/cityloom_editor/src/map/model/corridor.rs` | Modèle                                  |     7 |
| `crates/cityloom_editor/src/map/model/network.rs`  | Modèle                                  |    23 |
| `crates/cityloom_model/src/corridor.rs`            | `citymodel_builder` et `cityloom_model` |     4 |
| `e2e/tests/network.spec.ts`                        | Niveau de l’interface                   |     8 |

### `map.area-rules`

Des règles se déclarent sur une zone tracée sur la carte (sans voitures, autobus et vélos seulement, limite de vitesse, stationnement et chargement interdits), et les rues qui les enfreignent sont signalées.

- Tests Rust : 108 (modèle 64, modèle de vue 44)
- Tests Playwright : 18 (niveau de l’interface)

| Fichier de test ou spécification                  | Palier                | Tests |
| ------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/map/vm/areas.rs`      | Modèle de vue         |    44 |
| `crates/cityloom_editor/src/restriction/draw.rs`  | Modèle                |    14 |
| `crates/cityloom_editor/src/restriction/mod.rs`   | Modèle                |     1 |
| `crates/cityloom_editor/src/restriction/model.rs` | Modèle                |    26 |
| `crates/cityloom_editor/src/restriction/store.rs` | Modèle                |    23 |
| `e2e/tests/restrictions.spec.ts`                  | Niveau de l’interface |    18 |

### `map.report`

Le rapport d’examen de l’aménagement se télécharge en PDF, dans la langue de la page.

- Tests Rust : 13 (modèle de vue 12, rapport et PDF 1)
- Tests Playwright : 7 (niveau de l’interface)

| Fichier de test ou spécification                     | Palier                | Tests |
| ---------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/map/vm/report.rs`        | Modèle de vue         |     5 |
| `crates/cityloom_editor/src/map/vm/report_review.rs` | Modèle de vue         |     7 |
| `crates/cityloom_report/src/lib.rs`                  | Rapport et PDF        |     1 |
| `e2e/tests/report.spec.ts`                           | Niveau de l’interface |     7 |

### `city.area`

La zone d’un projet est tirée de ses tuiles, découpée et transformée en une ville de rues et de jonctions, puis refaite quand sa tuile change.

- Tests Rust : 185 (modèle)
- Tests Playwright : 4 (niveau de l’interface)

| Fichier de test ou spécification                        | Palier                | Tests |
| ------------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/city/import.rs`             | Modèle                |    32 |
| `crates/cityloom_editor/src/city/model.rs`              | Modèle                |   105 |
| `crates/cityloom_editor/src/city/model/neighbours.rs`   | Modèle                |     6 |
| `crates/cityloom_editor/src/city/model/network_read.rs` | Modèle                |     2 |
| `crates/cityloom_editor/src/city/projection.rs`         | Modèle                |     5 |
| `crates/cityloom_editor/src/place/area.rs`              | Modèle                |     9 |
| `crates/cityloom_editor/src/place/loader.rs`            | Modèle                |    13 |
| `crates/cityloom_editor/src/place/mod.rs`               | Modèle                |     6 |
| `crates/cityloom_editor/src/place/tiles.rs`             | Modèle                |     7 |
| `e2e/tests/world.spec.ts`                               | Niveau de l’interface |     4 |

### `city.edits`

Les modifications d’une rue ou d’une jonction sont conservées dans le navigateur sous ses identifiants OpenStreetMap, et survivent au rechargement.

- Tests Rust : 160 (modèle)
- Tests Playwright : 10 (niveau de l’interface)

| Fichier de test ou spécification              | Palier                | Tests |
| --------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/city/binding.rs`  | Modèle                |     6 |
| `crates/cityloom_editor/src/city/edits.rs`    | Modèle                |    76 |
| `crates/cityloom_editor/src/city/keys.rs`     | Modèle                |     9 |
| `crates/cityloom_editor/src/city/store.rs`    | Modèle                |    61 |
| `crates/cityloom_editor/src/shared/scoped.rs` | Modèle                |     7 |
| `crates/cityloom_editor/src/shared/store.rs`  | Modèle                |     1 |
| `e2e/tests/edits.spec.ts`                     | Niveau de l’interface |     6 |
| `e2e/tests/world.spec.ts`                     | Niveau de l’interface |     4 |

### `data.importer`

Les routes d’OpenStreetMap deviennent le réseau de rues que lit l’éditeur.

- Tests Rust : 114 (`citymodel_builder` et `cityloom_model`)
- Tests Playwright : 0

| Fichier de test ou spécification                  | Palier                                  | Tests |
| ------------------------------------------------- | --------------------------------------- | ----: |
| `crates/cityloom_model/src/catalogue.rs`          | `citymodel_builder` et `cityloom_model` |     2 |
| `crates/cityloom_model/src/corridor.rs`           | `citymodel_builder` et `cityloom_model` |     4 |
| `crates/cityloom_model/src/cut.rs`                | `citymodel_builder` et `cityloom_model` |    21 |
| `crates/cityloom_model/src/frame.rs`              | `citymodel_builder` et `cityloom_model` |     4 |
| `crates/cityloom_model/src/graph.rs`              | `citymodel_builder` et `cityloom_model` |     5 |
| `crates/cityloom_model/src/hours.rs`              | `citymodel_builder` et `cityloom_model` |     3 |
| `crates/cityloom_model/src/importer.rs`           | `citymodel_builder` et `cityloom_model` |     2 |
| `crates/cityloom_model/src/junction.rs`           | `citymodel_builder` et `cityloom_model` |     2 |
| `crates/cityloom_model/src/lane.rs`               | `citymodel_builder` et `cityloom_model` |     3 |
| `crates/cityloom_model/src/osm.rs`                | `citymodel_builder` et `cityloom_model` |     1 |
| `crates/cityloom_model/src/point.rs`              | `citymodel_builder` et `cityloom_model` |     3 |
| `crates/cityloom_model/src/provenance.rs`         | `citymodel_builder` et `cityloom_model` |     4 |
| `crates/cityloom_model/src/road.rs`               | `citymodel_builder` et `cityloom_model` |     7 |
| `crates/cityloom_model/src/road_type.rs`          | `citymodel_builder` et `cityloom_model` |     2 |
| `crates/citymodel_builder/src/importer/blocks.rs` | `citymodel_builder` et `cityloom_model` |    14 |
| `crates/citymodel_builder/src/importer/merge.rs`  | `citymodel_builder` et `cityloom_model` |    14 |
| `crates/citymodel_builder/src/importer/mod.rs`    | `citymodel_builder` et `cityloom_model` |    23 |

### `data.archive`

Le réseau est découpé en tuiles et regroupé dans l’archive de la ville, que le serveur sert.

- Tests Rust : 50 (`citymodel_builder` et `cityloom_model`)
- Tests Playwright : 3 (niveau de bout en bout)

| Fichier de test ou spécification                 | Palier                                  | Tests |
| ------------------------------------------------ | --------------------------------------- | ----: |
| `crates/citymodel_builder/src/archive/index.rs`  | `citymodel_builder` et `cityloom_model` |    11 |
| `crates/citymodel_builder/src/download.rs`       | `citymodel_builder` et `cityloom_model` |     1 |
| `crates/citymodel_builder/src/metro_index.rs`    | `citymodel_builder` et `cityloom_model` |     3 |
| `crates/citymodel_builder/src/network_tiles.rs`  | `citymodel_builder` et `cityloom_model` |     5 |
| `crates/citymodel_builder/src/progress/frame.rs` | `citymodel_builder` et `cityloom_model` |    25 |
| `crates/citymodel_builder/src/progress/mod.rs`   | `citymodel_builder` et `cityloom_model` |     3 |
| `crates/citymodel_builder/src/progress/view.rs`  | `citymodel_builder` et `cityloom_model` |     2 |
| `e2e/tests/archive.spec.ts`                      | Niveau de bout en bout                  |     3 |

### `projects.list`

La page d’accueil liste les projets de la personne; un projet se crée dans une région métropolitaine et s’ouvre sur sa carte.

- Tests Rust : 139 (modèle 22, composant 117)
- Tests Playwright : 17 (niveau de l’interface)

| Fichier de test ou spécification                  | Palier                | Tests |
| ------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/project/copies.rs`    | Modèle                |    13 |
| `crates/cityloom_editor/src/project/tests.rs`     | Composant             |   117 |
| `crates/cityloom_editor/src/shared/project_id.rs` | Modèle                |     9 |
| `e2e/tests/home.spec.ts`                          | Niveau de l’interface |     5 |
| `e2e/tests/projects.spec.ts`                      | Niveau de l’interface |    12 |

### `projects.timeline`

Les jalons d’un projet conservent son travail à un moment donné, et se comparent au travail actuel.

- Tests Rust : 43 (modèle 23, composant 20)
- Tests Playwright : 7 (niveau de l’interface)

| Fichier de test ou spécification               | Palier                | Tests |
| ---------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/timeline/model.rs` | Modèle                |     2 |
| `crates/cityloom_editor/src/timeline/store.rs` | Modèle                |     9 |
| `crates/cityloom_editor/src/timeline/tests.rs` | Composant             |    20 |
| `crates/cityloom_editor/src/timeline/work.rs`  | Modèle                |    12 |
| `e2e/tests/timeline.spec.ts`                   | Niveau de l’interface |     7 |

### `projects.design-archive`

L’aménagement d’un projet s’exporte en un seul fichier et s’importe sur un autre appareil.

- Tests Rust : 29 (composant)
- Tests Playwright : 4 (niveau de l’interface)

| Fichier de test ou spécification              | Palier                | Tests |
| --------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/archive/tests.rs` | Composant             |    29 |
| `e2e/tests/design-archive.spec.ts`            | Niveau de l’interface |     4 |

### `projects.share`

Un projet se partage par un lien qui l’ouvre en lecture seule, et un aménagement partagé se copie dans ses propres projets.

- Tests Rust : 60 (modèle 15, composant 37, serveur 8)
- Tests Playwright : 9 (niveau d’intégration)

| Fichier de test ou spécification               | Palier               | Tests |
| ---------------------------------------------- | -------------------- | ----: |
| `crates/cityloom_editor/src/project/copies.rs` | Modèle               |    13 |
| `crates/cityloom_editor/src/share/store.rs`    | Modèle               |     2 |
| `crates/cityloom_editor/src/share/tests.rs`    | Composant            |    37 |
| `crates/cityloom_server/src/share.rs`          | Serveur              |     8 |
| `e2e/tests/share.spec.ts`                      | Niveau d’intégration |     9 |

### `projects.read-only`

Un aménagement ouvert en lecture seule se consulte et on peut y choisir des éléments, mais aucune commande ne le modifie.

- Tests Rust : 1 (modèle)
- Tests Playwright : 5 (niveau de l’interface)

| Fichier de test ou spécification              | Palier                | Tests |
| --------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/locked.rs` | Modèle                |     1 |
| `e2e/tests/read-only.spec.ts`                 | Niveau de l’interface |     5 |

### `account.sign-in`

Une personne se connecte par un fournisseur, associe ou dissocie des connexions, choisit où conserver son travail et supprime son compte.

- Tests Rust : 105 (modèle 20, modèle de vue 25, composant 15, serveur 45)
- Tests Playwright : 19 (niveau de l’interface 7, niveau d’intégration 12)

| Fichier de test ou spécification                    | Palier                | Tests |
| --------------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/account/tests.rs`       | Composant             |    15 |
| `crates/cityloom_editor/src/account/vm.rs`          | Modèle de vue         |    25 |
| `crates/cityloom_editor/src/shell/session.rs`       | Modèle                |    20 |
| `crates/cityloom_server/src/accounts.rs`            | Serveur               |     9 |
| `crates/cityloom_server/src/crypto.rs`              | Serveur               |    10 |
| `crates/cityloom_server/src/db.rs`                  | Serveur               |     1 |
| `crates/cityloom_server/src/flow.rs`                | Serveur               |     6 |
| `crates/cityloom_server/src/providers.rs`           | Serveur               |     4 |
| `crates/cityloom_server/src/providers/apple.rs`     | Serveur               |     4 |
| `crates/cityloom_server/src/providers/microsoft.rs` | Serveur               |     3 |
| `crates/cityloom_server/src/session.rs`             | Serveur               |     8 |
| `e2e/tests/account.spec.ts`                         | Niveau de l’interface |     7 |
| `e2e/tests/account-server.spec.ts`                  | Niveau d’intégration  |    12 |

### `account.sync`

Le travail d’une personne connectée est conservé sur le serveur et fusionné entre ses appareils, même après un travail hors ligne.

- Tests Rust : 113 (modèle 50, composant 54, serveur 9)
- Tests Playwright : 13 (niveau d’intégration)

| Fichier de test ou spécification           | Palier               | Tests |
| ------------------------------------------ | -------------------- | ----: |
| `crates/cityloom_editor/src/sync/hub.rs`   | Modèle               |     4 |
| `crates/cityloom_editor/src/sync/local.rs` | Modèle               |    24 |
| `crates/cityloom_editor/src/sync/model.rs` | Modèle               |    22 |
| `crates/cityloom_editor/src/sync/tests.rs` | Composant            |    54 |
| `crates/cityloom_server/src/work.rs`       | Serveur              |     9 |
| `e2e/tests/sync.spec.ts`                   | Niveau d’intégration |    13 |

### `shell.settings`

Chaque page a la barre du haut, le menu des réglages (thème, unités, région, raccourcis clavier), le panneau des détails et le tiroir des notes.

- Tests Rust : 118 (modèle 44, modèle de vue 30, composant 44)
- Tests Playwright : 84 (niveau de l’interface)

| Fichier de test ou spécification                | Palier                | Tests |
| ----------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/shortcut.rs` | Modèle                |     1 |
| `crates/cityloom_editor/src/shared/units.rs`    | Modèle                |    17 |
| `crates/cityloom_editor/src/shell/header.rs`    | Modèle                |     1 |
| `crates/cityloom_editor/src/shell/popover.rs`   | Modèle                |     4 |
| `crates/cityloom_editor/src/shell/session.rs`   | Modèle                |    20 |
| `crates/cityloom_editor/src/shell/share.rs`     | Modèle                |     1 |
| `crates/cityloom_editor/src/shell/tests.rs`     | Composant             |    44 |
| `crates/cityloom_editor/src/shell/vm.rs`        | Modèle de vue         |    30 |
| `e2e/tests/shell.spec.ts`                       | Niveau de l’interface |    47 |
| `e2e/tests/editors.spec.ts`                     | Niveau de l’interface |    26 |
| `e2e/tests/footer.spec.ts`                      | Niveau de l’interface |    11 |

### `shell.language`

Chaque page de l’application se lit en anglais ou en français canadien, et le choix est conservé.

- Tests Rust : 23 (modèle)
- Tests Playwright : 21 (niveau de l’interface)

| Fichier de test ou spécification            | Palier                | Tests |
| ------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/i18n.rs` | Modèle                |    18 |
| `crates/cityloom_editor/src/shared/said.rs` | Modèle                |     5 |
| `e2e/tests/language.spec.ts`                | Niveau de l’interface |    21 |

### `shell.accessibility`

Chaque page s’utilise au clavier et avec un lecteur d’écran, à tout zoom et toute largeur, et ses couleurs respectent les WCAG 2.2 AA.

- Tests Rust : 28 (garde)
- Tests Playwright : 31 (niveau de l’interface)

| Fichier de test ou spécification                | Palier                | Tests |
| ----------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/shared/contrast.rs` | Garde                 |    28 |
| `e2e/tests/a11y.spec.ts`                        | Niveau de l’interface |    21 |
| `e2e/tests/focus-touch.spec.ts`                 | Niveau de l’interface |    10 |

### `pages.about`

La page À propos dit à quoi sert CityLoom, sur la ville du dernier projet ouvert.

- Tests Rust : 14 (modèle)
- Tests Playwright : 16 (niveau de l’interface)

| Fichier de test ou spécification             | Palier                | Tests |
| -------------------------------------------- | --------------------- | ----: |
| `crates/cityloom_editor/src/map/view/svg.rs` | Modèle                |    14 |
| `e2e/tests/about.spec.ts`                    | Niveau de l’interface |    16 |

### `pages.credits`

La page des remerciements nomme les données et les logiciels dont CityLoom est fait.

- Tests Rust : 0
- Tests Playwright : 0

### `server.delivery`

Le serveur livre rapidement le site et les données de la ville : adresses versionnées, compression et plages d’octets.

- Tests Rust : 22 (serveur)
- Tests Playwright : 18 (niveau de bout en bout)

| Fichier de test ou spécification       | Palier                 | Tests |
| -------------------------------------- | ---------------------- | ----: |
| `crates/cityloom_server/src/app.rs`    | Serveur                |     5 |
| `crates/cityloom_server/src/config.rs` | Serveur                |    17 |
| `e2e/tests/delivery.spec.ts`           | Niveau de bout en bout |    14 |
| `e2e/tests/serve.spec.ts`              | Niveau de bout en bout |     4 |

## Non associés

Ces tests n’appartiennent à aucune fonctionnalité : aucun motif de `scripts/ci/features.json` ne correspond à leur
fichier. Avec eux, chaque test est dans cette page : 3 666 tests Rust associés et 886 non associés font les
4 552, les 607 tests Playwright sont tous associés, et les 226 tests des scripts sont tous non associés. Cela fait
4 273 tests associés et 1 112 non associés.

### Tests Rust non associés

57 fichiers, 886 tests (modèle 164, modèle de vue 20, composant 307, garde 25, serveur 180, `citymodel_builder` et `cityloom_model` 167, rapport et PDF 23).

| Fichier de test                                          | Palier                                  |   Tests |
| -------------------------------------------------------- | --------------------------------------- | ------: |
| `crates/cityloom_editor/src/atlas_contract.rs`           | Garde                                   |       7 |
| `crates/cityloom_editor/src/city/mod.rs`                 | Modèle                                  |       5 |
| `crates/cityloom_editor/src/i18n_guard.rs`               | Garde                                   |      14 |
| `crates/cityloom_editor/src/layout_guard.rs`             | Garde                                   |       3 |
| `crates/cityloom_editor/src/lib.rs`                      | Modèle                                  |       3 |
| `crates/cityloom_editor/src/map/view/tests.rs`           | Composant                               |     122 |
| `crates/cityloom_editor/src/shared/atlas.rs`             | Modèle                                  |       3 |
| `crates/cityloom_editor/src/shared/core.rs`              | Modèle                                  |       3 |
| `crates/cityloom_editor/src/shared/diagram/mod.rs`       | Modèle                                  |       2 |
| `crates/cityloom_editor/src/shared/diagram/render.rs`    | Modèle                                  |      13 |
| `crates/cityloom_editor/src/shared/flat.rs`              | Modèle                                  |      21 |
| `crates/cityloom_editor/src/shared/keeper.rs`            | Modèle                                  |       3 |
| `crates/cityloom_editor/src/shared/platform.rs`          | Modèle                                  |       5 |
| `crates/cityloom_editor/src/shared/ports.rs`             | Modèle                                  |      31 |
| `crates/cityloom_editor/src/shared/provenance.rs`        | Modèle                                  |       1 |
| `crates/cityloom_editor/src/shared/random.rs`            | Modèle                                  |       1 |
| `crates/cityloom_editor/src/shared/sources.rs`           | Modèle                                  |       8 |
| `crates/cityloom_editor/src/shared/sources_view.rs`      | Modèle                                  |       3 |
| `crates/cityloom_editor/src/shared/symbols.rs`           | Modèle                                  |      13 |
| `crates/cityloom_editor/src/shared/tick.rs`              | Modèle                                  |       3 |
| `crates/cityloom_editor/src/street/model/beyond.rs`      | Modèle                                  |      17 |
| `crates/cityloom_editor/src/street/model/matching.rs`    | Modèle                                  |       3 |
| `crates/cityloom_editor/src/street/model/replay.rs`      | Modèle                                  |      15 |
| `crates/cityloom_editor/src/street/view/notes.rs`        | Modèle                                  |       9 |
| `crates/cityloom_editor/src/street/view/scroll.rs`       | Modèle                                  |       1 |
| `crates/cityloom_editor/src/street/view/tests.rs`        | Composant                               |     185 |
| `crates/cityloom_editor/src/street/view/transform.rs`    | Modèle                                  |       1 |
| `crates/cityloom_editor/src/street/vm/text.rs`           | Modèle de vue                           |      20 |
| `crates/cityloom_report_pdf/tests/pdf.rs`                | Rapport et PDF                          |      23 |
| `crates/cityloom_server/src/commands.rs`                 | Serveur                                 |       4 |
| `crates/cityloom_server/src/fake_provider.rs`            | Serveur                                 |       7 |
| `crates/cityloom_server/src/lib.rs`                      | Serveur                                 |       1 |
| `crates/cityloom_server/tests/api.rs`                    | Serveur                                 |      10 |
| `crates/cityloom_server/tests/areas.rs`                  | Serveur                                 |       4 |
| `crates/cityloom_server/tests/auth_flow.rs`              | Serveur                                 |      25 |
| `crates/cityloom_server/tests/data_mount.rs`             | Serveur                                 |       9 |
| `crates/cityloom_server/tests/db.rs`                     | Serveur                                 |      10 |
| `crates/cityloom_server/tests/no_plaintext.rs`           | Garde                                   |       1 |
| `crates/cityloom_server/tests/providers.rs`              | Serveur                                 |       8 |
| `crates/cityloom_server/tests/rotate.rs`                 | Serveur                                 |       4 |
| `crates/cityloom_server/tests/share_api.rs`              | Serveur                                 |      13 |
| `crates/cityloom_server/tests/share_db.rs`               | Serveur                                 |      10 |
| `crates/cityloom_server/tests/static_files.rs`           | Serveur                                 |      27 |
| `crates/cityloom_server/tests/unavailable.rs`            | Serveur                                 |       3 |
| `crates/cityloom_server/tests/work_api.rs`               | Serveur                                 |      20 |
| `crates/cityloom_server/tests/work_db.rs`                | Serveur                                 |      25 |
| `crates/citymodel_builder/tests/archive.rs`              | `citymodel_builder` et `cityloom_model` |      33 |
| `crates/citymodel_builder/tests/commands.rs`             | `citymodel_builder` et `cityloom_model` |      14 |
| `crates/citymodel_builder/tests/ctrl_c.rs`               | `citymodel_builder` et `cityloom_model` |       2 |
| `crates/citymodel_builder/tests/download.rs`             | `citymodel_builder` et `cityloom_model` |      17 |
| `crates/citymodel_builder/tests/importer/clip_agrees.rs` | `citymodel_builder` et `cityloom_model` |       3 |
| `crates/citymodel_builder/tests/importer/import.rs`      | `citymodel_builder` et `cityloom_model` |      56 |
| `crates/citymodel_builder/tests/importer/projection.rs`  | `citymodel_builder` et `cityloom_model` |       1 |
| `crates/citymodel_builder/tests/importer/real_city.rs`   | `citymodel_builder` et `cityloom_model` |      11 |
| `crates/citymodel_builder/tests/prepare.rs`              | `citymodel_builder` et `cityloom_model` |       7 |
| `crates/citymodel_builder/tests/progress.rs`             | `citymodel_builder` et `cityloom_model` |      14 |
| `crates/citymodel_builder/tests/tiles.rs`                | `citymodel_builder` et `cityloom_model` |       9 |
| **Total**                                                |                                         | **886** |

### Tests Playwright non associés

Aucun : chaque spécification d’`e2e/tiers.json` est dans une fonctionnalité, et
`scripts/include/tests/test_features.py` échoue sinon.

### Tests des scripts non associés

Aucun motif ne couvre `scripts/`.

| Fichier de test                                   |   Tests |
| ------------------------------------------------- | ------: |
| `scripts/include/tests/test_architecture_docs.py` |      19 |
| `scripts/include/tests/test_ci_changes.py`        |      71 |
| `scripts/include/tests/test_factory_config.py`    |       7 |
| `scripts/include/tests/test_factory_gate.py`      |      63 |
| `scripts/include/tests/test_factory_guard.py`     |      12 |
| `scripts/include/tests/test_features.py`          |      21 |
| `scripts/include/tests/test_issue_system.py`      |      33 |
| **Total**                                         | **226** |

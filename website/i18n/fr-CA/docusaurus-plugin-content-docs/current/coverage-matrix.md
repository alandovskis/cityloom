---
id: coverage-matrix
title: Matrice de couverture
---

# Matrice de couverture

Cette page met les fonctionnalités de CityLoom en regard des paliers de tests, avec le nombre de tests dans chaque
case. Elle donne aussi les tests qui n’appartiennent à aucune fonctionnalité, et la couverture de lignes que mesure
SonarCloud. Les nombres sont copiés de l’[inventaire des tests](test-inventory.md), qui dit comment les tests sont
comptés et nomme leurs fichiers. Ce sont ceux du commit `0d9cb8b9` (version 0.113.0). Aucun script ne produit ni ne
vérifie ces nombres : ils restent donc ceux de ce commit jusqu’au prochain décompte. Ce que les tests ne couvrent pas
est dans [Lacunes et risques connus](testing.md#lacunes-et-risques-connus).

## Fonctionnalités et paliers

Une ligne est l’une des 45 fonctionnalités de `scripts/ci/features.json`, dans son ordre, et mène aux fichiers de test
de la fonctionnalité dans l’inventaire. Une colonne est l’un des 11 [paliers](test-inventory.md#par-palier). Une case
est le nombre de tests de la fonctionnalité à ce palier.

- Un test de plusieurs fonctionnalités compte dans chacune.
- Un zéro n’est pas en soi une lacune. Le palier d’un test vient de l’endroit où se trouve son fichier, et ses
  fonctionnalités, du code et des spécifications que nomme `scripts/ci/features.json` : ce que fait une fonctionnalité
  peut donc être testé par des tests que sa ligne ne compte pas. Par exemple :
  - Les cases Composant de toutes les fonctionnalités `map`, et de toutes les fonctionnalités `street` sauf
    `street.standards`, sont à 0. Les tests de composant des deux pages sont `street/view/tests.rs` (185 tests) et
    `map/view/tests.rs` (122 tests) dans `crates/cityloom_editor/src/`, qui ne sont pas associés.
  - `pages.credits` est à 0 à chaque palier. `e2e/tests/home.spec.ts` et `e2e/tests/about.spec.ts`, qui appartiennent
    à d’autres fonctionnalités, ouvrent `/credits.html`.
  - Les cases Serveur de `projects.share`, `account.sync` et `account.sign-in` laissent de côté `share_api.rs`,
    `work_api.rs` et `auth_flow.rs` de `crates/cityloom_server/tests/`, qui ne sont pas associés.

| Fonctionnalité                                                             |    Modèle | Modèle de vue | Composant |  Garde | Serveur | `citymodel_builder` et `cityloom_model` | Rapport et PDF | Script | Niveau de l’interface | Niveau d’intégration | Niveau de bout en bout |     Total |
| -------------------------------------------------------------------------- | --------: | ------------: | --------: | -----: | ------: | --------------------------------------: | -------------: | -----: | --------------------: | -------------------: | ---------------------: | --------: |
| [`street.pieces`](test-inventory.md#streetpieces)                          |       164 |           178 |         0 |      0 |       0 |                                      19 |              0 |      0 |                    97 |                    0 |                      0 |       458 |
| [`street.drawing`](test-inventory.md#streetdrawing)                        |       240 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    14 |                    0 |                      0 |       254 |
| [`street.stretches`](test-inventory.md#streetstretches)                    |        95 |            23 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    92 |                    0 |                      0 |       210 |
| [`street.speed`](test-inventory.md#streetspeed)                            |        14 |             0 |         0 |      0 |       0 |                                       1 |              0 |      0 |                    10 |                    0 |                      0 |        25 |
| [`street.curbside`](test-inventory.md#streetcurbside)                      |        68 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     5 |                    0 |                      0 |        73 |
| [`street.stops`](test-inventory.md#streetstops)                            |        52 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     6 |                    0 |                      0 |        58 |
| [`street.calming`](test-inventory.md#streetcalming)                        |        58 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    10 |                    0 |                      0 |        68 |
| [`street.green`](test-inventory.md#streetgreen)                            |        64 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     7 |                    0 |                      0 |        71 |
| [`street.checks`](test-inventory.md#streetchecks)                          |        17 |            15 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    82 |                    0 |                      0 |       114 |
| [`street.templates`](test-inventory.md#streettemplates)                    |        27 |            23 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     5 |                    0 |                      0 |        55 |
| [`street.standards`](test-inventory.md#streetstandards)                    |        23 |             0 |        13 |      0 |       0 |                                       0 |              0 |      0 |                     7 |                    0 |                      0 |        43 |
| [`junction.layout`](test-inventory.md#junctionlayout)                      |       469 |            93 |       235 |      0 |       0 |                                       0 |              0 |      0 |                   112 |                    0 |                      0 |       909 |
| [`junction.roundabout`](test-inventory.md#junctionroundabout)              |        40 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    81 |                    0 |                      0 |       121 |
| [`junction.signal.phases`](test-inventory.md#junctionsignalphases)         |        46 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    81 |                    0 |                      0 |       127 |
| [`junction.signal.plans`](test-inventory.md#junctionsignalplans)           |        14 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       100 |
| [`junction.signal.calls`](test-inventory.md#junctionsignalcalls)           |        33 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       119 |
| [`junction.signal.actuation`](test-inventory.md#junctionsignalactuation)   |        16 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       102 |
| [`junction.signal.bicycles`](test-inventory.md#junctionsignalbicycles)     |        14 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       100 |
| [`junction.signal.transit`](test-inventory.md#junctionsignaltransit)       |        16 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       102 |
| [`junction.signal.preemption`](test-inventory.md#junctionsignalpreemption) |        19 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    86 |                    0 |                      0 |       105 |
| [`design.scenarios`](test-inventory.md#designscenarios)                    |        10 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     9 |                    0 |                      0 |        19 |
| [`design.compare`](test-inventory.md#designcompare)                        |        44 |            21 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     8 |                    0 |                      0 |        73 |
| [`map.city`](test-inventory.md#mapcity)                                    |         6 |           191 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    39 |                    0 |                      0 |       236 |
| [`map.search`](test-inventory.md#mapsearch)                                |         2 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    13 |                    0 |                      0 |        15 |
| [`map.goals`](test-inventory.md#mapgoals)                                  |        13 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    38 |                    0 |                      0 |        51 |
| [`map.network-checks`](test-inventory.md#mapnetwork-checks)                |        30 |             0 |         0 |      0 |       0 |                                       4 |              0 |      0 |                     8 |                    0 |                      0 |        42 |
| [`map.area-rules`](test-inventory.md#maparea-rules)                        |        64 |            44 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    18 |                    0 |                      0 |       126 |
| [`map.report`](test-inventory.md#mapreport)                                |         0 |            12 |         0 |      0 |       0 |                                       0 |              1 |      0 |                     7 |                    0 |                      0 |        20 |
| [`city.area`](test-inventory.md#cityarea)                                  |       185 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     4 |                    0 |                      0 |       189 |
| [`city.edits`](test-inventory.md#cityedits)                                |       160 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    10 |                    0 |                      0 |       170 |
| [`data.importer`](test-inventory.md#dataimporter)                          |         0 |             0 |         0 |      0 |       0 |                                     114 |              0 |      0 |                     0 |                    0 |                      0 |       114 |
| [`data.archive`](test-inventory.md#dataarchive)                            |         0 |             0 |         0 |      0 |       0 |                                      50 |              0 |      0 |                     0 |                    0 |                      3 |        53 |
| [`projects.list`](test-inventory.md#projectslist)                          |        22 |             0 |       117 |      0 |       0 |                                       0 |              0 |      0 |                    17 |                    0 |                      0 |       156 |
| [`projects.timeline`](test-inventory.md#projectstimeline)                  |        23 |             0 |        20 |      0 |       0 |                                       0 |              0 |      0 |                     7 |                    0 |                      0 |        50 |
| [`projects.design-archive`](test-inventory.md#projectsdesign-archive)      |         0 |             0 |        29 |      0 |       0 |                                       0 |              0 |      0 |                     4 |                    0 |                      0 |        33 |
| [`projects.share`](test-inventory.md#projectsshare)                        |        15 |             0 |        37 |      0 |       8 |                                       0 |              0 |      0 |                     0 |                    9 |                      0 |        69 |
| [`projects.read-only`](test-inventory.md#projectsread-only)                |         1 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     5 |                    0 |                      0 |         6 |
| [`account.sign-in`](test-inventory.md#accountsign-in)                      |        20 |            25 |        15 |      0 |      45 |                                       0 |              0 |      0 |                     7 |                   12 |                      0 |       124 |
| [`account.sync`](test-inventory.md#accountsync)                            |        50 |             0 |        54 |      0 |       9 |                                       0 |              0 |      0 |                     0 |                   13 |                      0 |       126 |
| [`shell.settings`](test-inventory.md#shellsettings)                        |        44 |            30 |        44 |      0 |       0 |                                       0 |              0 |      0 |                    84 |                    0 |                      0 |       202 |
| [`shell.language`](test-inventory.md#shelllanguage)                        |        23 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    21 |                    0 |                      0 |        44 |
| [`shell.accessibility`](test-inventory.md#shellaccessibility)              |         0 |             0 |         0 |     28 |       0 |                                       0 |              0 |      0 |                    31 |                    0 |                      0 |        59 |
| [`pages.about`](test-inventory.md#pagesabout)                              |        14 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                    16 |                    0 |                      0 |        30 |
| [`pages.credits`](test-inventory.md#pagescredits)                          |         0 |             0 |         0 |      0 |       0 |                                       0 |              0 |      0 |                     0 |                    0 |                      0 |         0 |
| [`server.delivery`](test-inventory.md#serverdelivery)                      |         0 |             0 |         0 |      0 |      22 |                                       0 |              0 |      0 |                     0 |                    0 |                     18 |        40 |
| **Somme**                                                                  | **2 215** |       **655** |   **564** | **28** |  **84** |                                 **188** |          **1** |  **0** |             **1 471** |               **34** |                 **21** | **5 261** |
| **Tests associés distincts**                                               | **2 164** |       **644** |   **564** | **28** |  **84** |                                 **181** |          **1** |  **0** |               **552** |               **34** |                 **21** | **4 273** |

La somme dépasse de 988 les tests associés distincts, parce que [9 fichiers Rust et 8
spécifications](test-inventory.md#par-fonctionnalité) appartiennent à plusieurs fonctionnalités : 51 au palier Modèle,
11 au palier Modèle de vue, 7 au palier `citymodel_builder` et `cityloom_model`, et 919 au niveau de l’interface.

## Tests non associés

1 112 tests n’appartiennent à aucune fonctionnalité. L’inventaire nomme leurs fichiers dans [Non
associés](test-inventory.md#non-associés).

| Palier                                  | Tests associés distincts | Tests non associés | Tous les tests |
| --------------------------------------- | -----------------------: | -----------------: | -------------: |
| Modèle                                  |                    2 164 |                164 |          2 328 |
| Modèle de vue                           |                      644 |                 20 |            664 |
| Composant                               |                      564 |                307 |            871 |
| Garde                                   |                       28 |                 25 |             53 |
| Serveur                                 |                       84 |                180 |            264 |
| `citymodel_builder` et `cityloom_model` |                      181 |                167 |            348 |
| Rapport et PDF                          |                        1 |                 23 |             24 |
| Script                                  |                        0 |                226 |            226 |
| Niveau de l’interface                   |                      552 |                  0 |            552 |
| Niveau d’intégration                    |                       34 |                  0 |             34 |
| Niveau de bout en bout                  |                       21 |                  0 |             21 |
| **Total**                               |                **4 273** |          **1 112** |      **5 385** |

## Couverture de lignes

Les nombres ont été lus dans SonarCloud (le projet `alandovskis_cityloom`) le 23 novembre 2026 à 8 h 22 UTC. La
dernière analyse était alors celle du 23 novembre 2026 à 7 h 25 min 52 s UTC, de la révision `0d9cb8b9`, le commit des
nombres ci-dessus.

- C’est la couverture des tests sur l’hôte seulement, sans `cityloom_server` (`just sonar-reports`, qu’exécute
  `.github/workflows/sonar.yml`). Les tests dans le navigateur et les tests des scripts n’y sont pas.
- `cityloom_server`, `cityloom_report` et `cityloom_report_pdf` ne sont pas mesurés : `sonar.sources`, dans
  `sonar-project.properties`, les laisse de côté. Le code sous `cfg(target_arch = "wasm32")` n’est pas mesuré non
  plus.
- Le 0,0 de `web/` n’est pas une mesure de ses tests : le seul rapport de couverture est celui de Rust, donc rien ne
  mesure `web/`.
- `sonar.coverage.exclusions` laisse de côté `**/tests.rs`, `i18n_guard.rs` et `scripts/**`. Les lignes d’un module
  `#[cfg(test)]` placé dans le code comptent comme des lignes.

### Par caisse

| Caisse                  | Couverture de lignes (%) | Lignes à couvrir | Lignes non couvertes |
| ----------------------- | -----------------------: | ---------------: | -------------------: |
| `cityloom_editor`       |                     95,3 |           79 034 |                3 689 |
| `cityloom_model`        |                     99,2 |            1 851 |                   15 |
| `citymodel_builder`     |                     97,6 |            3 147 |                   77 |
| `cityloom_server`       |               non mesuré |       non mesuré |           non mesuré |
| `cityloom_report`       |               non mesuré |       non mesuré |           non mesuré |
| `cityloom_report_pdf`   |               non mesuré |       non mesuré |           non mesuré |
| `web/` (pas une caisse) |                      0,0 |              365 |                  365 |
| **Total du projet**     |                 **95,1** |       **84 397** |            **4 146** |

### Par tranche de l’éditeur

Les nombres d’une tranche comprennent ses sous-dossiers.

| Tranche                                      | Couverture de lignes (%) | Lignes à couvrir | Lignes non couvertes |
| -------------------------------------------- | -----------------------: | ---------------: | -------------------: |
| `account`                                    |                     94,5 |              763 |                   42 |
| `archive`                                    |                     93,8 |              464 |                   29 |
| `city`                                       |                     99,3 |            6 246 |                   45 |
| `junction`                                   |                     96,7 |           21 473 |                  717 |
| `map`                                        |                     96,6 |            8 348 |                  287 |
| `place`                                      |                    100,0 |              606 |                    0 |
| `project`                                    |                     92,7 |            1 206 |                   88 |
| `restriction`                                |                     97,9 |            1 119 |                   24 |
| `share`                                      |                     93,6 |              629 |                   40 |
| `shared` (le noyau partagé, pas une tranche) |                     95,0 |            5 791 |                  287 |
| `shell`                                      |                     79,3 |            1 731 |                  358 |
| `standards`                                  |                     87,8 |            1 152 |                  141 |
| `street`                                     |                     94,8 |           26 494 |                1 365 |
| `sync`                                       |                     96,9 |            1 554 |                   48 |
| `timeline`                                   |                     91,4 |            1 133 |                   98 |
| Les fichiers à la racine de `src/`           |                        – |              325 |                  120 |
| **`cityloom_editor`**                        |                 **95,3** |       **79 034** |            **3 689** |

Les fichiers à la racine de `src/` sont ce que les dossiers laissent des lignes de la caisse. C’est surtout `lib.rs` :
180 lignes à couvrir, dont 119 non couvertes, soit une couverture de lignes de 33,9 %.

## Les lacunes

Une lacune est un manquement à [Où va un nouveau test](testing.md#où-va-un-nouveau-test), c’est-à-dire à une ligne de
son tableau ou à sa liste de ce que seuls les tests dans le navigateur atteignent, confirmé dans le code. Les 23
lacunes, chacune avec son billet, sont dans [Lacunes et risques connus](testing.md#lacunes-et-risques-connus).

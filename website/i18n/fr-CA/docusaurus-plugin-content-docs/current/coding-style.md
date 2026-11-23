---
id: coding-style
title: Style du code
---

# Style du code

Cette page décrit le style que le code de CityLoom a déjà. Une règle d’ici est ce que fait la plus grande partie du
code : certains fichiers s’en écartent donc. Là où le code n’a pas de façon établie, la page n’énonce aucune règle.

Chaque règle se termine par ce qui l’applique :

- **Outil** : un outil de mise en forme, le compilateur ou un script la vérifie.
- **Garde** : un test sur le code la vérifie (voir [Les gardes](testing.md#les-gardes)); la règle nomme le test.
- **Revue** : seule la revue de la demande de fusion la vérifie.

L’organisation du code (tranches, couches, ports) est décrite dans [Architecture](architecture/index.md). La façon dont
il est testé est décrite dans [Tests](testing.md). Les termes français et les espacements sont dans
[`docs/glossary-fr-ca.md`](https://github.com/alandovskis/cityloom/blob/main/docs/glossary-fr-ca.md). Les chemins des
fichiers de l’éditeur sont relatifs à `crates/cityloom_editor/src/`.

## Ce que les outils décident

- rustfmt met en forme le Rust, et Prettier met en forme `e2e/`, `web/*.js` et `website/`. `just format` exécute les
  deux et `just format-check` ne fait que vérifier, comme l’intégration continue (voir
  [Tests et mise en forme](getting-started.md#tests-et-mise-en-forme)). Exemple : `justfile` (`format`). **Outil**
- Le code compile sans avertissement : l’intégration continue refuse chaque avertissement du compilateur. Exemple :
  `.github/workflows/presubmit.yml` (`RUSTFLAGS`). **Outil**
- Aucun outil de mise en forme ne couvre `web/*.css`, le HTML, `scripts/`, le SQL, les fichiers `.ftl`, ni le Markdown
  hors de `website/` et de `e2e/` : ils sont mis en forme à la main, comme les lignes qui les entourent. Exemple :
  `web/app.css` (`:root`). **Revue**

Clippy ne refuse rien : il ne s’exécute que pour [SonarCloud](testing.md#sonarcloud), ses constats plafonnés au niveau
d’avertissement.

## Rust

### Noms

- Les types, les fonctions et les constantes ont les noms que Rust attend : le compilateur avertit de tout autre nom.
  Exemple : `street/model/pick.rs` (`PickRefusal`, `said`). **Outil**
- Le type d’un modèle de vue se termine par `Vm`. Exemple : `project/vm.rs` (`ProjectsVm`). **Revue**
- Un trait qui ajoute les règles de l’éditeur à un type de `cityloom_model` porte le nom du type suivi de `Ext`.
  Exemple : `street/model/curb.rs` (`UseExt`). **Revue**

### Modules et fichiers

- Dans l’éditeur, le serveur, `citymodel_builder` et les deux caisses du rapport, un fichier de `src/` s’ouvre sur un
  commentaire `//!` qui dit ce que contient le fichier. Les racines de caisse du serveur et de `citymodel_builder`
  (`lib.rs`, `main.rs`) n’en ont pas. Exemple : `shared/iso/format.rs` (le fichier de `esc`). **Revue**
- Les importations au début d’un fichier forment trois groupes, séparés par une ligne vide : la bibliothèque standard,
  les autres caisses, puis cette caisse. Exemple : `project/vm.rs` (les importations au-dessus de `ProjectsVm`).
  **Revue**
- Dans `street/`, `junction/` et `map/`, chaque couche a son propre dossier (voir
  [Couches dans une tranche](architecture/index.md#couches-dans-une-tranche)). Exemple : `layout_guard.rs`
  (`layered_slices_keep_their_files_in_model_vm_and_view`). **Garde** (`layout_guard.rs`)

### Erreurs et refus

- Il n’y a aucun `unsafe`. Exemple : `crates/cityloom_server/src/crypto.rs` (`seal`) chiffre sans lui. **Revue**
- Le message d’un `expect` énonce le fait qui rend l’échec impossible, en minuscules. Exemple :
  `crates/cityloom_server/src/crypto.rs` (`seal` : `"a key is 32 bytes"`). **Revue**
- Hors des tests, `unwrap()` sert à un `write!` dans une `String` et au verrou d’un mutex. Une donnée reçue ne passe
  jamais par `unwrap()`. Exemples : `map/view/svg.rs` (`map_svg`), `crates/cityloom_server/src/clock.rs` (`FixedClock`).
  **Revue**
- Un refus du modèle est une énumération avec une variante par raison. Exemple : `street/model/pick.rs`
  (`PickRefusal`). **Revue**
- Un refus ne contient aucune formulation : ses mots sont un message Fluent, atteint par sa clé. Exemple :
  `street/model/pick.rs` (`PickRefusal::said`). **Garde** (`i18n_guard.rs`) pour les fichiers de sa liste `MIGRATED`,
  **Revue** pour tout autre.

### Possession et état partagé

- L’éditeur partage une valeur avec `Rc`; le serveur et `citymodel_builder` la partagent avec `Arc`. Exemples :
  `street/vm/watch.rs` (`SheetWatch`), `crates/cityloom_server/src/app.rs` (`Account`). **Revue**
- Une valeur qui n’est pas `Send` va dans un `StoredValue::new_local`; `StoredValue::new` contient des valeurs simples.
  Exemple : `street/vm/watch.rs` (`SheetWatch::new`). **Outil**
- Une vue garde son modèle de vue dans une poignée `Copy` sur un `Rc`. Exemples : `junction/vm/watch.rs` (`Watch`),
  `street/vm/watch.rs` (`SheetWatch`), `shared/bind.rs` (`Bound`). **Revue**

### Constantes et unités

- Une constante d’un modèle qui contient une longueur, une durée ou une vitesse se termine par son suffixe d’unité :
  `_MM` ou `_M`, `_S`, `_KMH`. Exemples : `street/model/calming.rs` (`RAMP_MM`), `junction/model/mod.rs`
  (`GREEN_MIN_S`), `crates/citymodel_builder/src/importer/merge.rs` (`MAX_APART_M`). **Revue**
- Une longueur d’une rue ou d’une jonction, comme champ ou paramètre des modèles, est un nombre entier de millimètres
  dans un `i32`, et son nom se termine par `_mm`. Le code de dessin calcule en `f64`. Les longueurs du réseau importé
  sont des mètres en `f64`, nommées `_m`. Exemples : `crates/cityloom_model/src/street/calming.rs` (`length_mm`),
  `crates/cityloom_model/src/lane.rs` (`width_m`). **Revue**
- Une longueur est écrite pour la personne par `Units::*_in(…, locale)`, jamais par `format!("{:.1}")`, et un nombre que
  la personne saisit est lu par `parse_number`. Exemple : `shared/units.rs` (`length_in`, `parse_number`). **Revue**

## Leptos et SVG

- Une vue se lie à son modèle de vue et transmet les événements, et un modèle de vue n’atteint le navigateur que par les
  ports (voir [Couches dans une tranche](architecture/index.md#couches-dans-une-tranche) et
  [Ports](architecture/index.md#ports)). Exemple : `project/vm.rs` (`ProjectsVm`). **Revue**
- Un composant est nommé en `PascalCase`. Exemple : `map/view/network_view.rs` (`NetworkChecks`). **Revue**
- Une fonction de rendu va de ce qui est à dessiner au balisage SVG, sans Leptos, sans signal et sans DOM. Exemple :
  `street/view/plan/mod.rs` (`plan_svg`). **Revue**
- Une vue place ce balisage avec `inner_html`. Exemple : `street/view/plan/component.rs` (`inner_html`). **Revue**
- Un texte mis dans du balisage passe par `esc`. Exemple : `street/view/plan/mod.rs` (`thing`). **Revue**
- Une coordonnée est arrondie avant d’être écrite dans du balisage, par une fonction comme `f1`. Exemple :
  `shared/iso/format.rs` (`f1`). **Revue**

## Commentaires

- Un commentaire de documentation commence par une majuscule et se termine par un point. Exemple :
  `street/view/plan/mod.rs` (`plan_svg`). **Revue**
- Il dit ce qu’est la chose, par un groupe nominal (« The… », « A… », « What… », « Whether… »), ou ce qu’elle fait, par
  un verbe à la troisième personne (« Keeps… », « Draws… »). Exemple : `shared/iso/format.rs` (`f1`). **Revue**
- Le Rust n’a que des commentaires de ligne, aucun commentaire de bloc. Exemple : `shared/iso/format.rs` (`esc`).
  **Revue**
- Aucun `TODO` ni `FIXME` ne reste dans le Rust. Exemple : `shared/iso/format.rs` (`f1`). **Revue**

## Tests

L’endroit où va un nouveau test est indiqué dans [Tests](testing.md#où-va-un-nouveau-test).

- Le nom d’un test Rust est une phrase qui énonce le comportement, sans préfixe `test_`. Exemple :
  `crates/cityloom_server/src/crypto.rs` (`two_seals_of_one_value_differ`). **Revue**
- Les tests unitaires d’un fichier sont dans un `mod tests` à sa fin, avec `use super::*`. Exemple :
  `shared/iso/format.rs` (`tests`). **Revue**
- Une doublure d’un port est nommée d’après ce qu’elle fait à la place du port : `Fake…`, `Recording…`, `Memory…`,
  `Manual…`, `Fixed…`. Exemples : `shared/ports.rs` (`RecordingAnnouncer`), `crates/cityloom_server/src/clock.rs`
  (`FixedClock`). **Revue**

## Autres langages

### JavaScript

Ce qui relève de JavaScript est décrit dans [Architecture](architecture/index.md#javascript).

- Un script de `web/` s’ouvre sur un commentaire qui dit ce qu’il fait. Exemple : `web/mapcanvas.js` (le commentaire
  au-dessus de `createMapCanvas`). **Revue**
- Les scripts sont des modules ES, sauf `boot.js`, qui est un script classique. Exemple : `web/mapcanvas.js`
  (`createMapCanvas`). **Revue**
- Ils déclarent avec `const` et `let`, jamais `var`, et comparent avec `===`, jamais `==`. Exemple : `web/mapcanvas.js`
  (`here`, `themeNow`). **Revue**

### TypeScript

- Une spécification prend `test` et `expect` de `./fixtures`. Exemple : `e2e/tests/shell.spec.ts` (`expect`, `test`).
  **Revue**
- Le titre d’un test d’une spécification est une phrase qui énonce le comportement. Il commence par une minuscule, à
  moins que son premier mot ne soit un nom comme celui d’une touche, d’une commande ou d’une langue, et n’a pas de point
  final. Exemple : `e2e/tests/street.spec.ts`
  (`a piece is selected with the arrow keys, announced, and shown in the details`). **Revue**

### Python

- Un fichier Python de `scripts/` a une chaîne de documentation à son début et `from __future__ import annotations`, et
  n’utilise que la bibliothèque standard. Exemple : `scripts/ci/ci_changes.py` (`CI_CONFIGURATION`). **Revue**
- Un test est une méthode d’une `unittest.TestCase`, nommée `test_` suivi d’une phrase. Exemple :
  `scripts/include/tests/test_factory_gate.py` (`test_the_red_gate_passes_a_red_commit_whose_new_test_fails`).
  **Revue**

### Shell

- Un script est en `sh` POSIX, avec à son début un commentaire qui dit ce qu’il fait, et `set -eu`. Exemple :
  `scripts/check-version.sh` (`version_of`). **Revue**

### CSS

- Une feuille de style s’ouvre sur un commentaire. Exemple : `web/app.css` (au-dessus de `:root`). **Revue**
- Une couleur est une propriété personnalisée, utilisée avec `var(--…)`. Exemple : `web/app.css` (`--accent`).
  **Revue**
- Une paire de couleurs pour du texte ou une commande est mesurée dans `shared/contrast.rs`. Exemple :
  `shared/contrast.rs` (`contrast`). **Garde** (`shared/contrast.rs`) pour les paires qu’elle énumère, **Revue** pour
  toute autre.
- Une classe est nommée en kebab-case. Exemple : `web/editor.css` (`insp-meter`). **Revue**

### Migrations SQL

- Une migration est nommée `000N_name.sql` et s’ouvre sur un commentaire. Son SQL est en minuscules et ses noms sont en
  snake_case. Exemple : `crates/cityloom_server/migrations/0005_shares.sql` (`shares_user`). **Revue**
- Une migration fusionnée n’est pas modifiée : une nouvelle est ajoutée. Exemple :
  `crates/cityloom_server/migrations/0006_share_bounds.sql` supprime l’index `shares_user`, que `0005_shares.sql` a
  créé. **Revue**

### Messages Fluent

- Un fichier `.ftl` s’ouvre sur un commentaire qui dit ce qu’il met en mots. Exemple : `project/i18n/en.ftl` (le
  commentaire à son début). **Revue**
- L’identifiant d’un message est en kebab-case et une variable est en snake_case. Exemple : `junction/i18n/en.ftl`
  (`jn-calls-said`, `$not_served`). **Revue**
- Un message est une phrase entière avec des variables nommées, jamais des morceaux mis bout à bout, et un pluriel est un
  sélecteur (`[one]` / `*[other]`). Exemple : `project/i18n/en.ftl` (`project-delete-confirm-links`). **Revue**
- Un identifiant utilisé dans un fichier de la liste `MIGRATED` de la garde existe dans les deux langues. Exemple :
  `i18n_guard.rs` (`every_message_id_used_in_a_migrated_file_exists_in_both_languages`). **Garde** (`i18n_guard.rs`)
- Les deux langues ont les mêmes identifiants, avec les mêmes variables. Exemple : `shared/i18n.rs`
  (`parity_problems`). **Garde** (le test de parité de la tranche), **Revue** pour `timeline` et `standards`, qui n’ont
  pas de test de parité (voir [Traduction](testing.md#traduction)).

## Commits et demandes de fusion

- Le titre d’une demande de fusion est une ligne qui dit ce que fait le changement. Il commence par une
  majuscule et n’a ni point final ni préfixe de type comme `feat:`. Exemple :
  `.claude/skills/factory-evidence/SKILL.md` (`<what it does> (<version>) (CLM-<n>)`). **Revue**
- Après ce que fait le changement, le titre donne la version, quand la demande de fusion l’augmente, puis la clé du
  billet. GitHub ajoute le numéro. Exemple :
  `Keep a junction's folded options reachable by heading (0.113.1) (CLM-50) (#289)`. **Revue**
- Une demande de fusion fait ce que demande son billet et rien d’autre : aucun remaniement, renommage ni correctif
  ailleurs. Exemple : `.claude/agents/reviewer.md` (Scope). **Revue**
- Chaque changement est un commit distinct, fait une fois que ses tests réussissent. Exemple : `CLAUDE.md`
  (Conventions and gotchas). **Revue**
- Une demande de fusion augmente la version dans `crates/cityloom_editor/Cargo.toml` (majeure pour un changement
  incompatible, mineure pour une fonctionnalité, corrective pour un correctif) et met à jour `Cargo.lock`, à moins de ne
  changer que la documentation et la configuration de l’intégration continue (voir
  [Ce qui doit réussir avant une fusion](testing.md#ce-qui-doit-réussir-avant-une-fusion)). Exemple :
  `scripts/check-version.sh` (`version_of`). **Outil** (la tâche Version bump du flux de prévérification) pour la
  version, **Revue** pour `Cargo.lock`.

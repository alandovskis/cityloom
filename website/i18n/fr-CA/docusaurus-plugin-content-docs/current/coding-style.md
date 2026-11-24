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
  deux et `just format-check` ne fait que vérifier, comme l’intégration continue (voir [Tests et mise en
  forme](getting-started.md#tests-et-mise-en-forme)); pour `website/` seul, le flux de travail Documentation exécute
  la vérification de Prettier. Exemple : `justfile` (`format`). **Outil**
- Le code compile sans avertissement : l’intégration continue refuse chaque avertissement du compilateur. Exemple :
  `.github/workflows/presubmit.yml` (`RUSTFLAGS`). **Outil**
- Aucun outil de mise en forme ne couvre le reste, comme `web/*.css`, le HTML, `scripts/`, le SQL, les fichiers `.ftl`
  et le Markdown hors de `website/` et de `e2e/` : ils sont mis en forme à la main. Exemple : `web/app.css` (`:root`).
  **Revue**

Clippy ne refuse rien : il ne s’exécute que pour [SonarCloud](testing.md#sonarcloud), ses constats plafonnés au niveau
d’avertissement.

## Rust

### Noms

- Les types, les fonctions et les constantes ont les noms que Rust attend : le compilateur avertit d’un nom d’une
  autre forme. Exemple : `street/model/pick.rs` (`PickRefusal`, `said`). **Outil**
- Le type d’un modèle de vue se termine par `Vm`, sauf `PopoverViewModel` et `ShareViewModel` du shell. Exemple :
  `project/vm.rs` (`ProjectsVm`). **Revue**
- Un trait qui sert hors de son fichier et ajoute les règles de l’éditeur à un type de `cityloom_model` porte le nom
  du type suivi de `Ext`. Exemple : `street/model/curb.rs` (`UseExt`). **Revue**

### Modules et fichiers

- Dans l’éditeur, le serveur, `citymodel_builder` et les deux caisses du rapport, un fichier de `src/` s’ouvre sur un
  commentaire `//!` qui dit ce que contient le fichier. Les racines de caisse du serveur et de `citymodel_builder`
  (`lib.rs`, `main.rs`) n’en ont pas, ni certains `tests.rs` de l’éditeur. Exemple : `shared/iso/format.rs` (le
  fichier de `esc`). **Revue**
- Les importations au début de presque chaque fichier forment jusqu’à trois groupes, séparés par une ligne vide : la
  bibliothèque standard, les autres caisses, puis cette caisse. Exemple : `project/vm.rs` (les importations au-dessus
  de `ProjectsVm`). **Revue**
- Dans `street/`, `junction/` et `map/`, chaque couche a son propre dossier (voir [Couches dans une
  tranche](architecture/index.md#couches-dans-une-tranche)). Exemple : `layout_guard.rs`
  (`layered_slices_keep_their_files_in_model_vm_and_view`). **Garde** (`layout_guard.rs`)

### Erreurs et refus

- Il n’y a aucun `unsafe`. Exemple : `crates/cityloom_server/src/crypto.rs` (`seal`) chiffre sans lui. **Revue**
- Le message de presque chaque `expect` énonce le fait qui rend l’échec impossible, en minuscules sauf pour un nom
  comme `HMAC`. Exemple : `crates/cityloom_server/src/crypto.rs` (`seal` : `"a key is 32 bytes"`). **Revue**
- Hors des tests, presque chaque `unwrap()` sert à un `write!` dans une `String` ou au verrou d’un mutex. Une donnée
  reçue ne passe jamais par `unwrap()`. Exemples : `map/view/svg.rs` (`map_svg`),
  `crates/cityloom_server/src/clock.rs` (`FixedClock`). **Revue**
- Un refus du modèle est le plus souvent une énumération avec une variante par raison. Exemple :
  `street/model/pick.rs` (`PickRefusal`). **Revue**
- Les mots d’un tel refus sont un message, atteint par sa clé, et l’énumération n’en contient aucun (voir
  [Traduction](architecture/index.md#traduction)). Exemple : `street/model/pick.rs` (`PickRefusal::said`). **Garde**
  (`i18n_guard.rs`) contre une formulation dans les fichiers de sa liste `MIGRATED`, **Revue** pour le reste.

### Possession et état partagé

- L’éditeur partage presque chaque valeur avec `Rc`; le serveur et `citymodel_builder` partagent les leurs avec `Arc`.
  Exemples : `street/vm/watch.rs` (`SheetWatch`), `crates/cityloom_server/src/app.rs` (`Account`). **Revue**
- Une valeur qui n’est pas `Send`, gardée dans un `StoredValue`, y est mise avec `new_local`. Exemple :
  `street/vm/watch.rs` (`SheetWatch::new`). **Outil** (le compilateur)
- Un composant Leptos garde le plus souvent son modèle de vue dans une poignée `Copy` sur un `Rc`. Exemples :
  `junction/vm/watch.rs` (`Watch`), `street/vm/watch.rs` (`SheetWatch`), `shared/bind.rs` (`Bound`). **Revue**

### Constantes et unités

- Presque chaque constante d’un modèle qui contient une seule longueur, durée ou vitesse se termine par son unité,
  comme `_MM`, `_S` ou `_KMH`. Exemples : `street/model/calming.rs` (`RAMP_MM`, `MAX_TABLE_SPEED_KMH`),
  `junction/model/mod.rs` (`GREEN_MIN_S`). **Revue**
- Une longueur que conservent les documents de la rue et de la jonction est un nombre entier de millimètres dans un
  `i32`, dans un champ dont le nom se termine par `_mm` (sauf `cycle`, la largeur de la piste cyclable d’un carrefour
  giratoire). La géométrie qui en est calculée est souvent en `f64`, dans les modèles comme dans le code de dessin. Le
  réseau importé garde la largeur d’une voie en mètres, dans un `f64` (`width_m` dans
  `crates/cityloom_model/src/lane.rs`). Exemples : `crates/cityloom_model/src/street/calming.rs` (`length_mm`),
  `junction/model/mod.rs` (`corner_mm`). **Revue**
- Une longueur est écrite pour la personne par `Units::*_in(…, locale)`, jamais par un `format!("{:.1}")` hors de
  `shared/units.rs`, et un nombre que la personne saisit est lu par `parse_number`. Exemple : `shared/units.rs`
  (`length_in`, `parse_number`). **Revue**

## MVVM

Dans chaque tranche, le code suit MVVM : un modèle, un modèle de vue et une vue, selon les besoins de la tranche. La
répartition de ces couches en dossiers est décrite dans Architecture ([Couches dans une
tranche](architecture/index.md#couches-dans-une-tranche)). Une règle d’ici qui dit ce qu’une couche n’utilise pas
porte sur le code hors des tests.

### Modèle

- Un modèle (`model.rs`, `model/`) ne nomme ni Leptos, ni le navigateur (`web_sys`, `js_sys`, `wasm_bindgen`), et ne
  contient aucun signal. Exemple : `street/model/pick.rs` (`Pick`). **Revue**
- Les modèles de la rue et de la jonction ne nomment aucun port ni aucun magasin : ils ne lisent et ne conservent donc
  rien hors de la mémoire de la page. Parmi les autres modèles, `archive/model.rs`, `share/model.rs` et
  `sync/model.rs` prennent le port `Storage`, et `map/model/corridor.rs` prend un `CityStore`. Exemple :
  `street/model/mod.rs` (`Editor`). **Revue**
- Un modèle n’a pas de mots à lui pour la personne : il dit une chose par la clé d’un message, seule ou avec ses
  arguments (`Said`), et la chose est mise en mots hors du modèle, dans la langue de la page. Seule `Problem::say`
  dans `street/model/measures.rs` demande elle-même ses mots. Exemple : `street/model/calming.rs` (`calming_label`).
  **Garde** (`i18n_guard.rs`) contre un littéral qui ressemble à des mots dans les fichiers de sa liste `MIGRATED`,
  **Revue** pour le reste.
- Un modèle n’utilise le modèle de vue, la vue ni le `text.rs` d’aucune tranche. Exemple : `map/model/corridor.rs`
  (`plan`), qui utilise d’autres modèles. **Revue**

### Modèle de vue

- L’état d’un modèle de vue est dans des champs privés : une vue le lit par des méthodes, les accesseurs, et le change
  par des méthodes, les commandes. Un accesseur d’où une vue tire ce qu’elle dessine suit le plus souvent les signaux
  qu’il lit, pour que la vue soit redessinée quand ils changent. Seule `version`, de `StreetVm` et de `JunctionVm`,
  donne un signal. Exemple : `project/vm.rs` (`rows`, `create`). **Revue**
- Un accesseur donne souvent ce qui est prêt à présenter, dans la langue de la page. Dans la rue et la jonction, une
  grande partie vient sous la forme des `Said` de la vue du modèle (`View`, `JView`), que les vues Leptos mettent en
  mots. Exemple : `project/vm.rs` (`Row`, que donne `rows`). **Revue**
- Les fichiers des modèles de vue (`vm.rs`, `vm/`) ne nomment ni `web_sys`, ni `js_sys`, ni `wasm_bindgen` : le
  navigateur est derrière les ports (voir [Ports](architecture/index.md#ports)). Exemple : `project/vm.rs`
  (`ProjectsVm`). **Revue**
- Un modèle de vue n’utilise aucune vue, sauf `map/vm/report.rs`, qui dessine les figures du rapport avec des
  fonctions des vues de la rue et de la jonction. Exemple : `project/vm.rs` (`ProjectsVm`). **Revue**
- Un modèle de vue qui prend les ports est testé en natif contre leurs doublures, par `test_ports()`,
  `test_ports_with_time()` ou une autre fonction de `shared/ports.rs` (voir [Tests des modèles de
  vue](testing.md#tests-des-modèles-de-vue)). Exemple : `standards/tests.rs` (`setup`). **Revue**

### Vue

- Presque chaque composant prend son modèle de vue (`vm: Rc<…Vm>`), montre ce que donnent ses accesseurs et appelle
  ses commandes pour ce que fait la personne. Exemple : `map/view/network_view.rs` (`NetworkChecks`, dont `row`
  appelle `show_check`). **Revue**
- Une vue ne change l’aménagement que par une commande de son modèle de vue, et n’a pas à elle d’`Editor`, de
  `Junction` ni de `CityStore`. Dans la rue et la jonction, la commande est souvent `edit`, qui prend la modification
  du modèle sous forme de fermeture. Exemple : `street/view/inspector.rs` (`commit_width`). **Revue**
- Un signal propre à une vue ne contient que ce dont son composant a besoin pendant qu’il est affiché, comme ce qui y
  est saisi ou choisi, ce qui est ouvert ou pointé, ou une taille mesurée. Seules des vues de la rue et de la
  jonction, et `shared/scenarios_view.rs`, ont de tels signaux. Exemple : `street/view/green_tools.rs` (`Fields`).
  **Revue**
- Ce que fait une touche dans les dessins de la rue et de la jonction est le plus souvent décidé par une fonction pure
  et testée, dont la vue exécute la réponse ou la transmet à son modèle de vue. Le dessin du contour d’une zone sur la
  carte est une machine à états pure et testée, que fait avancer le modèle de vue. Ailleurs, comme pour la touche
  Entrée dans un champ ou les flèches dans la zone de recherche de la carte, la vue distingue souvent elle-même les
  touches. Exemples : `street/vm/keys.rs` (`on_section`), `junction/vm/keys.rs` (`on_drawing`), `restriction/draw.rs`
  (`Draw`). **Revue**

## Leptos et SVG

- Un composant est nommé en `PascalCase`. Exemple : `map/view/network_view.rs` (`NetworkChecks`). **Revue**
- Les fonctions qui renvoient du SVG sous forme de chaîne (voir
  [Couches dans une tranche](architecture/index.md#couches-dans-une-tranche)) n’utilisent ni Leptos, ni signal, ni
  DOM. Exemple : `street/view/plan/mod.rs` (`plan_svg`). **Revue**
- Un texte mis dans du balisage SVG passe par `esc`. Exemple : `street/view/plan/mod.rs` (`thing`). **Revue**
- Une coordonnée est presque toujours arrondie avant d’être écrite dans du balisage SVG, par exemple par `f1`.
  Exemple : `shared/iso/format.rs` (`f1`). **Revue**

## Commentaires

- Presque chaque commentaire de documentation commence par une majuscule et se termine par un point. Exemple :
  `street/view/plan/mod.rs` (`plan_svg`). **Revue**
- Il dit le plus souvent ce qu’est la chose, par un groupe nominal (comme « The… » ou « Whether… »), ou ce qu’elle
  fait, par un verbe à la troisième personne (comme « Keeps… » ou « Draws… »). Exemple : `shared/iso/format.rs`
  (`f1`). **Revue**
- Le Rust n’a que des commentaires de ligne, aucun commentaire de bloc. Exemple : `shared/iso/format.rs` (`esc`).
  **Revue**
- Aucun `TODO` ni `FIXME` ne reste dans le Rust. Exemple : `shared/iso/format.rs` (`f1`). **Revue**

## Tests

L’endroit où va un nouveau test est indiqué dans [Tests](testing.md#où-va-un-nouveau-test).

- Le nom de presque chaque test Rust est une phrase qui énonce le comportement; aucun n’a de préfixe `test_`.
  Exemple : `crates/cityloom_server/src/crypto.rs` (`two_seals_of_one_value_differ`). **Revue**
- Les tests unitaires gardés dans le fichier qu’ils testent sont presque toujours dans un `mod tests` à sa fin, avec
  `use super::*`. Exemple : `shared/iso/format.rs` (`tests`). **Revue**
- Une doublure d’un port est nommée d’après ce qu’elle fait à la place du port, comme `RecordingAnnouncer`,
  `MemoryStorage` ou `FixedClock`. Exemples : `shared/ports.rs` (`RecordingAnnouncer`),
  `crates/cityloom_server/src/clock.rs` (`FixedClock`). **Revue**

## Autres langages

### JavaScript

Ce qui relève de JavaScript est décrit dans [Architecture](architecture/index.md#javascript).

- Un fichier `.js` de `web/` s’ouvre sur un commentaire qui dit ce qu’il fait. Exemple : `web/mapcanvas.js` (le
  commentaire au-dessus de `createMapCanvas`). **Revue**
- Les fichiers `.js` de `web/` sont des modules ES, sauf `boot.js`, qui est un script classique. Exemple :
  `web/mapcanvas.js` (`createMapCanvas`). **Revue**
- Les fichiers `.js` de `web/` déclarent avec `const` et `let`, jamais `var`, et comparent avec `===`, jamais `==`.
  Exemple : `web/mapcanvas.js` (`here`, `themeNow`). **Revue**

### TypeScript

- Une spécification prend `test` et `expect` de `./fixtures`. Exemple : `e2e/tests/shell.spec.ts` (`expect`, `test`).
  **Revue**
- Le titre d’un test d’une spécification est une phrase qui énonce le comportement. Il commence par une minuscule, à
  moins que son premier mot ne soit un nom comme celui d’une touche, d’une commande ou d’une langue, et n’a pas de
  point final. Exemple : `e2e/tests/street.spec.ts`
  (`a piece is selected with the arrow keys, announced, and shown in the details`). **Revue**

### Python

- Un fichier Python de `scripts/` a une chaîne de documentation à son début et `from __future__ import annotations`,
  et n’importe que la bibliothèque standard et les modules du dépôt. Exemple : `scripts/ci/ci_changes.py`
  (`CI_CONFIGURATION`). **Revue**
- Un test est une méthode d’une `unittest.TestCase`, presque toujours nommée `test_` suivi d’une phrase. Exemple :
  `scripts/include/tests/test_factory_gate.py` (`test_the_red_gate_passes_a_red_commit_whose_new_test_fails`).
  **Revue**

### Shell

- Un script est en `sh` POSIX, avec à son début un commentaire qui dit ce qu’il fait, et `set -eu`. Exemple :
  `scripts/check-version.sh` (`version_of`). **Revue**

### CSS

- Une feuille de style s’ouvre sur un commentaire. Exemple : `web/app.css` (au-dessus de `:root`). **Revue**
- Une couleur est une propriété personnalisée, utilisée avec `var(--…)`, sauf quelques-unes écrites sur place, comme
  le `#000` d’un masque. Exemple : `web/app.css` (`--accent`). **Revue**
- Une nouvelle paire de couleurs pour du texte ou une commande est mesurée dans `shared/contrast.rs`. Exemple :
  `shared/contrast.rs` (`contrast`). **Garde** (`shared/contrast.rs`) pour les paires qu’elle énumère, **Revue** pour
  l’ajout d’une nouvelle.
- Une classe est nommée en kebab-case; une classe faite d’un identifiant du catalogue en garde la graphie
  (`mat-rain_garden`). Exemple : `web/editor.css` (`insp-meter`). **Revue**

### Migrations SQL

- Une migration est nommée `000N_name.sql` et s’ouvre sur un commentaire. Son SQL est en minuscules et ses noms sont
  en snake_case. Exemple : `crates/cityloom_server/migrations/0005_shares.sql` (`shares_user`). **Revue**
- Une migration fusionnée n’est pas modifiée : une nouvelle est ajoutée. Exemple :
  `crates/cityloom_server/migrations/0003_work_project_kind.sql` supprime et remplace `work_kind_check`, que
  `0002_work.sql` a créé. Aucune vérification de la demande de fusion ne cherche une modification; au démarrage du
  serveur, son outil de migration (`migrate` dans `crates/cityloom_server/src/db.rs`) refuse une migration appliquée
  dont la somme de contrôle diffère. **Revue**

### Messages Fluent

- Un fichier `.ftl` s’ouvre sur un commentaire qui dit ce qu’il met en mots. Exemple : `project/i18n/en.ftl` (le
  commentaire à son début). **Revue**
- L’identifiant d’un message est en kebab-case et une variable est en snake_case; un identifiant fait d’un identifiant
  du catalogue en garde la graphie (`material-rain_garden`). Exemple : `junction/i18n/en.ftl` (`jn-calls-said`,
  `$not_served`). **Revue**
- Une phrase est un seul message entier, avec des variables nommées pour ce qui varie, et n’est pas assemblée de
  morceaux dans le code. Une variable peut être un mot qui est lui-même un message (`$kind` de `clock-except` dans
  `street/i18n/en.ftl`), et des messages entiers peuvent être joints, comme phrases ou comme liste. Un pluriel est un
  sélecteur (`[one]` / `*[other]`). Exemple : `project/i18n/en.ftl` (`project-delete-confirm-links`). **Revue**
- Un identifiant en kebab-case écrit dans un fichier de la liste `MIGRATED` de la garde existe dans les deux langues.
  Exemple : `i18n_guard.rs` (`every_message_id_used_in_a_migrated_file_exists_in_both_languages`). **Garde**
  (`i18n_guard.rs`)
- Les deux langues ont les mêmes identifiants, avec les mêmes variables. Exemple : `shared/i18n.rs`
  (`parity_problems`). **Garde** (le test de parité de la tranche), **Revue** pour `timeline` et `standards`, qui
  n’ont pas de test de parité (voir [Traduction](testing.md#traduction)).

## Commits et demandes de fusion

- Le titre d’une demande de fusion est une ligne qui dit ce que fait le changement. Il commence par une majuscule et
  n’a ni point final ni préfixe de type comme `feat:`. Exemple : `.claude/skills/factory-evidence/SKILL.md`
  (`<what it does> (<version>) (CLM-<n>)`). **Revue**
- Après ce que fait le changement, le titre donne la version, quand la demande de fusion l’augmente, puis la clé du
  billet. GitHub ajoute le numéro. Exemple :
  `Keep a junction's folded options reachable by heading (0.113.1) (CLM-50) (#289)`. **Revue**
- Une demande de fusion fait ce que demande son billet et rien d’autre : aucun remaniement, renommage ni correctif
  ailleurs. Exemple : `.claude/agents/reviewer.md` (Scope). **Revue**
- Chaque changement est un commit distinct, fait une fois que ses tests réussissent. Exemple : `CLAUDE.md`
  (Conventions and gotchas). **Revue**
- Une demande de fusion augmente la version dans `crates/cityloom_editor/Cargo.toml` (majeure pour un changement
  incompatible, mineure pour une fonctionnalité, corrective pour un correctif) et met à jour `Cargo.lock`, à moins de
  ne changer que la documentation et la configuration de l’intégration continue (voir [Ce qui doit réussir avant une
  fusion](testing.md#ce-qui-doit-réussir-avant-une-fusion)). Exemple : `scripts/check-version.sh` (`version_of`).
  **Outil** (la tâche Version bump du flux de prévérification) pour l’augmenter, **Revue** pour le choix de majeure,
  mineure ou corrective et pour `Cargo.lock`.

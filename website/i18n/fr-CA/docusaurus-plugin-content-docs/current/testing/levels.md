---
title: Paliers de tests
sidebar_label: Paliers
---

# Paliers de tests

Cette page décrit chaque palier de tests, et les gardes. La recette et la tâche d’intégration continue qui exécutent
chaque palier, l’endroit où va un nouveau test et la racine des chemins de l’éditeur sont dans la
[vue d’ensemble](index.md).

## Tests unitaires du modèle

Chaque fichier du modèle garde ses tests à côté du code, dans un `#[cfg(test)] mod tests` : par exemple les fichiers
de `street/model/`, et `crates/cityloom_model/src/cut.rs`, `graph.rs`, `osm.rs`, `importer.rs` et `corridor.rs`. Un
modèle ne contient ni Leptos ni formulation. Une modification renvoie un `bool`, et un refus porte une raison : un test
appelle donc la règle et lit ce qu’elle renvoie. `cargo test shell::vm` exécute un seul module; le nom complet d’un
test fonctionne aussi.

## Tests des modèles de vue

Un modèle de vue n’atteint le navigateur que par les ports : ses tests s’exécutent donc en natif contre leurs
doublures, dans `shared/ports.rs`.

- `test_ports()` donne un ensemble de ports avec un `RecordingAnnouncer` (ce qui a été dit à un lecteur d’écran) et un
  `MemoryStorage`, que le test peut lire.
- `test_ports_with_time()` y ajoute un `ManualScheduler` : le travail différé ne s’exécute que lorsque le test appelle
  `advance(ms)`.
- D’autres fonctions du même fichier ajoutent le reste, comme `test_ports_with_fetcher()`, dont le `FakeFetcher` garde
  chaque requête jusqu’à ce que le test y réponde.

Les tests des modèles de vue sont les `mod tests` de `street/vm/mod.rs`, de `junction/vm/mod.rs` et de
`map/vm/mod.rs`, entre autres. La synchronisation est testée de la même façon, contre un serveur en mémoire :
`ServerSim`, `Device` et `device()` dans `sync/tests.rs`.

## Tests de composant

Un test de composant rend une vue en HTML sur l’hôte, avec `shared::testing::html` (`shared/testing.rs`), et lit le
balisage. Ils se trouvent dans le `tests.rs` de chaque tranche : `account`, `archive`, `junction/view`, `map/view`,
`project`, `share`, `shell`, `standards`, `street/view`, `sync` et `timeline`.

Les tests de composant des commandes partagées sont dans le code, dans `crates/cityloom_ui/src/` : les `mod tests` de
`radio.rs`, de `stepper.rs`, de `section.rs` et de `confirm.rs`, avec la fonction `testing::html` propre à cette caisse
(`testing.rs`). `cargo test -p cityloom_ui` les exécute.

## La vérification de la construction wasm32

`just test-wasm` exécute
`cargo build --target wasm32-unknown-unknown --workspace --exclude cityloom_server --exclude citymodel_builder`. Elle
n’exécute aucun test : elle vérifie les types du code sous `cfg(target_arch = "wasm32")`, que `cargo test` sur l’hôte
ne compile pas.

## Tests du serveur

Les tests du serveur sont dans `crates/cityloom_server/tests/` : `api`, `areas`, `auth_flow`, `data_mount`, `db`,
`no_plaintext`, `providers`, `rotate`, `share_api`, `share_db`, `static_files`, `unavailable`, `work_api` et
`work_db`, avec leurs outils dans `tests/support/mod.rs`.

- Un test qui a besoin de la base de données est un `#[sqlx::test]`. Il reçoit une base de données neuve sur le
  Postgres que nomme `DATABASE_URL`. `areas`, `data_mount`, `providers`, `static_files` et `unavailable` n’en ont
  aucun.
- `TestApp` (dans `tests/support/mod.rs`) est le routeur du serveur sur un `PgRepo`, avec des clés de test fixes et
  une horloge fixe. Son `Browser` garde des témoins devant le routeur, et `sign_in` le connecte par le faux
  fournisseur.
- `api`, `auth_flow`, `providers`, `rotate`, `share_api` et `work_api` exigent la fonctionnalité `test-provider` du
  serveur. `just test-host` l’active chaque fois que le serveur est testé.

`just test-host` refuse de s’exécuter sans `DATABASE_URL`, quelles que soient les caisses qu’il teste.

## Tests de `citymodel_builder` et de `cityloom_model`

`crates/citymodel_builder/tests/` contient les tests de la chaîne (`archive`, `city`, `commands`, `ctrl_c`, `download`,
`prepare`, `progress` et `tiles`) et ceux de l’importateur, qui forment une seule caisse de tests, `tests/importer/` :
`import.rs`, `real_city.rs`, `clip_agrees.rs` et `projection.rs`, avec de petits extraits OpenStreetMap comme
`boulevard.osm` et `split_block.osm`. Les tests de `cityloom_model` sont dans le code (voir
[Tests unitaires du modèle](#tests-unitaires-du-modèle)). `cityloom_model::IMPORTER_VERSION` augmente chaque fois que
ce que produit l’importateur change pour les mêmes données.

## Tests du rapport et du PDF

- `cityloom_report` a ses tests dans le code, dans `crates/cityloom_report/src/lib.rs`.
- `crates/cityloom_report_pdf/tests/pdf.rs` écrit des PDF avec `write_pdf` et les relit avec lopdf.
- Les tests de l’éditeur écrivent le rapport par l’écrivain de PDF : `cityloom_report_pdf` est l’une des dépendances
  de développement de l’éditeur, et reste donc hors de la construction de l’éditeur lui-même. `scripts/build.sh` échoue
  si le JavaScript de l’éditeur exporte `report_pdf`.
- Dans le navigateur, `e2e/tests/report.spec.ts` lit le PDF que fait une page avec `pdfjs-dist`.

## Tests des scripts

`just test-scripts` exécute `python3 -m unittest discover -s scripts/include/tests`, avec la bibliothèque standard
seulement. Les tests sont `test_ci_changes.py` (le sélecteur de changements de l’intégration continue, et le fait que
chaque spécification est dans un seul niveau et un seul groupe), `test_factory_config.py`, `test_factory_gate.py` et
`test_factory_guard.py` (l’usine logicielle), `test_issue_system.py` (les étiquettes, les formulaires de billets et
les règles d’étiquetage concordent), `test_architecture_docs.py` (les pages d’Architecture nomment chaque tranche, clé
de stockage et table du serveur, dans les deux langues) et `test_features.py` (la liste des fonctionnalités est
complète, et la part des tests Rust qu’aucune fonctionnalité ne nomme reste dans sa limite).

## Tests dans le navigateur

Les tests dans le navigateur sont des spécifications Playwright, `e2e/tests/*.spec.ts`, exécutées contre le
`web/pkg` construit et un serveur construit avec le faux fournisseur. `e2e/tiers.json` donne à chaque spécification un
seul niveau :

- **Interface** (35 spécifications) : une page, le réseau simulé, le serveur statique seulement; ni base de données ni
  archive. `just e2e-ui`.
- **Intégration** : `account-server`, `share` et `sync`, contre le vrai serveur et un Postgres (`DATABASE_URL`).
  `just e2e-integration`.
- **Bout en bout** : `archive`, `delivery` et `serve`, sur un serveur qui extrait l’archive empaquetée, et la
  livraison du site par le vrai serveur. `just e2e-archive`.
- **`archive_also`** : `map` et `street` sont des spécifications de l’interface que le niveau de bout en bout exécute
  de nouveau sur l’archive extraite.

`e2e/playwright.config.ts` crée un projet Playwright par niveau, et lève une erreur quand une spécification n’est dans
aucun niveau ou dans deux. Avec `E2E_ARCHIVE=1`, seul le projet de bout en bout s’exécute, avec les spécifications
d’`archive_also` en plus. Chaque projet est Desktop Chrome en 1440 sur 900. Dans l’intégration continue, un test en
échec est repris une fois, `test.only` fait échouer l’exécution, et deux processus travaillent. Le serveur est
`target/e2e/cityloom_server`, la copie que fait `scripts/pin-server.sh`, démarré avec `FAKE_PROVIDER=1` et des clés de
test fixes. Hors de l’intégration continue et du mode archive, Playwright réutilise un serveur déjà sur le port.
`E2E_PORT` change le port.

`just e2e` construit le wasm et le serveur et fait les tuiles des données d’essai, puis exécute les trois niveaux.
Avec des arguments, il exécute ces spécifications, quel que soit leur niveau : `just e2e tests/street.spec.ts`.

## Les gardes

Les gardes sont des tests sur le code lui-même. Ils s’exécutent avec les tests sur l’hôte.

- **`i18n_guard.rs`** échoue sur une formulation dans les fichiers de sa liste `MIGRATED` (`LEFT_OUT` nomme les
  quelques fichiers laissés de côté, chacun avec sa raison), sur un identifiant de message utilisé qui n’est pas dans
  les deux langues, sur un fichier d’une tranche traduite qui n’est dans aucune des deux listes, et sur une entrée
  morte d’`ALLOWED_LITERALS` ou de `NOT_MESSAGE_IDS`.
- **`dependency_guard.rs`** échoue sur une utilisation d’une autre tranche que son tableau `ALLOWED` ne permet pas, sur
  un manquement à une règle des couches ou des bibliothèques que son tableau `EXCEPTIONS` ne laisse pas passer, sur une
  entrée que rien n’utilise ou qui n’a pas de raison, et sur un dossier de `src/` qui n’est pas dans `SLICES`. Le code
  des tests en est exempté.
- **`layout_guard.rs`** garde les fichiers de `street/`, `junction/` et `map/` dans `model/`, `vm/` et `view/` : leur
  racine ne contient que `mod.rs`, `CLAUDE.md` et `i18n/`.
- **`ui_guard.rs`** échoue quand une tranche dessine elle-même un groupe de boutons radio, des boutons plus et moins, une
  section des détails avec son titre ou une question, au lieu d’utiliser ceux de `cityloom_ui`.
- **`atlas_contract.rs`** tient le tableau Atlas (`shared::atlas::MEASURES`) pour un contrat : chaque mesure qu’il
  énumère est construite, rien d’autre ne l’est, et chaque mesure est nommée dans les deux langues.
- **`shared/contrast.rs`** mesure les jetons de couleur du CSS selon les critères 1.4.3 (texte) et 1.4.11 (le reste)
  des WCAG.
- **`no_plaintext.rs`** (dans les tests du serveur) lit chaque colonne de `users`, `identities`, `sessions`, `work` et
  `shares` en octets bruts et n’y trouve rien de ce avec quoi la personne s’est connectée.

Les six premiers sont des modules `#[cfg(test)]` de l’éditeur.

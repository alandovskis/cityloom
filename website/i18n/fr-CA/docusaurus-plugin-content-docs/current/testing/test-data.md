---
title: Données d’essai
---

# Données d’essai

Cette page décrit les données d’essai. Les paliers qui s’en servent sont dans [Paliers de tests](levels.md).

## Données d’essai OpenStreetMap et tuiles

Les données routières des tests dans le navigateur sont `tests/e2e/fixtures/plateau.osm.pbf`, `turbo.osm.pbf` et
`open-ends.osm.pbf`. `open-ends.osm` est la source OSM XML de `open-ends.osm.pbf`, avec son élément `<bounds>`; les
tests ne le lisent pas. `just e2e-fixtures` importe chaque `.osm.pbf` en une tuile du réseau,
`tests/e2e/fixtures/generated/<nom>.json` (`citymodel_builder fixture`; ignorée par Git). C’est une recette privée, que
`just` n’énumère donc pas; chaque recette de navigateur l’exécute d’abord. Dans l’intégration continue, la tâche
`build` l’exécute une fois et transmet les tuiles dans l’artefact `browser-build`, avec le wasm, `web/vendor` et le
serveur figé.

## `fixtures.ts`

`tests/e2e/tests/fixtures.ts` est ce qu’importe chaque spécification.

- `PLATEAU_TILE`, `TURBO_TILE` et `OPEN_ENDS_TILE` sont les tuiles produites, et `tileFrom(file)` répond par l’une
  d’elles à une demande de tuile.
- Chaque test échoue sur une erreur de la page ou une erreur de la console. Les erreurs de `/api/me`,
  `/api/providers` et `/api/work` sont admises, et celles de `/api/me/*` quand le test porte l’annotation
  `failing-account-api`.
- Chaque test reçoit un contexte de navigateur neuf : le stockage commence vide.
- Une donnée d’essai automatique sert la tuile du Plateau comme tuile métropolitaine par défaut, sauf avec
  `E2E_ARCHIVE`, où le serveur répond à partir de l’archive.
- Le stockage commence avec `cityloom-lang=en` et le premier projet.

## Le faux fournisseur de connexion

La fonctionnalité `test-provider` du serveur (`crates/cityloom_server/Cargo.toml`) ajoute `src/fake_provider.rs`, un
fournisseur OpenID servi par le serveur lui-même. Les tests choisissent qui se connecte avec le témoin `fake-login`.
Les tests dans le navigateur démarrent le serveur avec `FAKE_PROVIDER=1`, et `account-server.spec.ts` utilise un `sub`
unique par test.

## Le Postgres jetable

`just db` démarre un conteneur nommé `cityloom-db` (`postgres:18`) sur 127.0.0.1:55432 et affiche la ligne
`export DATABASE_URL=…` à exécuter; `just db-stop` le supprime. Chaque `#[sqlx::test]` y crée sa propre base de
données. Dans l’intégration continue, les tâches des tests unitaires et d’intégration exécutent un service
`postgres:18`.

## L’archive e2e

`scripts/e2e-archive.sh` copie `tests/e2e/fixtures/generated/plateau.json` comme seule tuile métropolitaine, `0_0`,
l’empaquette avec `citymodel_builder pack` dans `target/e2e-archive/areas/plateau.citymodel`, et exécute
`E2E_ARCHIVE=1 npx playwright test --project=e2e`. Le serveur extrait l’archive par `CITYLOOM_AREAS` et `DATA_ROOT`,
tous deux dans `target/e2e-archive`.

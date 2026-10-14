---
id: getting-started
title: Démarrage
sidebar_position: 2
---

# Démarrage

Cette page s’adresse aux développeuses et aux développeurs. Le dépôt utilise [`just`](https://just.systems/) pour lancer
ses tâches; lancez `just` sans argument pour les lister.

## Ce qu’il faut

- Une chaîne d’outils Rust avec la cible `wasm32-unknown-unknown`.
- Node.js et npm.
- Python 3 (`just prepare` exécute `scripts/prepare.py`).
- `just`.

`just setup` installe le reste, une seule fois :

```sh
just setup
```

La recette exécute `npm ci` à la racine (les bibliothèques d’exécution de la carte, MapLibre et PMTiles) et dans `e2e`
(Playwright), installe `wasm-bindgen-cli` dans `.tools` à la version fixée dans `Cargo.toml` et installe Chromium pour
Playwright.

## Construire et servir

```sh
just build   # wasm de production des deux modules, puis wasm-bindgen dans web/pkg
just serve   # http://127.0.0.1:8137/
```

`just build` exécute `scripts/build.sh`. Elle construit l’éditeur et le lecteur OpenStreetMap comme deux modules
WebAssembly, les transforme en code d’accès JavaScript avec `wasm-bindgen` dans `web/pkg` (non versionné) et copie les
bibliothèques de la carte dans `web/vendor`.

`just serve` exécute `node e2e/serve.mjs`. Le serveur répond aux requêtes de plage, dont les PMTiles du fond de carte ont
besoin. Définissez `E2E_PORT` pour utiliser un autre port.

## Données cartographiques

```sh
just prepare
```

La recette télécharge dans `data/` l’extrait du Québec de Geofabrik s’il n’y est pas déjà (environ 1,2 Go;
`just prepare --refresh` en prend un plus récent). Elle découpe ensuite les routes de la région métropolitaine en
tuiles dans `web/data/metro` (nécessite `osmium-tool`) et construit le fond de carte de la page Carte,
`web/data/basemap/montreal.pmtiles`, avec Planetiler (nécessite Java 21 ou plus récent). Git ignore ces deux sorties.

Rien n’exige les tuiles de routes : sans elles, tous les lieux, y compris celui par défaut, viennent d’Overpass. Sans le
fond de carte, la page Carte l’indique et n’affiche aucune carte.

## Tests et mise en forme

```sh
just test          # cargo test --workspace, et une construction wasm32 du code que `cargo test` seul ne compile pas
just e2e           # construit, puis exécute les tests Playwright de e2e/
just check         # les deux
just format        # rustfmt et Prettier
just format-check  # la même vérification, sans rien modifier
```

L’intégration continue exécute `just format-check`, `just test` et `just e2e` avec `RUSTFLAGS=-D warnings`; un nouvel
avertissement du compilateur la fait donc échouer. Elle construit aussi cette documentation avec `just docs-build`. Chaque demande de fusion augmente la version dans le `Cargo.toml` de
la racine.

## Cette documentation

```sh
just setup-docs  # installer les paquets de cette documentation (une fois)
just docs        # servir le site avec rechargement automatique
just docs-build  # construire les deux langues dans website/build
```

Les pages anglaises sont dans `website/docs/` et les pages françaises dans
`website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`. Chaque page existe dans les deux langues.

---
id: getting-started
title: Démarrage
sidebar_position: 3
---

# Démarrage

Cette page s’adresse aux développeuses et aux développeurs. Le dépôt utilise [`just`](https://just.systems/) pour lancer
ses tâches; lancez `just` sans argument pour les lister.

## Ce qu’il faut

- Une chaîne d’outils Rust avec la cible `wasm32-unknown-unknown`.
- Node.js et npm.
- `just`.

`just setup` installe le reste, une seule fois :

```sh
just setup
```

La recette exécute `npm ci` à la racine (la bibliothèque d’exécution de la carte, MapLibre) et dans `e2e`
(Playwright), installe `wasm-bindgen-cli` dans `.tools` à la version fixée dans `crates/cityloom_editor/Cargo.toml` et installe Chromium pour
Playwright.

## Construire et servir

```sh
just build   # wasm de production des deux modules, puis wasm-bindgen dans web/pkg
just serve   # http://127.0.0.1:8137/ (cargo run -p cityloom_server)
```

`just build` exécute `scripts/build.sh`. Elle construit l’éditeur et le lecteur OpenStreetMap comme deux modules
WebAssembly, les transforme en code d’accès JavaScript avec `wasm-bindgen` dans `web/pkg` (non versionné) et copie les
bibliothèques de la carte dans `web/vendor`.

`just serve` exécute `cargo run -p cityloom_server`, le serveur Rust. Il sert `web/` et répond aux requêtes de plage. Définissez `E2E_PORT` pour utiliser un autre port. Sans `DATABASE_URL`, les
routes de compte répondent 503 et les pages n’ont pas de session ouverte.

## Données cartographiques

```sh
just prepare montreal
```

La recette exécute `citymodel_builder`, l’outil Rust du dépôt (`cargo run -p citymodel_builder -- prepare`; ses arguments
sont ceux de la recette), pour la ville de `cities/montreal.toml`. Elle télécharge dans `data/` l’extrait OpenStreetMap
de la ville, l’extrait du Québec de Geofabrik, s’il n’y est pas déjà (environ 1,2 Go; `just prepare montreal --refresh`
en prend un plus récent, et `just prepare montreal --extract <path>` lit un autre fichier à sa place). Elle conserve
ensuite les routes de la région métropolitaine (nécessite `osmium-tool`), les importe en une fois avec osm2streets et
découpe le réseau de rues obtenu selon la grille de la ville en tuiles JSON dans `web/data/metro` (environ 1 700 tuiles,
environ 570 Mo). Enfin, elle les réunit dans une seule archive, `data/montreal.citymodel`, un fichier tar muni d’un index
SHA-256 que le serveur extrait au démarrage. Git ignore les tuiles et l’archive. `just pack montreal` empaquette de
nouveau les tuiles sans le reste.

`just prepare` sans ville, ou avec une ville qui n’a pas de fichier dans `cities/`, énumère les villes connues et
s’arrête avant de télécharger quoi que ce soit.

### Ajouter une ville

Une ville est un seul fichier de configuration, `cities/<id>.toml`, versionné; en ajouter une ne change aucun code Rust.
`just prepare <id>` le lit, et `cities/montreal.toml` en est un exemple. Ses clés :

- `id` : le nom du fichier, qui est aussi le nom de zone de l’archive : de 1 à 64 lettres minuscules, chiffres et
  traits d’union.
- `extract_url` : l’adresse `http://` ou `https://` de l’extrait OpenStreetMap de la ville. Sa dernière partie, un
  simple nom de fichier qui se termine par `.osm.pbf`, est le nom de l’extrait dans `data/`. Deux villes partagent un
  extrait seulement si elles donnent la même adresse.
- `[grid]` : les tuiles, en degrés. `west` et `south` sont le coin sud-ouest de la tuile `0_0`; `tile_width` et
  `tile_height`, la taille du cœur d’une tuile; `columns` et `rows`, le nombre de tuiles, d’ouest en est et du sud au
  nord. La grille doit se trouver sur la Terre et contenir au moins une tuile.
- `[margin]` : la marge, soit la distance jusqu’où une tuile s’étend au-delà de son cœur, en degrés, `lon` et `lat`.
  Chacune doit valoir au moins les 400 m qu’un lieu couvre depuis son centre, mesurés au bord de la grille le plus près
  d’un pôle.

Chaque ville garde son propre extrait, son propre dossier de travail, `data/metro-work/<id>`, et sa propre archive de
zone, `data/<id>.citymodel`; `web/data/metro` contient les tuiles de la dernière ville préparée. Un fichier auquel il
manque une clé, qui en a une inconnue ou qui échoue à une vérification est refusé avant tout téléchargement, avec le
fichier et la clé nommés. Le test `every_city_file_loads` de l’outil (`cargo test -p citymodel_builder --test city`)
charge chaque fichier de `cities/`, de sorte qu’une nouvelle ville est vérifiée avec les autres tests.

L’ouverture d’un lieu exige les tuiles de routes : sans elles, aucun lieu ne s’ouvre, y compris celui par défaut, et la page qui l’ouvre indique que le lieu n’est pas couvert.

## Tests et mise en forme

```sh
just db            # un Postgres jetable dans Docker pour les tests du serveur et la spécification du compte
just test          # cargo test --workspace, et une construction wasm32 du code que `cargo test` seul ne compile pas; exige `DATABASE_URL`
just e2e           # construit, puis exécute les trois niveaux de tests Playwright de tests/e2e/ (`just e2e-ui`, `just e2e-integration` et `just e2e-archive` en exécutent un chacun); le niveau d’intégration exige `DATABASE_URL`
just check         # les deux
just format        # rustfmt et Prettier
just format-check  # la même vérification, sans rien modifier
```

L’intégration continue se compose de deux flux de travail. `.github/workflows/presubmit.yml` vérifie chaque demande de fusion et `.github/workflows/postsubmit.yml` vérifie `main` au complet une fois la demande fusionnée, puis la déploie. Les deux exécutent `just format-check`, `just test` et les tests de navigateur avec `RUSTFLAGS=-D warnings`; un nouvel
avertissement du compilateur les fait donc échouer. Le flux de prévérification omet le niveau de bout en bout (l’archive empaquetée), que seul le flux de postvérification exécute. Un flux de travail distinct, `.github/workflows/docs.yml`, vérifie et construit cette documentation (ce qu’exécute `just docs-build`) quand un
changement touche `website/`, et la publie sur GitHub Pages depuis main. Ni l’un ni l’autre ne démarre pour un changement qui ne touche que la
documentation. Leurs tâches `test` et d’intégration
exécutent un service Postgres; les tâches de tests de l’interface et de bout en bout n’en ont pas besoin. Chaque demande de fusion augmente la version dans `crates/cityloom_editor/Cargo.toml`, sauf celle qui ne
touche que la documentation et la configuration de l’intégration continue (`CI_CONFIGURATION` dans `scripts/ci/ci_changes.py` : `.github/`,
`scripts/ci/`, `scripts/include/` et `.claude/`).

[Tests](testing/index.md) indique à quel palier appartient un test et où va un nouveau test;
[Ce qui doit réussir avant une fusion](testing/when-tests-run.md#ce-qui-doit-réussir-avant-une-fusion) indique ce qui
doit réussir avant une fusion.

`just db` démarre le Postgres dans un conteneur nommé `cityloom-db`, sur 127.0.0.1:55432, et affiche la ligne à
exécuter : `export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/cityloom`. Les tests du serveur et la
spécification du compte (`tests/e2e/tests/account-server.spec.ts`) l’exigent et le disent quand il manque. `just db-stop`
supprime le conteneur.

## Comptes et déploiement

Le serveur lit ses réglages dans l’environnement; `.env.example` énumère chaque variable et la façon de fabriquer une
clé. Le volet des comptes exige `BASE_URL`, `DATABASE_URL`, `COOKIE_KEY`, `DATA_KEYS`, `DATA_KEY_CURRENT` et
`LOOKUP_KEY`; un fournisseur n’est offert que si toutes ses variables sont définies. Sans les variables des comptes, le
site est servi et `/auth/*` et `/api/*` répondent 503. `/api/work` a sa propre limite de débit, `WORK_RATE_PERIOD_SECS` (1 par défaut) et `WORK_RATE_BURST` (60 par
défaut), distincte des `AUTH_RATE_*` des routes de connexion.

Enregistrez l’URL de redirection `https://cityloom.app/auth/{osm|google|microsoft|apple}/callback` auprès de chaque
fournisseur.

Sauvegardez `DATA_KEYS` et `LOOKUP_KEY` dans un gestionnaire de mots de passe : les perdre fait perdre toutes les
données. Pour renouveler une clé maîtresse, dans cet ordre :

1. Ajoutez la nouvelle clé à `DATA_KEYS` (gardez l’ancienne) et donnez son identifiant à `DATA_KEY_CURRENT`. Déployez,
   puis attendez que le serveur tourne avec ces valeurs.
2. Exécutez `cityloom_server rewrap`. La commande affiche `resealed N data keys`. Exécutez-la de nouveau jusqu’à ce
   qu’elle affiche `resealed 0 data keys`.
3. Seulement alors, retirez l’ancienne clé de `DATA_KEYS` et déployez.

L’ordre compte : un serveur qui tourne encore avec l’ancienne clé courante continue d’envelopper les nouveaux comptes
avec l’ancienne clé, et `rewrap` ne voit pas les comptes créés après son démarrage. Si l’on retire alors l’ancienne
clé, ces comptes ne peuvent plus être ouverts et leurs données sont perdues. Sur Railway, ouvrez un terminal dans le
service en cours d’exécution avec `railway ssh`, puis exécutez `/app/cityloom_server rewrap` (le `WORKDIR` de l’image
est `/app`; les variables du service sont déjà dans l’environnement). Pour renouveler la clé de recherche,
donnez l’ancienne à `OLD_LOOKUP_KEY` et la nouvelle à `LOOKUP_KEY`, arrêtez le serveur et exécutez
`cityloom_server rehash` : une connexion pendant l’exécution pourrait créer un compte en double. La commande affiche le
nombre d’identités vérifiées, le nombre de celles qui ont été recalculées et le nombre de celles qui étaient déjà sur la
nouvelle clé.

`just deploy` construit le wasm et le serveur pour Linux (`x86_64-unknown-linux-musl`), refuse un serveur qui contient le
faux fournisseur et téléverse `web/` et le binaire vers Railway. Elle exige `just setup-deploy` (cargo-zigbuild et la
cible musl) et zig (`brew install zig` ou `python3 -m pip install ziglang`). `just check-no-fake` fait la même
vérification sur une version de production, comme l’intégration continue.

La suppression d’un compte en retire aussitôt les lignes. Les copies dans les sauvegardes expirent avec la période de
conservation de Railway.

### Vérifications manuelles, une fois par fournisseur

Faites-les après avoir enregistré un fournisseur (ses variables sont dans `.env.example`), avec un navigateur sans
session.

1. Connectez-vous depuis la page Carte. Vous arrivez sur la page Compte, le choix du lieu de conservation est présenté
   sans rien de sélectionné, et l’en-tête affiche votre nom.
2. Choisissez où votre travail est conservé, déconnectez-vous, puis reconnectez-vous : même compte, choix conservé.
3. Associez un deuxième fournisseur, déconnectez-vous, puis connectez-vous avec le deuxième : même compte. Dissociez le
   premier; la dernière connexion n’a pas de bouton Dissocier.
4. Annulez sur la page de consentement du fournisseur : la page dit « Connexion annulée. » et rien n’a été créé.
5. Consultez les journaux du serveur : ils contiennent des chemins, des statuts, des fournisseurs et des résultats, et
   aucun courriel, jeton ni témoin.
6. Supprimez le compte depuis sa page : la confirmation est dans la page, pas dans une boîte de dialogue; ensuite, se
   connecter avec le même fournisseur redemande le choix du lieu de conservation.
7. Sur Railway, confirmez que l’entrée la plus à droite de `X-Forwarded-For` est l’adresse du client : la limite de
   débit compte cette entrée. Depuis un interpréteur de commandes sur votre machine, exécutez
   `for i in $(seq 1 25); do curl -s -o /dev/null -w '%{http_code} ' -H "X-Forwarded-For: 10.0.0.$i" https://cityloom.app/auth/google/start; done`
   (une autre adresse inventée à gauche chaque fois) : les derniers codes doivent être 429 (rafale de 20), parce que le
   mandataire ajoute votre vraie adresse et que c’est elle, la clé. Une requête venant d’un autre réseau (un téléphone
   en données mobiles) au même moment doit toujours recevoir 303. Si le 429 n’arrive jamais, ou si l’autre réseau est
   limité aussi, le mandataire de Railway n’ajoute pas l’adresse comme le serveur le suppose; arrêtez-vous et examinez
   `client_address` dans `cityloom_server/src/app.rs`.
8. Choisissez « Stocker mon travail sur le serveur » sur la page Compte, modifiez une rue, puis ouvrez le site sur un
   deuxième appareil connecté au même compte : la rue montre la modification quand l’onglet est affiché de nouveau.

Apple : connectez-vous, déconnectez-vous, puis reconnectez-vous sans retirer l’app de votre identifiant Apple : le nom
reste (Apple ne l’envoie que la première fois). Masquer mon adresse courriel donne une adresse relais comme libellé.
L’échange du jeton doit réussir avec le secret client dans le corps (`client_secret_post`), et Apple doit accepter
`openid` dans la portée.
Microsoft : connectez-vous une fois avec un compte personnel et une fois avec un compte professionnel ou scolaire : deux
identités distinctes.
Google : tant que l’écran de consentement n’est pas vérifié, Google affiche l’avertissement « application non
vérifiée »; c’est normal.
Renouvellement des clés : `rewrap` seulement quand le serveur tourne avec la nouvelle clé courante, répété jusqu’à
`resealed 0 data keys`, l’ancienne clé étant retirée en dernier; `rehash` exige que le serveur soit arrêté (voir plus
haut). Vérifiez la ligne affichée.

## Cette documentation

```sh
just setup-docs  # installer les paquets de cette documentation (une fois)
just docs        # servir le site avec rechargement automatique
just docs-build  # construire les deux langues dans website/build
```

Les pages anglaises sont dans `website/docs/` et les pages françaises dans
`website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`. Chaque page existe dans les deux langues.

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
just serve   # http://127.0.0.1:8137/ (cargo run -p cityloom_server)
```

`just build` exécute `scripts/build.sh`. Elle construit l’éditeur et le lecteur OpenStreetMap comme deux modules
WebAssembly, les transforme en code d’accès JavaScript avec `wasm-bindgen` dans `web/pkg` (non versionné) et copie les
bibliothèques de la carte dans `web/vendor`.

`just serve` exécute `cargo run -p cityloom_server`, le serveur Rust. Il sert `web/` et répond aux requêtes de plage,
dont les PMTiles du fond de carte ont besoin. Définissez `E2E_PORT` pour utiliser un autre port. Sans `DATABASE_URL`, les
routes de compte répondent 503 et les pages n’ont pas de session ouverte.

## Données cartographiques

```sh
just prepare
```

La recette exécute `cityloom_prepare`, l’outil Rust du dépôt (`cargo run -p cityloom_prepare -- prepare`; ses arguments
sont ceux de la recette). Il télécharge dans `data/` l’extrait du Québec de Geofabrik s’il n’y est pas déjà (environ 1,2 Go;
`just prepare --refresh` en prend un plus récent). Elle découpe ensuite les routes de la région métropolitaine en
tuiles dans `web/data/metro` (nécessite `osmium-tool`) et construit le fond de carte de la page Carte,
`web/data/basemap/montreal.pmtiles`, avec Planetiler (nécessite Java 21 ou plus récent). Enfin, il réunit les deux dans une
seule archive, `data/montreal.cityloom`, un fichier tar muni d’un index SHA-256 que le serveur extrait au démarrage.
Git ignore les tuiles, le fond de carte et l’archive.

Rien n’exige les tuiles de routes : sans elles, tous les lieux, y compris celui par défaut, viennent d’Overpass. Sans le
fond de carte, la page Carte l’indique et n’affiche aucune carte.

## Tests et mise en forme

```sh
just db            # un Postgres jetable dans Docker pour les tests du serveur et la spécification du compte
just test          # cargo test --workspace, et une construction wasm32 du code que `cargo test` seul ne compile pas; exige `DATABASE_URL`
just e2e           # construit, puis exécute les tests Playwright de e2e/; la spécification du compte exige `DATABASE_URL`
just check         # les deux
just format        # rustfmt et Prettier
just format-check  # la même vérification, sans rien modifier
```

L’intégration continue exécute `just format-check`, `just test` et `just e2e` avec `RUSTFLAGS=-D warnings`; un nouvel
avertissement du compilateur la fait donc échouer. Elle construit aussi cette documentation avec `just docs-build`. Les tâches `test` et `e2e` de l’intégration continue
exécutent un service Postgres. Chaque demande de fusion augmente la version dans le `Cargo.toml` de la racine.

`just db` démarre le Postgres dans un conteneur nommé `cityloom-db`, sur 127.0.0.1:55432, et affiche la ligne à
exécuter : `export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/cityloom`. Les tests du serveur et la
spécification du compte (`e2e/tests/account-server.spec.ts`) l’exigent et le disent quand il manque. `just db-stop`
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

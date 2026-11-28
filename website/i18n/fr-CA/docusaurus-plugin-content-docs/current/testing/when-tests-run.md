---
title: Quand les tests s’exécutent
---

# Quand les tests s’exécutent

Cette page dit ce qu’exécute chaque flux de travail, et ce qui doit réussir avant une fusion. Les paliers qu’ils
exécutent sont dans la [vue d’ensemble](index.md).

## Flux de prévérification

`.github/workflows/presubmit.yml` s’exécute pour chaque demande de fusion, avec `RUSTFLAGS=-D warnings` : un nouvel
avertissement du compilateur le fait donc échouer. Un changement qui ne touche que la documentation (`website/**`,
`docs/**`, `**/*.md`, `LICENSE*`) ne le démarre pas.

Sa tâche `changes` exécute `scripts/ci/ci_changes.py`, qui décide de ce qui s’exécute d’après les fichiers que change la
demande de fusion :

- Les tests Rust des caisses qu’elle touche et des caisses qui en dépendent. Un changement à ce que lisent toutes les
  caisses, comme `web/`, `Cargo.lock`, le `justfile` ou un flux de travail, exécute toutes les caisses. La vérification
  wasm et celle du serveur de publication suivent les mêmes caisses.
  Une simple hausse de version (la ligne de version de `crates/cityloom_editor/Cargo.toml` et son entrée dans
  `Cargo.lock`) ne compte pas comme un changement à ces fichiers.
- Seules les spécifications de navigateur que ses fichiers peuvent toucher : la première règle de
  `scripts/ci/spec-map.json` qui correspond à un fichier décide, par groupe. Un fichier qu’aucune règle ne nomme
  exécute toutes les spécifications.
  Les fichiers propres à une page dans `web/` exécutent le groupe de cette page et `@cross`, les spécifications
  communes aux pages. Ce que charge chaque page (`app.css`, `boot.js`, `city.js`), le serveur et
  `crates/citymodel_builder/**`, qui fait les tuiles des données d’essai, exécutent toutes les spécifications.
- Chaque spécification choisie s’exécute dans son niveau, et un niveau sans spécification choisie ne s’exécute pas. Il
  n’y a pas de tâche de bout en bout.

Les tâches sont Format, Script tests, Unit tests, Wasm type-check, Release server has no fake provider, Build for the
browser tests, UI tests, Integration tests et Version bump, et `CI`, qui les rassemble.

## Flux de postvérification

`.github/workflows/postsubmit.yml` s’exécute à chaque poussée sur `main` (avec le même `paths-ignore`) et à la main. Il
n’a pas de tâche `changes` : il exécute toutes les tâches et toutes les spécifications, le niveau de l’interface en six
partitions Playwright, et le niveau de bout en bout par `scripts/e2e-archive.sh`. Il fixe aussi
`RUSTFLAGS=-D warnings`, et ses exécutions ne sont jamais annulées. Sa tâche `All checks` échoue quand une tâche a
échoué ou a été annulée, et `Deploy to Railway` l’attend. Le déploiement ne se fait que pour une poussée sur `main`;
une exécution à la main ne déploie jamais.

## Documentation

`.github/workflows/docs.yml` s’exécute quand un changement touche `website/**` ou le flux de travail. Il vérifie
`website/` avec Prettier (depuis `tests/e2e/`), puis construit le site dans les deux langues; un lien ou une ancre brisé fait
échouer la construction. Il publie le site sur GitHub Pages depuis `main`. Un changement à `docs/**` ou à un `*.md` hors
de `website/` ne démarre aucun flux de travail.

## SonarCloud

`.github/workflows/sonar.yml` s’exécute pour les demandes de fusion et sur `main`, avec le même `paths-ignore`. Il
exécute `just sonar-reports` : la couverture des tests sur l’hôte (sans `cityloom_server`) et les constats de Clippy
avec `--cap-lints warn`. Il n’a pas de `-D warnings`.

## Ce qui doit réussir avant une fusion

- **`CI`**, la dernière tâche du flux de prévérification. Elle échoue quand une tâche a échoué ou a été annulée, et
  laisse passer celles que `changes` a sautées.
- **SonarCloud.** Son analyse ne fait pas partie de `CI` et ne bloque jamais une fusion. L’agent de fusion
  (`.claude/agents/merge.md`) exige tout de même aucun problème ouvert et aucun point chaud sur la demande de fusion,
  et la barrière de qualité à `OK`. Les autres conditions de la barrière sont une couverture du nouveau code d’au
  moins 80 % et une duplication d’au plus 3 %.
- **La version.** `scripts/check-version.sh` exige que la version de `crates/cityloom_editor/Cargo.toml` soit
  supérieure à celle de la base. Un changement qui ne touche que la documentation et la configuration de l’intégration
  continue (`CI_CONFIGURATION` dans `ci_changes.py` : `.github/**`, `scripts/ci/**`, `scripts/include/**`,
  `.claude/**`) n’en exige pas, et la tâche est sautée.
- **La vérification Documentation**, pour un changement à `website/`. `CI` ne la couvre pas.
- **La fusion elle-même.** L’agent d’intégration (`.claude/agents/integrate.md`) ne fusionne que lorsque chaque
  vérification a réussi ou a été sautée, que GitHub indique `CLEAN` et que la dernière exécution de Postsubmit sur
  `main` a réussi. L’agent de déploiement (`.claude/agents/deploy.md`) suit ensuite le déploiement et vérifie le site en
  ligne. Dans l’usine logicielle, une personne approuve d’abord la tête de la demande de fusion.
- **Les barrières de l’usine** (`.claude/factory/gate.py`, décrites dans `.claude/factory/README.md`), pour un billet
  que l’usine construit :
  - `red` : les nouveaux tests compilent, chaque nouveau test échoue, et chaque ancien test réussit;
  - `green` : chaque test réussit avec `-D warnings`, la mise en forme et la version tiennent, et aucun fichier de
    test, bloc `#[cfg(test)]`, vérification ou fichier de l’usine n’a changé depuis le commit rouge;
  - `docs` : seule la documentation a changé, chaque page a sa page dans l’autre langue, et le site passe Prettier et
    se construit;
  - `tests`, pour un billet qui ne demande que des tests d’un code qui fonctionne déjà : seuls des tests ont changé
    (et la ligne de version), chaque test existant est tel qu’il était, chaque test réussit comme pour `green`, et chaque
    nouveau test échoue sous au moins un défaut du code que le plan nomme (par mutation : la barrière introduit chaque
    défaut, tiré de `.claude/factory/faults.py` et de `cargo-mutants`, exécute les nouveaux tests et le retire);
  - `faults` : avant qu’un plan de ce flux soit publié, chaque fonction, message et déclaration que ses défauts nomment
    existe et donne un défaut. Elle ne construit rien.

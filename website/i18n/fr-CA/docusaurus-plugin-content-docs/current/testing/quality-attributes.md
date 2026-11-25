---
title: Attributs de qualité
---

# Attributs de qualité

Cette page dit ce qui vérifie l’accessibilité, la traduction et la sécurité. Les paliers sont dans
[Paliers de tests](levels.md), et la racine des chemins de l’éditeur, dans la [vue d’ensemble](index.md).

## Accessibilité

La cible est WCAG 2.2 AA.

- `e2e/tests/a11y.spec.ts` exécute axe (`@axe-core/playwright`) avec les étiquettes `wcag2a`, `wcag2aa`, `wcag21a`,
  `wcag21aa` et `wcag22aa`. `compare.spec.ts` exécute aussi axe.
- `e2e/tests/junction-a11y.spec.ts` mesure les cibles de la jonction à 1440, 1050, 600 et 280 px de large.
- `shared/contrast.rs` mesure les jetons de couleur du CSS (voir [Les gardes](levels.md#les-gardes)).

## Traduction

- Le test de parité de chaque tranche vérifie que ses deux fichiers `.ftl` ont les mêmes messages avec les mêmes
  variables (`shared::i18n::tests::parity_problems`) : `account`, `archive`, `city`, `junction`, `map`, `place`,
  `project`, `restriction`, `share`, `shell`, `street` et `sync`.
- `i18n_guard` trouve la formulation restée dans le code (voir [Les gardes](levels.md#les-gardes)).
- `shared::i18n::typography` applique au fr-CA les espacements de l’OQLF, et est testée à côté.
- `e2e/tests/language.spec.ts` fait passer une page au fr-CA depuis le menu du compte.

## Sécurité

- `PgRepo` scelle ce qu’il écrit et ouvre ce qu’il lit. Le corps d’un aménagement partagé est scellé sous
  `Token::body_key`. `no_plaintext.rs` vérifie que les lignes ne contiennent rien en clair.
- `just check-no-fake` construit le serveur de publication et exécute `scripts/check-no-fake.sh`, qui échoue sur les
  marqueurs du faux fournisseur, `fake-login` et `fake-apple`.
- La tâche `Release server has no fake provider` de l’intégration continue construit aussi le serveur avec
  `test-provider` et s’attend à ce que ce script échoue sur lui : un contrôle positif, pour que les marqueurs ne
  puissent pas dériver sans qu’on le voie.

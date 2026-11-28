---
title: Quality attributes
---

# Quality attributes

This page says what checks accessibility, translation and security. The levels are in
[Levels of tests](levels.md), and the root of the editor's paths is in the [overview](index.md).

## Accessibility

The target is WCAG 2.2 AA.

- `tests/e2e/tests/a11y.spec.ts` runs axe (`@axe-core/playwright`) with the tags `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`
  and `wcag22aa`. `compare.spec.ts` runs axe too.
- `tests/e2e/tests/junction-a11y.spec.ts` measures the junction's targets at 1440, 1050, 600 and 280 px wide.
- `shared/contrast.rs` measures the colour tokens of the CSS (see [The guards](levels.md#the-guards)).

## Translation

- Each slice's parity test checks that its two `.ftl` files have the same messages with the same variables
  (`shared::i18n::tests::parity_problems`): `account`, `archive`, `city`, `junction`, `map`, `place`, `project`,
  `restriction`, `share`, `shell`, `street` and `sync`.
- `i18n_guard` finds wording left in the code (see [The guards](levels.md#the-guards)).
- `shared::i18n::typography` applies the OQLF's spacing to fr-CA, and is tested beside it.
- `tests/e2e/tests/language.spec.ts` switches a page to fr-CA from the account menu.

## Security

- `PgRepo` seals what it writes and opens what it reads. A shared design's body is sealed under `Token::body_key`.
  `no_plaintext.rs` checks the rows hold nothing in the clear.
- `just check-no-fake` builds the release server and runs `scripts/check-no-fake.sh`, which fails on the fake
  provider's markers, `fake-login` and `fake-apple`.
- CI's `Release server has no fake provider` job also builds the server with `test-provider` and expects that script to
  fail on it: a positive control, so the markers cannot drift unseen.

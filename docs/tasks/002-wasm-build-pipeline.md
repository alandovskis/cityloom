---
id: 002
title: "Add the wasm-pack build pipeline and idempotent setup recipe"
depends_on: []
features: [OPS-009, OPS-010, OPS-011, OPS-015]
status: todo
acceptance:
  - "just verify"
  - "just setup && just setup"
  - "test -f web/pkg/cityloom_client_bg.wasm"
model_hint: haiku   # mechanical: wire existing tools, no design decisions
---

# Context
`just setup` and `just build-wasm` are written but unproven; `wasm-pack` is not
installed on the reference machine. The verify gate currently type-checks the
client against the wasm target but never produces a loadable bundle.

# Scope
- Make `just setup` install the wasm target, `wasm-pack`, the uv environment,
  and the Playwright chromium browser, and be safely re-runnable.
- Make `just build-wasm` emit `web/pkg/cityloom_client_bg.wasm`.
- Replace the `wasm` stage in `scripts/verify.sh` with the wasm-pack build so
  the gate covers bindgen output, not just compilation.
- Keep `uv.lock` current so `uv run --frozen` works on a clean checkout.

# Out of scope
- Any client source changes. The crate stays a stub until task 027.
- Bundle size optimisation. WASM-011 belongs to task 027.
- Tailwind. That is task 042.

# Acceptance criteria (beyond the acceptance commands)
- A second `just setup` on an already-provisioned machine exits 0 and
  reinstalls nothing.
- `just verify` still passes with the wasm-pack stage substituted in.

# Tasks

One file per task, shaped by `docs/templates/task-template.md`: YAML
frontmatter (`id`, `title`, `depends_on`, `features`, `status`, `acceptance`,
`model_hint`) followed by Context, Scope, Out of scope and Acceptance criteria.

`status` values:

| Value | Meaning |
|---|---|
| `todo` | written, not yet unblocked (a `depends_on` task is open) |
| `READY` | every dependency is done; `./scripts/next-tasks.sh` lists it |
| `doing` | in progress (name the branch in `docs/state/handoff.md`) |
| `done` | acceptance commands pass; `just verify` exits 0 |
| `blocked` | cannot proceed; the reason is in the handoff note |

`./scripts/state-summary.sh` prints the READY list at session start, per
`CLAUDE.md`. The AI-DLC workflow under `aidlc/` is the source of truth for
what is being built; tasks here are the session-sized slices of it.

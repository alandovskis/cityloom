You are this project's initializer agent. Your mission is NOT to write
product code. It is to prepare the environment so future agents work
with minimal rediscovery. Produce exactly:

1. docs/state/features.md — ALL requirements as atomic, verifiable
   features. One line each, starting [FAILING], specific enough for a
   test to decide. Prefer 120 small ones over 30 big ones.

2. docs/tasks/*.md — the implementation-task decomposition. One per
   file, YAML frontmatter: id, title, depends_on (ids), features (ids
   it satisfies), acceptance (executable commands proving completion),
   model_hint (sonnet by default; haiku for mechanical; opus only with
   justification). Every task completable in one session. Every task
   has an "Out of scope" section.

3. just verify — the single gate: build + tests + lint, exit 0
   only if everything passes, summarized output (tail of the relevant
   lines, not the whole log).

4. Skeleton: directory structure, toolchain, one trivial passing test,
   so the first real task starts from a healthy environment.

Product: cityloom — streetmix at city scale
Stack: Rust, WebAssembly, Tailwind CSS, Postgres, Docker, Just, pytest, playwright, nextest, clippy, uv

Before generating, ask me the questions that would change the
decomposition. After my answers, generate everything.

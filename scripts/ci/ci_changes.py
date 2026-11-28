#!/usr/bin/env python3
"""Decides what CI runs for a change (`.github/workflows/presubmit.yml`'s `changes` job).

    scripts/ci/ci_changes.py --event pull_request --base <sha> [--head HEAD]

Reads the files the change touches (git diff base..head) and the rules in scripts/ci/spec-map.json, prints what it
decided and, when GITHUB_OUTPUT is set, writes the job's outputs:

    unit       true unless every file is documentation (presubmit.yml does not start for a change to documentation alone,
               so in CI this is always true)
    rust       true when some Rust crate has tests to run: the changed crates and every crate that depends on them (a
               change to anything else Rust reads, such as web/, Cargo.lock or a workflow, runs every crate)
    rust_args  their `cargo` arguments ("-p a -p b"; "" runs the whole workspace)
    wasm       true when some selected crate is built for wasm32; wasm_args the same for it
    server_release   true when the server is among the selected crates (its release build is checked for the fake provider)
    version    true unless every file is documentation or CI configuration (CI_CONFIGURATION): the version must be raised
    browser    true when some browser tier runs (the shared build of the wasm and the server)
    ui         true when the UI tier runs (tests/e2e/tiers.json: no database, no archive)
    ui_specs   its Playwright arguments ("" runs every spec of the tier)
    integration, integration_specs   the same for the integration tier (the server and a Postgres)
    e2e        true when the E2E tier runs (`just e2e-archive`: the unpacked archive and the delivery specs)
    e2e_specs  its Playwright arguments
    deploy     push only: true when a file the site is built from (DEPLOY_INPUTS) changed since --base, which
               postsubmit.yml sets to the last deployed commit; also true when that base is not known or not behind HEAD.
               A Rust file whose only change is inside #[cfg(test)] items is not a change (outside_tests); one it
               cannot read is.

Only a pull request is narrowed. postsubmit.yml, which checks main, runs every spec and does not call this. Anything
doubtful runs every spec too: no files, a base that is not there, a file no rule names. A version bump alone
(the version in the editor's Cargo.toml and its line of Cargo.lock) is not a change to those files.

Standard library only; scripts/include/tests/test_ci_changes.py tests it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDITOR_TOML = "crates/cityloom_editor/Cargo.toml"  # where the product's version is
TIERS = ("ui", "integration", "e2e")  # the tiers of tests/e2e/tiers.json, and the CI job of each
# CI configuration and its tooling: they change nothing the pages show, so they need no version bump (they still run
# every test).
CI_CONFIGURATION = (".github/**", "scripts/ci/**", "scripts/include/**", ".claude/**")
# What no Rust crate reads: a change to only these leaves the crates' tests as they were. Anything else outside crates/
# (web/, tests/e2e/fixtures, Cargo.lock, the justfile, the workflows) runs every crate: the tests include_str! the pages, read the
# fixtures and are built by those files.
RUST_NEUTRAL = ("tests/e2e/tests/**", "website/**", "scripts/ci/**", "scripts/include/**", ".claude/**")
# `just test-wasm` leaves these out: they are not built for the browser.
NOT_WASM = ("cityloom_server", "citymodel_builder")
CRATES = ROOT / "crates"
# What `just deploy` and postsubmit.yml's deploy job build the site from (scripts/deploy.sh, scripts/build.sh, the tile
# cache's key): a push to main that changes one of these since the last deploy deploys; any other does not.
DEPLOY_INPUTS = (
    "crates/**",  # every crate is served: the server links citymodel_builder, and the wasm is built from the rest
    "web/**",
    "deploy/**",
    "cities/**",
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "justfile",
    "scripts/build.sh",
    "scripts/deploy.sh",
    "scripts/target-dir.sh",
    "scripts/retry.sh",
    "scripts/check-no-fake.sh",
    "scripts/precompress.mjs",
    ".github/workflows/postsubmit.yml",  # the deploy job's own steps
)
# Inside a folder of DEPLOY_INPUTS, and not served: the tests, the guards, the notes.
CARVED_OUT = ("**/tests.rs", "crates/*/tests/**", "crates/cityloom_editor/src/*_guard.rs", "**/*.md")
# Everything else git tracks. scripts/include/tests/test_ci_changes.py fails on a file in neither list, so a new kind of
# file is put on one side by the pull request that adds it.
NOT_SERVED = (
    "tests/**",
    "website/**",
    "docs/**",
    "LICENSE*",
    "*.md",
    ".claude/**",
    ".impeccable/**",
    "dockerfiles/**",  # the toolchain images to build and test in; the runtime image is deploy/Dockerfile
    ".github/actions/**",
    ".github/ISSUE_TEMPLATE/**",
    ".github/dependabot.yml",
    ".github/issue-flow.json",
    ".github/issue-labeler.yml",
    ".github/labels.yml",
    ".github/workflows/presubmit.yml",
    ".github/workflows/docs.yml",
    ".github/workflows/sonar.yml",
    ".github/workflows/labels.yml",
    ".github/workflows/issue-labels.yml",
    "scripts/ci/**",
    "scripts/include/**",
    "scripts/e2e-archive.sh",
    "scripts/check-version.sh",
    "scripts/pin-server.sh",
    "sonar-project.properties",
    "rustfmt.toml",
    ".prettierrc.json",
    ".prettierignore",
    ".gitignore",
    ".env.example",
)


def glob(pattern: str, path: str) -> bool:
    """`**` crosses folders (`**/` may be empty), `*` and `?` stay inside one."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif pattern.startswith("**", i):
            out += ".*"
            i += 2
        elif pattern[i] == "*":
            out += "[^/]*"
            i += 1
        elif pattern[i] == "?":
            out += "[^/]"
            i += 1
        else:
            out += re.escape(pattern[i])
            i += 1
    return re.fullmatch(out, path) is not None


def rule_for(rules: dict, path: str) -> dict | None:
    return next((r for r in rules["rules"] if any(glob(p, path) for p in r["paths"])), None)


def matched(rules: dict, path: str) -> bool:
    return rule_for(rules, path) is not None


def crate_dependents() -> dict[str, set[str]]:
    """For each crate of crates/, the crates that depend on it by path (`x = { path = "../x" }`), from their Cargo.toml."""
    dependents: dict[str, set[str]] = {d.name: set() for d in CRATES.iterdir() if (d / "Cargo.toml").is_file()}
    for name in dependents:
        toml = (CRATES / name / "Cargo.toml").read_text(encoding="utf-8")
        for dep in re.findall(r'path\s*=\s*"\.\./([\w-]+)"', toml):
            if dep in dependents:
                dependents[dep].add(name)
    return dependents


def rust_crates(rules: dict, files: list[str], dependents: dict[str, set[str]]) -> list[str] | None:
    """The crates whose tests a change needs, sorted: the crates it touches and every crate depending on them, directly or
    not. None is every crate (a file that no crate owns and that the crates may read). Empty is none."""
    touched: set[str] = set()
    for f in files:
        rule = rule_for(rules, f)
        if (rule is not None and rule.get("docs")) or any(glob(p, f) for p in RUST_NEUTRAL):
            continue
        owner = f.split("/")[1] if f.startswith("crates/") and f.count("/") >= 2 else None
        if owner not in dependents:
            return None
        touched.add(owner)
    closure, todo = set(touched), list(touched)
    while todo:
        for user in dependents[todo.pop()]:
            if user not in closure:
                closure.add(user)
                todo.append(user)
    return sorted(closure)


@dataclass
class Selection:
    docs_only: bool = False
    version: bool = True
    all: bool = False
    crates: list[str] | None = None  # None: every crate
    specs: list[str] = field(default_factory=list)
    tiers: dict = field(default_factory=dict)  # tests/e2e/tiers.json

    @property
    def unit(self) -> bool:
        return not self.docs_only

    @property
    def rust(self) -> bool:
        return not self.docs_only and (self.crates is None or bool(self.crates))

    @property
    def rust_args(self) -> str:
        return "" if self.crates is None else " ".join(f"-p {c}" for c in self.crates)

    @property
    def wasm_crates(self) -> list[str] | None:
        """The selected crates `just test-wasm` builds; None when it builds the whole workspace."""
        return None if self.crates is None else [c for c in self.crates if c not in NOT_WASM]

    @property
    def wasm(self) -> bool:
        return not self.docs_only and (self.crates is None or bool(self.wasm_crates))

    @property
    def wasm_args(self) -> str:
        return "" if self.crates is None else " ".join(f"-p {c}" for c in self.wasm_crates or [])

    @property
    def server_release(self) -> bool:
        return not self.docs_only and (self.crates is None or "cityloom_server" in self.crates)

    def runs(self, tier: str) -> bool:
        """Whether the tier has a spec to run (every one on main and for a change nobody can narrow)."""
        return not self.docs_only and (self.all or bool(set(self.tiers[tier]) & set(self.specs)))

    @property
    def ui(self) -> bool:
        return self.runs("ui")

    @property
    def integration(self) -> bool:
        return self.runs("integration")

    @property
    def e2e(self) -> bool:
        return self.runs("e2e")

    @property
    def browser(self) -> bool:
        return any(self.runs(t) for t in TIERS)

    def spec_args(self, tier: str) -> str:
        """The Playwright arguments for one tier: "" runs all of it, else the changed specs it owns."""
        if self.all:
            return ""
        chosen = set(self.tiers[tier]) & set(self.specs)
        if tier == "e2e" and "archive" in chosen:
            chosen |= set(self.tiers["archive_also"])  # the archive spec's change is judged by what the pages do over it
        return " ".join(f"tests/{s}.spec.ts" for s in sorted(chosen))


def specs_for(rules: dict, every_spec: list[str], f: str) -> set[str] | None:
    """The specs one changed file asks for; None when it asks for every spec."""
    specs = rule_for(rules, f)["specs"]
    if specs == "all":
        return None
    if specs == "self":
        name = Path(f).name[: -len(".spec.ts")]
        return {name} if name in every_spec else set()
    chosen: set[str] = set()
    if specs != "none":
        for s in specs:
            chosen.update(rules["groups"][s[1:]] if s.startswith("@") else [s])
    return chosen


def select(
    rules: dict,
    every_spec: list[str],
    files: list[str],
    tiers: dict,
    empty_means_none: bool = False,
    dependents: dict[str, set[str]] | None = None,
) -> Selection:
    if not files:
        return Selection(all=not empty_means_none, tiers=tiers)
    docs = 0
    no_version = 0
    chosen: set[str] = set()
    everything = False
    for f in files:
        rule = rule_for(rules, f)
        if (rule is not None and rule.get("docs")) or any(glob(p, f) for p in CI_CONFIGURATION):
            no_version += 1
        if rule is None:
            everything = True
        elif rule.get("docs"):
            docs += 1
        else:
            asked = specs_for(rules, every_spec, f)
            if asked is None:
                everything = True
            else:
                chosen |= asked
    return Selection(
        docs_only=docs == len(files),
        version=no_version < len(files),
        all=everything,
        crates=rust_crates(rules, files, dependents if dependents is not None else crate_dependents()),
        specs=sorted(every_spec if everything else chosen),
        tiers=tiers,
    )


def normalize_toml(text: str) -> str:
    return re.sub(r'(?m)^version = ".*"$', 'version = "-"', text, count=1)


def normalize_lock(text: str) -> str:
    return re.sub(r'(name = "cityloom_editor"\nversion = ")[^"]*', r"\1-", text)


REVISION = re.compile(r"[0-9a-fA-F]{7,64}|HEAD")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def changed_files(base: str, head: str) -> list[str] | None:
    """The files that differ, a bare version bump left out; None when the base is not known."""
    if not base or set(base) == {"0"}:
        return None
    if not (REVISION.fullmatch(base) and REVISION.fullmatch(head)):
        return None  # only a commit id or HEAD goes to git
    try:
        files = [f for f in git("diff", "--name-only", "--no-renames", base, head, "--").splitlines() if f]
        for name, norm in ((EDITOR_TOML, normalize_toml), ("Cargo.lock", normalize_lock)):
            if name in files and norm(git("show", f"{base}:{name}")) == norm(git("show", f"{head}:{name}")):
                files.remove(name)
        return files
    except subprocess.CalledProcessError:
        return None


def served(path: str) -> bool:
    """Whether the deployed site is built from this file."""
    return any(glob(p, path) for p in DEPLOY_INPUTS) and not any(glob(p, path) for p in CARVED_OUT)


# Rust's tokens, as far as telling code from #[cfg(test)] items needs: whitespace and comments are not tokens, a string
# or char is one token whatever it holds, and an operator of several characters is one token (so `- >` is not `->`).
OPERATORS = sorted(
    "<<= >>= ... ..= :: -> => == != <= >= && || += -= *= /= %= ^= &= |= << >> ..".split(), key=len, reverse=True
)
RAW_STRING_START = re.compile(r"[bc]?r(#*)\"")
STRING = re.compile(r"[bc]?\"(?:[^\"\\]|\\.)*\"", re.DOTALL)
CHAR = re.compile(r"b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]{1,6}\}|.)|[^'\\\n])'")
LIFETIME = re.compile(r"'(?:r#)?[^\W\d]\w*")
WORD = re.compile(r"(?:r#)?\w+")  # identifiers, keywords and numbers (a number's parts may split; both sides alike)
MARK = ["#", "[", "cfg", "(", "test", ")", "]"]  # exactly #[cfg(test)]: the code that only `cargo test` builds
PAIRS = {")": "(", "]": "[", "}": "{"}


def rust_tokens(text: str) -> list[str] | None:
    """The tokens of a Rust file; None when a string, char or comment does not end."""
    tokens: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c.isspace():
            i += 1
        elif text.startswith("//", i):
            end = text.find("\n", i)
            i = n if end < 0 else end + 1
        elif text.startswith("/*", i):
            depth, i = 1, i + 2
            while depth:
                if i >= n:
                    return None
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif m := RAW_STRING_START.match(text, i):
            end = text.find('"' + m.group(1), m.end())
            if end < 0:
                return None
            end += 1 + len(m.group(1))
            tokens.append(text[i:end])
            i = end
        elif c == '"' or (c in "bc" and text.startswith('"', i + 1)):
            m = STRING.match(text, i)
            if m is None:
                return None
            tokens.append(m.group())
            i = m.end()
        elif m := CHAR.match(text, i) or LIFETIME.match(text, i) or WORD.match(text, i):
            tokens.append(m.group())
            i = m.end()
        else:
            op = next((o for o in OPERATORS if text.startswith(o, i)), c)
            tokens.append(op)
            i += len(op)
    return tokens


def item_end(tokens: list[str], i: int) -> int | None:
    """Where the item that starts at i ends (after any further outer attributes): after its first `;` outside brackets, or
    after the `}` closing its first block (and a `;` right after it). None when it reaches a `,` (outside `<…>`, which
    holds a signature's generics), a `=>`, its parent's closer or the end first: a field, an argument or an arm, which
    this does not take apart."""
    stack: list[str] = []
    angles = 0  # `<` minus `>` outside brackets; a `<` that is less-than only keeps a `,` from ending the item
    bounds = False  # past a `where`, whose bounds a `,` separates (no field, argument or arm holds the keyword)
    for k in range(i, len(tokens)):
        t = tokens[k]
        if not stack and t == "where":
            bounds = True
        elif not stack and t in ("<", ">", ">>"):
            angles += {"<": 1, ">": -1, ">>": -2}[t]
        elif not stack and t == "=>":
            return None
        elif t in "([{":
            stack.append(t)
        elif t in PAIRS:
            if not stack or stack.pop() != PAIRS[t]:
                return None
            if not stack and t == "}":
                return k + 2 if tokens[k + 1 : k + 2] == [";"] else k + 1
        elif not stack and t == ";":
            return k + 1
        elif not stack and t == "," and angles <= 0 and not bounds:
            return None
    return None


def balanced(tokens: list[str]) -> bool:
    stack: list[str] = []
    for t in tokens:
        if t in "([{":
            stack.append(t)
        elif t in PAIRS and (not stack or stack.pop() != PAIRS[t]):
            return False
    return not stack


def drop_outer_attributes(out: list[str]) -> None:
    """Remove the outer attributes (`#[…]`, not `#![…]`) that end the tokens kept so far."""
    while out and out[-1] == "]":
        depth, k = 0, len(out) - 1
        while k >= 0:
            depth += {"]": 1, "[": -1}.get(out[k], 0)
            if depth == 0:
                break
            k -= 1
        if k < 1 or out[k - 1] != "#":
            return  # an index or array, or an inner attribute's `!`: not an outer attribute
        del out[k - 1 :]


def outside_tests(text: str) -> list[str] | None:
    """The tokens of a Rust file with every #[cfg(test)] item left out: what the deployed site is built from. None when
    the file cannot be read with confidence (it then counts as changed)."""
    tokens = rust_tokens(text)
    if tokens is None or not balanced(tokens):
        return None
    out: list[str] = []
    i = 0
    while i < len(tokens):
        if tokens[i : i + len(MARK)] == MARK:
            end = item_end(tokens, i + len(MARK))
            if end is None:
                return None
            drop_outer_attributes(out)  # the attributes above the mark are the item's too
            i = end
        else:
            out.append(tokens[i])
            i += 1
    return out


def changed_outside_tests(path: str, base: str, head: str) -> bool:
    """Whether a Rust file differs between base and head outside its #[cfg(test)] items. A file missing on a side, one
    that cannot be read, and any git error count as changed."""
    if not (REVISION.fullmatch(base) and REVISION.fullmatch(head)):
        return True
    try:
        old, new = (outside_tests(git("show", f"{rev}:{path}")) for rev in (base, head))
    except (subprocess.CalledProcessError, UnicodeDecodeError):
        return True
    return old is None or new is None or old != new


def deploys(files: list[str] | None, base: str = "", head: str = "HEAD") -> bool:
    """Whether a push with these changed files deploys; None (the change is not known) deploys. With the base, a served
    Rust file whose only change is inside #[cfg(test)] items does not count: no code the site is built from changed."""
    if files is None:
        return True

    def counts(f: str) -> bool:
        if not served(f):
            return False
        return not (base and f.endswith(".rs")) or changed_outside_tests(f, base, head)

    return any(counts(f) for f in files)


def behind(base: str, head: str) -> bool:
    """Whether base is an ancestor of head (or head itself), so that the diff between them is what head adds."""
    if not (REVISION.fullmatch(base) and REVISION.fullmatch(head)):
        return False
    try:
        git("merge-base", "--is-ancestor", base, head)
        return True
    except subprocess.CalledProcessError:
        return False


def describe(rule: dict | None) -> str:
    if rule is None:
        return "no rule: all"
    return "docs" if rule.get("docs") else str(rule["specs"])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--event", default="pull_request")
    ap.add_argument("--base", default="")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("files", nargs="*", help="the files, instead of git diff (for trying)")
    a = ap.parse_args()

    rules = json.loads((ROOT / "scripts" / "ci" / "spec-map.json").read_text(encoding="utf-8"))
    tiers = json.loads((ROOT / "tests" / "e2e" / "tiers.json").read_text(encoding="utf-8"))
    every = sorted(p.name[: -len(".spec.ts")] for p in (ROOT / "tests" / "e2e" / "tests").glob("*.spec.ts"))
    if a.files:
        files = a.files
    elif a.event == "push" and not behind(a.base, a.head):
        files = None  # the last deploy is not behind this push (or not known): everything is new
    else:
        files = changed_files(a.base, a.head)
    if files is None:
        print("The change is not known: everything runs.")
        s = Selection(all=True, tiers=tiers)
    else:
        # A change of a version number alone leaves nothing to test but the unit tests.
        s = select(rules, every, files, tiers, empty_means_none=True)
        for f in files:
            print(f"  {f}: {describe(rule_for(rules, f))}")
    if a.event != "pull_request" and not s.docs_only:
        s = Selection(all=True, version=s.version, tiers=tiers)
    out = {
        "unit": str(s.unit).lower(),
        "rust": str(s.rust).lower(),
        "rust_args": s.rust_args,
        "wasm": str(s.wasm).lower(),
        "wasm_args": s.wasm_args,
        "server_release": str(s.server_release).lower(),
        "version": str(s.version).lower(),
        "browser": str(s.browser).lower(),
        "ui": str(s.ui).lower(),
        "ui_specs": s.spec_args("ui"),
        "integration": str(s.integration).lower(),
        "integration_specs": s.spec_args("integration"),
        "e2e": str(s.e2e).lower(),
        "e2e_specs": s.spec_args("e2e"),
    }
    if a.event == "push":
        out["deploy"] = str(deploys(files, "" if a.files else a.base, a.head)).lower()
    print(json.dumps(out))
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as fh:
            fh.writelines(f"{k}={v}\n" for k, v in out.items())
    return 0


if __name__ == "__main__":
    sys.exit(main())

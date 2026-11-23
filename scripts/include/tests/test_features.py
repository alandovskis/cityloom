"""Keeps the feature list whole: `just test-scripts`.

`scripts/ci/features.json` lists the product's features at the level a person tests by hand, so that a test can be
mapped to the feature it covers (the test inventory). Each feature has a stable id, a name in English and in fr-CA,
the pages it lives on, the slice or crate that owns it, the paths of its code (globs) and the browser specs that
cover it. The checks below hold the file against the repository, and are tried against fixture trees that break it.

Every slice of the editor, every other crate, every page of `web/` and every spec of `e2e/tiers.json` belongs to at
least one feature: a slice or a crate when a feature names it as its owner or one of its globs lies inside it.
"""

from __future__ import annotations

import json
import re
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

FEATURES = Path("scripts/ci/features.json")
EDITOR_SRC = Path("crates/cityloom_editor/src")
CRATES = Path("crates")
WEB = Path("web")
TESTS = Path("e2e/tests")
TIERS = Path("e2e/tiers.json")

FIELDS = {"id", "en", "fr-CA", "pages", "owner", "code", "specs"}
ID = re.compile(r"[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)*")


def problems(root: Path) -> list[str]:
    """Everything wrong with the feature list of the tree at `root`, one line each; empty when it is whole."""
    try:
        features = json.loads((root / FEATURES).read_text(encoding="utf-8"))["features"]
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f"{FEATURES}: cannot be read as {{\"features\": [...]}} ({error})"]

    slices = {entry.relative_to(root).as_posix() for entry in (root / EDITOR_SRC).iterdir() if entry.is_dir()}
    crates = {entry.relative_to(root).as_posix() for entry in (root / CRATES).iterdir()
              if entry.is_dir() and entry.name != "cityloom_editor"}
    pages = {entry.name for entry in (root / WEB).glob("*.html")}
    tiers = json.loads((root / TIERS).read_text(encoding="utf-8"))
    tiered = {spec for tier in ("ui", "integration", "e2e") for spec in tiers.get(tier, [])}

    found: list[str] = []
    seen: set[str] = set()
    reached: set[str] = set()
    paged: set[str] = set()
    specced: set[str] = set()
    for index, entry in enumerate(features):
        name = entry.get("id") if isinstance(entry, dict) else None
        where = f"{FEATURES} feature {name or index}"
        if not isinstance(entry, dict):
            found.append(f"{where}: is not an object")
            continue
        found += [f"{where}: has no `{field}`" for field in sorted(FIELDS - entry.keys())]
        found += [f"{where}: has an unknown field `{field}`" for field in sorted(entry.keys() - FIELDS)]

        if not isinstance(name, str) or not ID.fullmatch(name):
            found.append(f"{where}: the id {name!r} is not lower-case words joined by dots")
        elif name in seen:
            found.append(f"{where}: the id {name} is used twice")
        seen.add(name if isinstance(name, str) else "")

        for language in ("en", "fr-CA"):
            if language in entry and not (isinstance(entry[language], str) and entry[language].strip()):
                found.append(f"{where}: the {language} name is empty")

        owner = entry.get("owner")
        if "owner" in entry:
            if owner in slices or owner in crates:
                reached.add(owner)
            else:
                found.append(f"{where}: the owner {owner!r} is neither a slice of {EDITOR_SRC} nor a crate")

        code = entry.get("code", [])
        if "code" in entry and not code:
            found.append(f"{where}: lists no code")
        for pattern in code:
            matched = [path for path in root.glob(pattern) if path.is_file()]
            if not matched:
                found.append(f"{where}: the glob {pattern} matches no file")
            for path in matched:
                text = path.relative_to(root).as_posix()
                reached.update(part for part in slices | crates if text.startswith(part + "/"))

        for page in entry.get("pages", []):
            if page in pages:
                paged.add(page)
            else:
                found.append(f"{where}: the page {page} is not in {WEB}")

        for spec in entry.get("specs", []):
            if not (root / TESTS / f"{spec}.spec.ts").is_file():
                found.append(f"{where}: the spec {spec} has no file {TESTS / (spec + '.spec.ts')}")
            elif spec not in tiered:
                found.append(f"{where}: the spec {spec} is in no tier of {TIERS}")
            specced.add(spec)

    found += [f"the slice {slice_} belongs to no feature" for slice_ in sorted(slices - reached)]
    found += [f"the crate {crate} belongs to no feature" for crate in sorted(crates - reached)]
    found += [f"the page {WEB / page} belongs to no feature" for page in sorted(pages - paged)]
    found += [f"the spec {spec} of {TIERS} belongs to no feature" for spec in sorted(tiered - specced)]
    return found


# Fixture trees.


def write(root: Path, path: Path | str, text: str) -> None:
    file = root / path
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(text, encoding="utf-8")


def feature(**changes) -> dict:
    base = {
        "id": "street.pieces",
        "en": "A street's cross-section is laid out from pieces",
        "fr-CA": "La coupe d’une rue se compose d’éléments",
        "pages": ["street.html"],
        "owner": "crates/cityloom_editor/src/street",
        "code": ["crates/cityloom_editor/src/street/**/*.rs"],
        "specs": ["street"],
    }
    base.update(changes)
    return base


def tree(root: Path, features: list[dict] | None = None, *, slices=("street",), crates=("cityloom_editor",),
         pages=("street.html",), ui=("street",), others: dict[str, list[str]] | None = None,
         spec_files: tuple[str, ...] | None = None) -> Path:
    """A small repository: the editor's slices, the other crates, the pages, the specs and their tiers, and the list.

    By default the one feature covers everything; a test changes one thing to see the check fail.
    """
    for name in slices:
        write(root, EDITOR_SRC / name / "mod.rs", "")
    for name in crates:
        if name != "cityloom_editor":
            write(root, CRATES / name / "src" / "lib.rs", "")
    for name in pages:
        write(root, WEB / name, "<!doctype html>\n")
    tiers = {"_comment": "fixture", "ui": list(ui), "integration": [], "e2e": [], "archive_also": []}
    tiers.update(others or {})
    write(root, TIERS, json.dumps(tiers))
    for name in spec_files if spec_files is not None else tuple(ui) + tuple(n for v in (others or {}).values() for n in v):
        write(root, TESTS / f"{name}.spec.ts", "")
    listed = features if features is not None else [feature()]
    write(root, FEATURES, json.dumps({"_comment": "fixture", "features": listed}, ensure_ascii=False))
    return root


class Case(unittest.TestCase):
    def setUp(self) -> None:
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)

    def tearDown(self) -> None:
        self._dir.cleanup()

    def assertOneProblem(self, root: Path, *words: str) -> None:
        found = problems(root)
        self.assertEqual(len(found), 1, found)
        for word in words:
            self.assertIn(word, found[0])


class TheFile(Case):
    def test_a_whole_list_has_no_problem(self) -> None:
        self.assertEqual(problems(tree(self.root)), [])

    def test_a_missing_file_is_a_problem(self) -> None:
        root = tree(self.root)
        (root / FEATURES).unlink()
        self.assertOneProblem(root, str(FEATURES))

    def test_a_file_that_is_not_json_is_a_problem(self) -> None:
        root = tree(self.root)
        write(root, FEATURES, "{ not json")
        self.assertOneProblem(root, str(FEATURES))

    def test_a_field_missing_or_unknown_is_named(self) -> None:
        broken = feature()
        del broken["owner"]
        broken["tests"] = []
        found = problems(tree(self.root, [broken]))
        self.assertTrue(any("owner" in p for p in found), found)
        self.assertTrue(any("tests" in p for p in found), found)


class Ids(Case):
    def test_two_features_with_one_id_are_a_problem(self) -> None:
        self.assertOneProblem(tree(self.root, [feature(), feature(en="Again", **{"fr-CA": "Encore"})]),
                              "street.pieces")

    def test_an_id_that_is_not_dotted_lower_case_is_a_problem(self) -> None:
        self.assertOneProblem(tree(self.root, [feature(id="Street Pieces")]), "Street Pieces")


class Names(Case):
    def test_an_empty_english_name_is_a_problem(self) -> None:
        self.assertOneProblem(tree(self.root, [feature(en=" ")]), "street.pieces", "en")

    def test_a_missing_french_name_is_a_problem(self) -> None:
        self.assertOneProblem(tree(self.root, [feature(**{"fr-CA": ""})]), "street.pieces", "fr-CA")


class Paths(Case):
    def test_a_glob_that_matches_no_file_is_a_problem(self) -> None:
        root = tree(self.root, [feature(code=["crates/cityloom_editor/src/street/**/*.rs",
                                              "crates/cityloom_editor/src/gone/*.rs"])])
        self.assertOneProblem(root, "crates/cityloom_editor/src/gone/*.rs")

    def test_a_feature_with_no_code_is_a_problem(self) -> None:
        self.assertOneProblem(tree(self.root, [feature(code=[])]), "street.pieces", "code")

    def test_an_owner_that_is_not_a_slice_or_a_crate_is_a_problem(self) -> None:
        root = tree(self.root, [feature(owner="crates/cityloom_editor/src/street/model")])
        self.assertTrue(any("crates/cityloom_editor/src/street/model" in p for p in problems(root)))

    def test_a_page_not_in_web_is_a_problem(self) -> None:
        root = tree(self.root, [feature(pages=["street.html", "gone.html"])])
        self.assertOneProblem(root, "gone.html")


class Specs(Case):
    def test_a_spec_with_no_file_is_a_problem(self) -> None:
        root = tree(self.root, [feature(specs=["street", "gone"])])
        found = problems(root)
        self.assertTrue(any("gone" in p for p in found), found)

    def test_a_spec_file_in_no_tier_is_a_problem(self) -> None:
        root = tree(self.root, [feature(specs=["street", "loose"])], spec_files=("street", "loose"))
        self.assertOneProblem(root, "loose")


class Coverage(Case):
    def test_a_slice_no_feature_owns_or_reaches_is_named(self) -> None:
        self.assertOneProblem(tree(self.root, slices=("street", "timeline")), "timeline")

    def test_a_slice_is_reached_by_a_glob_inside_it(self) -> None:
        other = feature(id="timeline.milestones", owner="crates/cityloom_editor/src/street",
                        code=["crates/cityloom_editor/src/timeline/*.rs"], specs=[])
        self.assertEqual(problems(tree(self.root, [feature(), other], slices=("street", "timeline"))), [])

    def test_a_crate_no_feature_reaches_is_named(self) -> None:
        self.assertOneProblem(tree(self.root, crates=("cityloom_editor", "cityloom_server")), "cityloom_server")

    def test_a_page_in_no_feature_is_named(self) -> None:
        self.assertOneProblem(tree(self.root, pages=("street.html", "credits.html")), "credits.html")

    def test_a_tiered_spec_in_no_feature_is_named(self) -> None:
        self.assertOneProblem(tree(self.root, others={"integration": ["sync"]}), "sync")


class TheRepository(unittest.TestCase):
    """The list in the repository: this fails when a slice, crate, page or spec is added and no feature takes it."""

    def test_the_feature_list_is_whole(self) -> None:
        self.assertEqual(problems(ROOT), [])

    def test_it_stays_at_tens_of_features(self) -> None:
        count = len(json.loads((ROOT / FEATURES).read_text(encoding="utf-8"))["features"])
        self.assertTrue(20 <= count <= 99, count)


if __name__ == "__main__":
    unittest.main()

"""Keeps the Architecture section in step with the code: `just test-scripts`, and the Documentation workflow.

Three tables of the documentation site list what the code has, in English and in fr-CA, and each must name all of it:

- the overview's slice table (`architecture/index.md`) names every folder of `crates/cityloom_editor/src/`;
- the browser data page's key table (`architecture/data-model-projects.md`) names every `cityloom-*` key the editor
  reads or writes, outside its tests;
- the database page's column table (`architecture/data-model-server.md`) names every table a server migration makes.

Only the tables are read, never the prose: a name counts when it is a code span in the table. Each check takes the
root of a tree, so the tests below run it on the repository and on small fixture trees that leave something out.
"""

from __future__ import annotations

import re
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

EDITOR_SRC = Path("crates/cityloom_editor/src")
MIGRATIONS = Path("crates/cityloom_server/migrations")
PAGES = {
    "en": Path("website/docs/architecture"),
    "fr-CA": Path("website/i18n/fr-CA/docusaurus-plugin-content-docs/current/architecture"),
}
SLICE_PAGE = "index.md"
KEY_PAGE = "data-model-projects.md"
TABLE_PAGE = "data-model-server.md"

# The first header cell of each table, in each language.
SLICE_HEADERS = {"Slice", "Tranche"}
KEY_HEADERS = {"Key", "Clé"}
TABLE_HEADERS = {"Table"}

# `cityloom-` literals of the editor's code that are not storage keys, each with its reason. One that no longer
# appears in the code fails the tests, so this list cannot keep a dead entry.
NOT_KEYS = {
    "cityloom-": "the start of a design archive's file name (archive/model.rs `file_name`), not a key",
    "cityloom-design": "the `kind` of a design archive file (archive/model.rs `MAGIC`), a format tag",
    "cityloom-share": "the `kind` of a share document (share/model.rs `MAGIC`), a format tag",
}


CODE_SPAN = re.compile(r"`([^`]+)`")
# A key literal stops before a suffix (`"cityloom-network:45.5,-73.6"`) or a file extension.
KEY_LITERAL = re.compile(r'"(cityloom-[A-Za-z0-9_-]*)')
CREATE_TABLE = re.compile(r"\bcreate\s+table\s+(?:if\s+not\s+exists\s+)?([A-Za-z_][A-Za-z0-9_]*)", re.IGNORECASE)
CFG_TEST = "#[cfg(test)]"
RAW_STRING = re.compile(r'r(#*)"')
CHAR = re.compile(r"'(?:\\.|[^\\'])'")
TEST_MODULE = re.compile(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")


def table_cells(text: str, headers: set[str]) -> list[list[str]]:
    """The body rows of each Markdown table whose first header cell is one of `headers`, as lists of cells."""
    rows: list[list[str]] = []
    block: list[str] = []
    for line in text.splitlines() + [""]:
        if line.lstrip().startswith("|"):
            block.append(line.strip())
            continue
        if len(block) >= 2:
            cells = [[cell.strip() for cell in row.strip("|").split("|")] for row in block]
            if cells[0][0] in headers:
                rows += cells[2:]
        block = []
    return rows


def named(text: str, headers: set[str], *, first_column: bool) -> set[str]:
    """The code spans of the matching tables: of the first column only, or of every cell."""
    spans: set[str] = set()
    for row in table_cells(text, headers):
        for cell in row[:1] if first_column else row:
            spans.update(CODE_SPAN.findall(cell))
    return spans


def problems(root: Path, page: str, headers: set[str], wanted: set[str], what: str, *, first_column: bool,
             normalise=lambda span: span) -> list[str]:
    found = []
    for lang, folder in PAGES.items():
        path = folder / page
        file = root / path
        if not file.is_file():
            found.append(f"{path} ({lang}) is missing")
            continue
        listed = {normalise(span) for span in named(file.read_text(encoding="utf-8"), headers, first_column=first_column)}
        found += [f"{path} ({lang}): the {what} `{name}` is not in the table" for name in sorted(wanted - listed)]
    return found


def missing_slices(root: Path) -> list[str]:
    """Each folder of the editor's src/ that a language's slice table does not name."""
    slices = {entry.name + "/" for entry in (root / EDITOR_SRC).iterdir() if entry.is_dir()}
    return problems(root, SLICE_PAGE, SLICE_HEADERS, slices, "slice", first_column=True,
                    normalise=lambda span: span.rstrip("/") + "/")


def skip_item(text: str, start: int) -> int:
    """The index just past the item that starts at `start`: up to its `;`, or its `{…}` block, whichever comes first.

    Strings, characters and comments are skipped, so a brace inside one does not count.
    """
    depth = 0
    i = start
    while i < len(text):
        c = text[i]
        if text.startswith("//", i):
            i = text.find("\n", i)
            i = len(text) if i < 0 else i
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = len(text) if end < 0 else end + 2
            continue
        raw = RAW_STRING.match(text, i)
        if raw and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            end = text.find('"' + raw.group(1), raw.end())
            i = len(text) if end < 0 else end + 1 + len(raw.group(1))
            continue
        if c == '"':
            i += 1
            while i < len(text) and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
            i += 1
            continue
        char = CHAR.match(text, i)
        if char:
            i = char.end()
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        elif c == ";" and depth == 0:
            return i + 1
        i += 1
    return len(text)


def without_tests(text: str) -> str:
    """The text with every item under `#[cfg(test)]` taken out."""
    while (at := text.find(CFG_TEST)) >= 0:
        text = text[:at] + text[skip_item(text, at + len(CFG_TEST)):]
    return text


def test_only_files(src: Path) -> set[Path]:
    """The files of modules declared under `#[cfg(test)]` (`mod tests;`), and every `tests.rs`."""
    files = {path for path in src.rglob("tests.rs")}
    for path in src.rglob("*.rs"):
        base = path.parent if path.name in ("lib.rs", "mod.rs", "main.rs") else path.parent / path.stem
        for name in TEST_MODULE.findall(path.read_text(encoding="utf-8")):
            files.add(base / f"{name}.rs")
            files.update((base / name).rglob("*.rs"))
    return files


def key_literals(root: Path) -> set[str]:
    """Every `cityloom-*` literal of the editor's non-test code, keys or not."""
    src = root / EDITOR_SRC
    skipped = test_only_files(src)
    literals: set[str] = set()
    for path in src.rglob("*.rs"):
        if path not in skipped:
            literals.update(KEY_LITERAL.findall(without_tests(path.read_text(encoding="utf-8"))))
    return literals


def missing_keys(root: Path) -> list[str]:
    """Each `cityloom-*` key of the editor's non-test code that a language's key table does not name."""
    keys = key_literals(root) - set(NOT_KEYS)
    return problems(root, KEY_PAGE, KEY_HEADERS, keys, "key", first_column=False,
                    normalise=lambda span: span.split(":", 1)[0])


def missing_tables(root: Path) -> list[str]:
    """Each table a migration makes that a language's column table does not name."""
    tables = set()
    for path in sorted((root / MIGRATIONS).glob("*.sql")):
        tables.update(name.lower() for name in CREATE_TABLE.findall(path.read_text(encoding="utf-8")))
    return problems(root, TABLE_PAGE, TABLE_HEADERS, tables, "table", first_column=True)


# Fixture trees.


def page_table(header: list[str], rows: list[list[str]]) -> str:
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join(" --- " for _ in header) + "|"]
    lines += ["| " + " | ".join(row) + " |" for row in rows]
    return "\n".join(lines) + "\n"


def write(root: Path, path: Path | str, text: str) -> None:
    file = root / path
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(text, encoding="utf-8")


def tree(
    root: Path,
    *,
    slices: tuple[str, ...] = ("city", "shell"),
    code: dict[str, str] | None = None,
    migrations: dict[str, str] | None = None,
    documented_slices: dict[str, tuple[str, ...]] | None = None,
    documented_keys: dict[str, tuple[str, ...]] | None = None,
    documented_tables: dict[str, tuple[str, ...]] | None = None,
) -> Path:
    """A small tree: the editor's slices and code, the migrations, and the three pages in both languages.

    By default every page names everything the code has; a test changes one thing to see the check fail.
    """
    code = code if code is not None else {"city/store.rs": 'pub const KEY: &str = "cityloom-edits";\n'}
    migrations = migrations if migrations is not None else {"0001_accounts.sql": "create table users (\n  id uuid\n);\n"}
    for name in slices:
        (root / EDITOR_SRC / name).mkdir(parents=True, exist_ok=True)
    write(root, EDITOR_SRC / "lib.rs", "")
    for path, text in code.items():
        write(root, EDITOR_SRC / path, text)
    for path, text in migrations.items():
        write(root, MIGRATIONS / path, text)
    for lang, folder in PAGES.items():
        slice_rows = (documented_slices or {}).get(lang, slices)
        key_rows = (documented_keys or {}).get(lang, ("cityloom-edits",))
        table_rows = (documented_tables or {}).get(lang, ("users",))
        slice_header = "Slice" if lang == "en" else "Tranche"
        key_header = "Key" if lang == "en" else "Clé"
        write(root, folder / SLICE_PAGE, "# Architecture\n\nProse may name `ghost/`.\n\n"
              + page_table([slice_header, "What"], [[f"`{name}/`", "…"] for name in slice_rows]))
        write(root, folder / KEY_PAGE, "# Data\n\n"
              + page_table([key_header, "What"], [[f"`{key}`", "…"] for key in key_rows]))
        write(root, folder / TABLE_PAGE, "# Server\n\n"
              + page_table(["Table", "Column"], [[f"`{table}`", "`id`"] for table in table_rows]))
    return root


class FixtureCase(unittest.TestCase):
    def setUp(self) -> None:
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)

    def tearDown(self) -> None:
        self._dir.cleanup()


class SliceTable(FixtureCase):
    def test_a_tree_whose_pages_name_every_slice_passes(self) -> None:
        self.assertEqual(missing_slices(tree(self.root)), [])

    def test_a_slice_left_out_of_one_language_is_named_with_that_page(self) -> None:
        root = tree(self.root, slices=("city", "shell", "timeline"),
                    documented_slices={"fr-CA": ("city", "shell")})
        problems = missing_slices(root)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("timeline/", problems[0])
        self.assertIn(str(PAGES["fr-CA"] / SLICE_PAGE), problems[0])

    def test_a_slice_named_only_in_the_prose_does_not_count(self) -> None:
        root = tree(self.root, slices=("city", "ghost"), documented_slices={"en": ("city",), "fr-CA": ("city", "ghost")})
        problems = missing_slices(root)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("ghost/", problems[0])

    def test_a_file_of_src_is_not_a_slice(self) -> None:
        root = tree(self.root, code={"lib.rs": "", "i18n_guard.rs": "", "city/mod.rs": ""})
        self.assertEqual(missing_slices(root), [])


class KeyTable(FixtureCase):
    def test_a_tree_whose_pages_name_every_key_passes(self) -> None:
        self.assertEqual(missing_keys(tree(self.root)), [])

    def test_a_key_left_out_of_one_language_is_named_with_that_page(self) -> None:
        code = {"city/store.rs": 'const A: &str = "cityloom-edits";\nconst B: &str = "cityloom-theme";\n'}
        root = tree(self.root, code=code, documented_keys={"en": ("cityloom-edits", "cityloom-theme")})
        problems = missing_keys(root)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("cityloom-theme", problems[0])
        self.assertIn(str(PAGES["fr-CA"] / KEY_PAGE), problems[0])

    def test_a_key_with_a_suffix_is_named_by_its_row_with_a_placeholder(self) -> None:
        code = {"city/store.rs": 'pub const NETWORK_KEY: &str = "cityloom-network";\n'}
        root = tree(self.root, code=code, documented_keys={
            "en": ("cityloom-network:<area key>",), "fr-CA": ("cityloom-network:<clé du lieu>",)})
        self.assertEqual(missing_keys(root), [])

    def test_a_damaged_copy_named_in_any_cell_of_the_table_counts(self) -> None:
        code = {"city/edits.rs": 'const A: &str = "cityloom-edits";\nconst D: &str = "cityloom-edits-damaged";\n'}
        root = tree(self.root, code=code)
        for folder in PAGES.values():
            write(root, folder / KEY_PAGE, page_table(["Key", "Damaged copy"],
                                                      [["`cityloom-edits`", "`cityloom-edits-damaged`, copied first"]]))
        self.assertEqual(missing_keys(root), [])

    def test_keys_of_the_tests_are_not_counted(self) -> None:
        code = {
            "city/store.rs": 'const A: &str = "cityloom-edits";\n'
                             '#[cfg(test)]\nmod tests {\n    const T: &str = "cityloom-test-key";\n'
                             '    fn f() { let s = "}"; let c = \'"\'; }\n}\n'
                             'pub fn after() -> &\'static str { "cityloom-edits" }\n'
                             '#[cfg(test)]\nconst ALSO: &str = "cityloom-test-nothing";\n',
            "city/mod.rs": "#[cfg(test)]\nmod fixtures;\n",
            "city/fixtures.rs": 'const F: &str = "cityloom-fixture-only";\n',
            "lib.rs": "#[cfg(test)]\nmod layout_guard;\n",
            "layout_guard.rs": 'const G: &str = "cityloom-layout-guard-";\n',
            "shell/tests.rs": 'const S: &str = "cityloom-in-tests-rs";\n',
        }
        self.assertEqual(missing_keys(tree(self.root, code=code)), [])

    def test_code_after_a_test_block_is_still_read(self) -> None:
        code = {"city/store.rs": '#[cfg(test)]\nmod tests {\n    fn f() {}\n}\npub const K: &str = "cityloom-region";\n'}
        problems = missing_keys(tree(self.root, code=code))
        self.assertEqual(len(problems), 2, problems)
        self.assertTrue(all("cityloom-region" in problem for problem in problems))

    def test_a_literal_that_is_not_a_key_is_not_counted(self) -> None:
        code = {"archive/model.rs": 'pub const MAGIC: &str = "cityloom-design";\nfn n() -> String { format!("cityloom-{}", 1) }\n',
                "city/store.rs": 'const A: &str = "cityloom-edits";\n'}
        self.assertEqual(missing_keys(tree(self.root, code=code)), [])


class DatabaseTable(FixtureCase):
    def test_a_tree_whose_pages_name_every_table_passes(self) -> None:
        self.assertEqual(missing_tables(tree(self.root)), [])

    def test_a_table_of_a_later_migration_left_out_is_named_with_each_page(self) -> None:
        migrations = {"0001_accounts.sql": "create table users (id uuid);\n",
                      "0005_shares.sql": "-- the shares\nCREATE TABLE shares (\n  id uuid\n);\n"}
        problems = missing_tables(tree(self.root, migrations=migrations))
        self.assertEqual(len(problems), 2, problems)
        self.assertTrue(all("shares" in problem for problem in problems))
        self.assertTrue(any(str(PAGES["en"] / TABLE_PAGE) in problem for problem in problems))

    def test_an_altered_table_is_not_a_new_one(self) -> None:
        migrations = {"0001_accounts.sql": "create table users (id uuid);\n",
                      "0002_work.sql": "alter table users add column work_rev bigint;\n"}
        self.assertEqual(missing_tables(tree(self.root, migrations=migrations)), [])


class TheRepository(unittest.TestCase):
    """The checks on the repository itself: these fail when the code gains what the section does not name."""

    def test_the_overview_names_every_slice_in_both_languages(self) -> None:
        self.assertEqual(missing_slices(ROOT), [])

    def test_the_browser_data_page_names_every_key_in_both_languages(self) -> None:
        self.assertEqual(missing_keys(ROOT), [])

    def test_the_database_page_names_every_table_in_both_languages(self) -> None:
        self.assertEqual(missing_tables(ROOT), [])

    def test_every_literal_said_not_to_be_a_key_is_still_in_the_code(self) -> None:
        literals = key_literals(ROOT)
        self.assertEqual(sorted(set(NOT_KEYS) - literals), [], "remove these entries from NOT_KEYS")

    def test_the_checks_find_something_to_check(self) -> None:
        self.assertGreater(len(key_literals(ROOT)), 10)


if __name__ == "__main__":
    unittest.main()

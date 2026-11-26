"""What the documentation says of preparing a city (CLM-75): `just test-scripts`.

`just prepare` takes the city it prepares, and each city is one file, `cities/<id>.toml`. The README, the documentation
site in both languages, the notes for agents and the recipes' comments must give the command with its city, say how a
city is added and name every key of its file. Standard library only.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EN = "website/docs/"
FR = "website/i18n/fr-CA/docusaurus-plugin-content-docs/current/"
BOTH = (EN, FR)
BUILDER = "crates/citymodel_builder/CLAUDE.md"
PLACE = "crates/cityloom_editor/src/place/CLAUDE.md"
# Every key of a city's file, the sections among them.
KEYS = ("id", "extract_url", "grid", "west", "south", "tile_width", "tile_height", "columns", "rows", "margin", "lon", "lat")
EVERY_FILE = (
    "README.md",
    "CLAUDE.md",
    "justfile",
    ".env.example",
    BUILDER,
    PLACE,
    *(d + page for d in BOTH for page in ("getting-started.md", "project-layout.md", "testing/levels.md", "architecture/runtime.md")),
)


def read(path: str) -> str:
    """A file's text; an escaped `<id>` (a mermaid label's) reads as a plain one."""
    return (ROOT / path).read_text(encoding="utf-8").replace("&lt;id&gt;", "<id>")


def commands(text: str, recipe: str) -> list[str]:
    """What follows `just <recipe>` on each line of the text's `sh` blocks that runs it, up to a comment."""
    found = []
    for block in re.findall(r"(?ms)^```sh\b[^\n]*\n(.*?)^```", text):
        for line in block.splitlines():
            words = line.split("#", 1)[0].split()
            if words[:2] == ["just", recipe]:
                found.append(" ".join(words[2:]))
    return found


def section(text: str, heading: str) -> str | None:
    """The text under a Markdown heading, up to the next one of the same or a higher level; None without the heading."""
    lines, level, inside = [], 0, False
    fenced = False
    for line in text.splitlines():
        fenced ^= line.startswith("```")
        title = None if fenced else re.fullmatch(r"(#+)\s+(.+?)\s*(?:\{#[\w-]+\})?\s*", line)
        if inside and title and len(title.group(1)) <= level:
            break
        if inside:
            lines.append(line)
        elif title and title.group(2) == heading:
            inside, level = True, len(title.group(1))
    return "\n".join(lines) if inside else None


def names_key(text: str, key: str) -> bool:
    """Whether a code span of the text is the key, a section in its brackets, or a key with its section's prefix."""
    spans = re.findall(r"`([^`\n]+)`", text)
    return any(span == key or span == f"[{key}]" or re.fullmatch(rf"(?:grid|margin)\.{key}", span) for span in spans)


def comment_above(text: str, recipe: str) -> str:
    """The comment lines just above a recipe of the justfile."""
    lines = text.splitlines()
    at = next(i for i, line in enumerate(lines) if line.startswith(recipe))
    first = at
    while first > 0 and lines[first - 1].startswith("#"):
        first -= 1
    return "\n".join(lines[first:at])


def paragraph_with(text: str, word: str) -> str:
    return next((p for p in re.split(r"\n\s*\n", text) if word in p), "")


class CityDocs(unittest.TestCase):
    def says(self, text: str, words: str) -> None:
        self.assertTrue(words in text, f"{words!r} is not said")

    def says_no(self, text: str, words: str) -> None:
        self.assertFalse(words in text, f"{words!r} is said")

    def test_the_command_is_shown_with_its_city(self):
        for path in ("README.md", "CLAUDE.md", EN + "getting-started.md", FR + "getting-started.md"):
            with self.subTest(path):
                prepares = commands(read(path), "prepare")
                self.assertTrue(prepares, "no sh block runs just prepare")
                for rest in prepares + commands(read(path), "pack"):
                    self.assertRegex(rest, r"^(montreal|<city>)( |$)", "the command without its city")

    def test_montreals_command_is_named(self):
        pages = [d + page for d in BOTH for page in ("getting-started.md", "architecture/runtime.md")]
        for path in ("README.md", "justfile", "CLAUDE.md", BUILDER, *pages):
            with self.subTest(path):
                self.says(read(path), "just prepare montreal")
        for path in ("README.md", "justfile", ".env.example"):
            with self.subTest(path):
                self.says(read(path), "just pack montreal")

    def test_no_page_runs_prepare_without_a_city(self):
        for path in EVERY_FILE:
            with self.subTest(path):
                self.says_no(read(path), "just prepare --")
        with self.subTest(".env.example"):
            self.says_no(read(".env.example"), "`just pack` makes")

    def test_the_recipe_comments_say_the_city_and_the_options(self):
        just = read("justfile")
        prepare, pack = comment_above(just, "prepare *args:"), comment_above(just, "pack *args:")
        for word in ("cities/", "--refresh", "--extract"):
            with self.subTest(word):
                self.says(prepare, word)
        with self.subTest("pack"):
            self.says(pack, "just pack montreal")

    def test_add_a_city_names_the_file_and_every_key(self):
        for path, heading in (
            ("README.md", "Add a city"),
            (EN + "getting-started.md", "Add a city"),
            (FR + "getting-started.md", "Ajouter une ville"),
        ):
            with self.subTest(path):
                text = section(read(path), heading)
                self.assertIsNotNone(text, f"no heading {heading}")
                self.says(text, "cities/<id>.toml")
                self.assertEqual([k for k in KEYS if not names_key(text, k)], [], "keys not named")

    def test_the_extract_option_is_described(self):
        for path in ("README.md", BUILDER, EN + "getting-started.md", FR + "getting-started.md"):
            with self.subTest(path):
                self.says(read(path), "--extract")

    def test_the_layout_pages_name_the_cities_folder(self):
        for d in BOTH:
            with self.subTest(d):
                # the first table is the top-level folders'
                table = paragraph_with(read(d + "project-layout.md"), "| `crates/`")
                first_cells = [line.split("|")[1].strip() for line in table.splitlines() if line.startswith("|")]
                self.assertIn("`crates/`", first_cells)
                self.assertIn("`cities/`", first_cells)
        with self.subTest("CLAUDE.md"):
            layout = next((line for line in read("CLAUDE.md").splitlines() if line.startswith("Layout:")), "")
            self.says(layout, "cities/")

    def test_the_runtime_pages_give_each_city_its_own_files(self):
        for d in BOTH:
            with self.subTest(d):
                text = read(d + "architecture/runtime.md")
                self.says(text, "data/metro-work/<id>")
                self.assertEqual(re.findall(r"data/metro-work(?!/<id>).{0,20}", text), [], "the work folder of no city")
                self.says(text, "cities/")

    def test_the_testing_pages_list_the_city_tests(self):
        for d in BOTH:
            with self.subTest(d):
                self.says(paragraph_with(read(d + "testing/levels.md"), "`ctrl_c`"), "`city`")

    def test_the_builder_notes_describe_the_config(self):
        with self.subTest(BUILDER):
            notes = read(BUILDER)
            self.says(notes, "cities/<id>.toml")
            cache = next((line for line in notes.splitlines() if "caches `web/data/metro`" in line), "")
            self.says(cache, "cities/montreal.toml")
        with self.subTest(PLACE):
            self.says_no(read(PLACE), "makes Montréal's grid only")


if __name__ == "__main__":
    unittest.main()

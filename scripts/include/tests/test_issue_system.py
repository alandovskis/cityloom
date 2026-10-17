"""Keeps the issue system consistent: `just test-scripts`.

The labels (.github/labels.yml), the issue forms (.github/ISSUE_TEMPLATE) and the form-to-label rules
(.github/issue-labeler.yml) are three files that must agree. The repository checks them here, and the checks
themselves are tried against deliberately wrong fixtures.

No YAML library is installed in CI, so `read_yaml` is a small strict reader for the shapes these files use: block
mappings and sequences, and scalars that are double-quoted, single-quoted or plain. It refuses anything else
(block scalars, flow collections, anchors, trailing comments, tabs), so a file that needs more fails loudly here.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
LABELS = ROOT / ".github" / "labels.yml"
RULES = ROOT / ".github" / "issue-labeler.yml"
FORMS = ROOT / ".github" / "ISSUE_TEMPLATE"

NBSP = " "
FAMILIES = {"A", "C", "O", "P", "S"}
# The labels that stay as they are: Dependabot's, and GitHub's resolution labels. English only, and not renamed.
KEPT = {
    "dependencies",
    "rust",
    "javascript",
    "github_actions",
    "duplicate",
    "invalid",
    "wontfix",
}
STANDALONE = {"needs-triage", "regression-untriaged"}
# Options that read the same in both languages, so the form writes them once.
SAME_IN_BOTH = {"Chrome", "Edge", "Firefox", "Safari (Mac)", "Chrome (Android)", "Documentation"}

# What the form's answers must label, as the contract the labeler rules are held to. The key is the option's text.
PAGE_LABELS: dict[str, str | None] = {
    "Home (finding a place) / Accueil (chercher un lieu)": "A-places · lieux",
    "Map / Carte": "A-map · carte",
    "Area rules (on the map) / Règles de zone (sur la carte)": "A-area-rules · règles de zone",
    "Street editor / Éditeur de rues": "A-street · rue",
    "Street in 2.5D / Rue en 2,5D": "A-2.5d · 2,5d",
    "Junction editor / Éditeur de jonctions": "A-junction · jonction",
    "Account / Compte": "A-accounts · comptes",
    "Documentation": "A-docs · documentation",
    "Not sure / Je ne sais pas": None,
}
BROWSER_LABELS: dict[str, str | None] = {
    "Chrome": "O-chrome · chrome",
    "Edge": "O-edge · edge",
    "Firefox": "O-firefox · firefox",
    "Safari (Mac)": "O-safari · safari",
    "Safari (iPhone or iPad) / Safari (iPhone ou iPad)": "O-ios · iPhone et iPad",
    "Chrome (Android)": "O-android · android",
    "Other or not sure / Autre ou incertain": None,
}
NEUTRAL = {"page": "Not sure / Je ne sais pas", "browser": "Other or not sure / Autre ou incertain"}


# A strict reader for the subset of YAML these files use.


class YamlError(Exception):
    pass


def _quoted(text: str, where: str) -> tuple[str, int]:
    """The value of the quoted scalar that starts `text`, and the index just after its closing quote."""
    quote = text[0]
    out: list[str] = []
    i = 1
    while i < len(text):
        c = text[i]
        if quote == '"' and c == "\\":
            if i + 1 >= len(text) or text[i + 1] not in '"\\':
                raise YamlError(f"{where}: only \\\" and \\\\ are read as escapes")
            out.append(text[i + 1])
            i += 2
        elif c == quote:
            if quote == "'" and text[i + 1 : i + 2] == "'":
                out.append("'")
                i += 2
            else:
                return "".join(out), i + 1
        else:
            out.append(c)
            i += 1
    raise YamlError(f"{where}: a quote is not closed")


def _scalar(text: str, where: str) -> Any:
    text = text.strip()
    if text[0] in "\"'":
        value, end = _quoted(text, where)
        if text[end:].strip():
            raise YamlError(f"{where}: text after the closing quote")
        return value
    if text[0] in "[{|>&*!%@`#" or " #" in text or ": " in text or text.endswith(":"):
        raise YamlError(f"{where}: a plain scalar this reader does not take: {text!r}")
    return {"true": True, "false": False}.get(text, text)


def _split_key(content: str, where: str) -> tuple[str, str] | None:
    """(key, rest of the line) if `content` is `key: rest` or `key:`, else None (it is a scalar)."""
    if content[0] in "\"'":
        key, end = _quoted(content, where)
        rest = content[end:]
        if not rest.startswith(":"):
            return None
        if rest[1:] and not rest[1:].startswith(" "):
            raise YamlError(f"{where}: a colon after a quoted key needs a space")
        return key, rest[1:].strip()
    if content.endswith(":"):
        return content[:-1], ""
    at = content.find(": ")
    if at < 0:
        return None
    return content[:at], content[at + 2 :].strip()


Line = tuple[int, str, int]  # indent, content, line number


def _lines(text: str) -> list[Line]:
    out: list[Line] = []
    for number, raw in enumerate(text.splitlines(), 1):
        if "\t" in raw[: len(raw) - len(raw.lstrip())]:
            raise YamlError(f"line {number}: a tab in the indent")
        content = raw.strip()
        if not content or content.startswith("#"):
            continue
        out.append((len(raw) - len(raw.lstrip(" ")), content, number))
    return out


def _block(lines: list[Line], i: int, indent: int) -> tuple[Any, int]:
    if lines[i][1] == "-" or lines[i][1].startswith("- "):
        return _sequence(lines, i, indent)
    return _mapping(lines, i, indent)


def _sequence(lines: list[Line], i: int, indent: int) -> tuple[list, int]:
    items: list[Any] = []
    while i < len(lines) and lines[i][0] == indent and (lines[i][1] == "-" or lines[i][1].startswith("- ")):
        _, content, number = lines[i]
        where = f"line {number}"
        rest = content[1:].lstrip()
        if not rest:
            if i + 1 >= len(lines) or lines[i + 1][0] <= indent:
                raise YamlError(f"{where}: an empty item")
            value, i = _block(lines, i + 1, lines[i + 1][0])
            items.append(value)
        elif _split_key(rest, where) is not None:
            inner = indent + (len(content) - len(rest))
            lines[i] = (inner, rest, number)
            value, i = _mapping(lines, i, inner)
            items.append(value)
        else:
            items.append(_scalar(rest, where))
            i += 1
    return items, i


def _mapping(lines: list[Line], i: int, indent: int) -> tuple[dict, int]:
    out: dict[str, Any] = {}
    while i < len(lines) and lines[i][0] == indent and not (lines[i][1] == "-" or lines[i][1].startswith("- ")):
        _, content, number = lines[i]
        where = f"line {number}"
        split = _split_key(content, where)
        if split is None:
            raise YamlError(f"{where}: expected `key: value`")
        key, rest = split
        if key in out:
            raise YamlError(f"{where}: the key {key!r} is repeated")
        if rest:
            out[key] = _scalar(rest, where)
            i += 1
        elif i + 1 < len(lines) and lines[i + 1][0] > indent:
            out[key], i = _block(lines, i + 1, lines[i + 1][0])
        elif i + 1 < len(lines) and lines[i + 1][0] == indent and lines[i + 1][1].startswith("- "):
            out[key], i = _sequence(lines, i + 1, indent)
        else:
            raise YamlError(f"{where}: {key!r} has no value")
    return out, i


def read_yaml(text: str) -> Any:
    lines = _lines(text)
    if not lines:
        raise YamlError("empty document")
    value, end = _block(lines, 0, lines[0][0])
    if end != len(lines):
        raise YamlError(f"line {lines[end][2]}: unexpected indentation")
    return value


# The checks. Each returns a list of what is wrong, empty when all is well.


def check_labels(labels: list[dict]) -> list[str]:
    errors: list[str] = []
    names: set[str] = set()
    colours: dict[str, set[str]] = {}
    words: dict[str, set[tuple[str, str]]] = {}
    for label in labels:
        name = label.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"a label has no name: {label}")
            continue
        if name in names:
            errors.append(f"{name}: the name is used twice")
        names.add(name)
        extra = set(label) - {"name", "color", "description", "from_name"}
        if extra:
            errors.append(f"{name}: unknown keys {sorted(extra)}")
        if not re.fullmatch(r"[0-9a-f]{6}", str(label.get("color", ""))):
            errors.append(f"{name}: the colour must be six lowercase hex digits")
        if name in KEPT:
            if "from_name" in label:
                errors.append(f"{name}: a kept label is not renamed")
            continue
        if len(name) > 50:
            errors.append(f"{name}: {len(name)} characters, GitHub allows 50")
        if not re.fullmatch(r"[^ ·]+ · [^ ·].*", name):
            errors.append(f"{name}: a name is `<english> · <français>`")
        if "'" in name:
            errors.append(f"{name}: use the typographic apostrophe")
        family = name[0] if name[1:2] == "-" else None
        if family is not None:
            if family not in FAMILIES:
                errors.append(f"{name}: no family {family}-")
            colours.setdefault(family, set()).add(str(label.get("color")))
        elif name.split(" · ")[0] not in STANDALONE:
            errors.append(f"{name}: not in a family and not one of {sorted(STANDALONE)}")
        description = label.get("description")
        if not isinstance(description, str):
            errors.append(f"{name}: no description")
            continue
        if len(description) > 100:
            errors.append(f"{name}: the description has {len(description)} characters, GitHub allows 100")
        if "'" in description:
            errors.append(f"{name}: use the typographic apostrophe in the description")
        halves = description.split(" / ")
        if len(halves) != 2 or not re.fullmatch(r"[^:]+: .+\.", halves[0]):
            errors.append(f"{name}: the description is `<Family>: <sentence>. / <Famille>{NBSP}: <phrase>.`")
            continue
        french = re.fullmatch(rf"([^:]+?){NBSP}: .+\.", halves[1])
        if not french:
            errors.append(f"{name}: the French half is `<Famille>{NBSP}: <phrase>.` with a no-break space")
            continue
        words.setdefault(name[0] if family else name, set()).add((halves[0].split(":")[0], french.group(1)))
    for family, found in colours.items():
        if len(found) > 1:
            errors.append(f"family {family}-: has {len(found)} colours, {sorted(found)}")
    for family, found in words.items():
        if len(found) > 1:
            errors.append(f"family {family}: its descriptions start with different names, {sorted(found)}")
    for kept in sorted(KEPT):
        if kept not in names:
            errors.append(f"{kept}: a label to keep is missing, and the sync would delete it")
    return errors


def parse_rule(pattern: str) -> re.Pattern:
    """The pattern as the action reads it: `/regex/` (with no flags here) or a plain regex."""
    found = re.match(r"^/(.+)/(.*)$", pattern)
    if not found:
        raise YamlError(f"{pattern!r}: write the rule as /regex/, as the action reads it")
    if found.group(2):
        raise YamlError(f"{pattern!r}: no flags, so that JavaScript and Python read the rule alike")
    return re.compile(found.group(1))


def dropdown(form: dict, field_id: str) -> dict | None:
    for item in form.get("body", []):
        if item.get("type") == "dropdown" and item.get("id") == field_id:
            return item
    return None


def rendered(form: dict, answers: dict[str, str], nasty: str, newline: str) -> str:
    """The body GitHub makes of the form: `### <label>`, a blank line, the answer, a blank line."""
    parts: list[str] = []
    for item in form.get("body", []):
        if item.get("type") == "markdown":
            continue
        field_id = item.get("id")
        label = item["attributes"]["label"]
        parts.append(f"### {label}{newline}{newline}{answers.get(field_id, nasty)}")
    return (newline * 2).join(parts) + newline * 2


def check_forms(forms: dict[str, dict], names: set[str]) -> list[str]:
    errors: list[str] = []
    for file, form in forms.items():
        for key in ("name", "description", "title", "labels", "body"):
            if key not in form:
                errors.append(f"{file}: no `{key}`")
        if any(key not in form for key in ("name", "description", "title", "labels", "body")):
            continue
        for key in ("name", "description", "title"):
            if " / " not in form[key]:
                errors.append(f"{file}: `{key}` is not in both languages (`English / Français`)")
        labels = form["labels"]
        for label in labels:
            if label not in names:
                errors.append(f"{file}: the label {label!r} is not in labels.yml")
        if not labels or not labels[0].startswith("C-"):
            errors.append(f"{file}: the first label is the kind's C- label")
        if not any(label.startswith("needs-triage") for label in labels):
            errors.append(f"{file}: needs-triage is missing from labels")
        if len([label for label in labels if label[1:2] == "-"]) != 1:
            errors.append(f"{file}: a form applies exactly one family label itself, its C- label")
        for item in form["body"]:
            attributes = item.get("attributes", {})
            for key in ("label", "description", "placeholder", "value"):
                if key in attributes and " / " not in attributes[key] and item.get("type") != "markdown":
                    errors.append(f"{file}: `{attributes[key]}` ({key}) is not in both languages")
            if item.get("type") == "markdown" and " / " not in attributes.get("value", ""):
                errors.append(f"{file}: a markdown block is not in both languages")
            for option in attributes.get("options", []):
                if " / " not in option and option not in SAME_IN_BOTH:
                    errors.append(f"{file}: the option {option!r} is not in both languages")
        for field_id, table in (("page", PAGE_LABELS), ("browser", BROWSER_LABELS)):
            found = dropdown(form, field_id)
            if found is None:
                errors.append(f"{file}: no `{field_id}` dropdown")
                continue
            if not found.get("validations", {}).get("required"):
                errors.append(f"{file}: the `{field_id}` dropdown is not required")
            options = found["attributes"]["options"]
            if sorted(options) != sorted(table):
                errors.append(f"{file}: the `{field_id}` options are not the contracted ones: {options}")
    return errors


def check_rules(rules: dict[str, list[str]], names: set[str], forms: dict[str, dict]) -> list[str]:
    errors: list[str] = []
    compiled: dict[str, list[re.Pattern]] = {}
    for label, patterns in rules.items():
        if label not in names:
            errors.append(f"issue-labeler.yml: the rule names {label!r}, which is not in labels.yml")
        if label[:2] not in ("A-", "O-"):
            errors.append(f"issue-labeler.yml: {label!r}: only A- and O- labels come from the form")
        if not isinstance(patterns, list) or not patterns:
            errors.append(f"issue-labeler.yml: {label!r} needs a list of patterns")
            continue
        try:
            compiled[label] = [parse_rule(p) for p in patterns]
        except (YamlError, re.error) as error:
            errors.append(f"issue-labeler.yml: {label!r}: {error}")
    if errors:
        return errors
    every_option = "\n".join([*PAGE_LABELS, *BROWSER_LABELS])
    for file, form in forms.items():
        for field_id, table in (("page", PAGE_LABELS), ("browser", BROWSER_LABELS)):
            found = dropdown(form, field_id)
            if found is None:
                continue
            for option in found["attributes"]["options"]:
                if option not in table:
                    errors.append(f"{file}: the option {option!r} has no declared label")
                    continue
                answers = {**NEUTRAL, field_id: option}
                wanted = {table[option]} - {None}
                for newline in ("\n", "\r\n"):
                    # Every free-text answer holds all the options' texts, one to a line: they must not label.
                    body = rendered(form, answers, every_option.replace("\n", newline), newline)
                    got = {label for label, patterns in compiled.items() if all(p.search(body) for p in patterns)}
                    if got != wanted:
                        errors.append(
                            f"{file}: answering {option!r} labels {sorted(got)}, expected {sorted(wanted)}"
                            f" (line ends {newline!r})"
                        )
    return errors


def labels_in(path: Path) -> list[dict]:
    return read_yaml(path.read_text(encoding="utf-8"))


def forms_in(directory: Path) -> dict[str, dict]:
    return {
        path.name: read_yaml(path.read_text(encoding="utf-8"))
        for path in sorted(directory.glob("*.yml"))
        if path.name != "config.yml"
    }


class TheRepository(unittest.TestCase):
    """The files as they are in the repository."""

    def test_labels(self):
        self.assertEqual([], check_labels(labels_in(LABELS)))

    def test_forms(self):
        forms = forms_in(FORMS)
        self.assertEqual(5, len(forms), sorted(forms))
        names = {label["name"] for label in labels_in(LABELS)}
        self.assertEqual([], check_forms(forms, names))

    def test_the_labeler_rules_label_exactly_what_was_picked(self):
        names = {label["name"] for label in labels_in(LABELS)}
        rules = read_yaml(RULES.read_text(encoding="utf-8"))
        self.assertEqual([], check_rules(rules, names, forms_in(FORMS)))

    def test_every_page_and_browser_label_has_a_rule(self):
        rules = read_yaml(RULES.read_text(encoding="utf-8"))
        wanted = {label for label in [*PAGE_LABELS.values(), *BROWSER_LABELS.values()] if label}
        self.assertEqual(wanted, set(rules))

    def test_the_contact_links_name_both_languages(self):
        config = read_yaml((FORMS / "config.yml").read_text(encoding="utf-8"))
        self.assertIs(False, config["blank_issues_enabled"])
        urls = [link["url"] for link in config["contact_links"]]
        self.assertEqual(
            ["https://alandovskis.github.io/cityloom/", "https://alandovskis.github.io/cityloom/fr-CA/"], urls
        )


def label(name: str, colour: str, description: str, **more: str) -> dict:
    return {"name": name, "color": colour, "description": description, **more}


def kept() -> list[dict]:
    return [label(name, "ffffff", "kept") for name in sorted(KEPT)]


GOOD = label("C-bug · bogue", "f5f1fd", f"Category: broken. / Catégorie{NBSP}: brisé.")


class TheLabelChecks(unittest.TestCase):
    def errors(self, *labels: dict) -> str:
        return "\n".join(check_labels([*labels, *kept()]))

    def test_a_good_label_passes(self):
        self.assertEqual("", self.errors(GOOD))

    def test_a_label_without_french_fails(self):
        self.assertIn("is `<english>", self.errors({**GOOD, "name": "C-bug"}))
        self.assertIn("<Family>", self.errors({**GOOD, "description": "Category: broken."}))

    def test_a_french_half_with_an_ordinary_space_before_the_colon_fails(self):
        self.assertIn("no-break space", self.errors({**GOOD, "description": "Category: broken. / Catégorie : brisé."}))

    def test_a_long_name_fails(self):
        self.assertIn("characters, GitHub allows 50", self.errors({**GOOD, "name": "C-" + "x" * 40 + " · " + "y" * 8}))

    def test_a_long_description_fails(self):
        long = f"Category: {'x' * 60}. / Catégorie{NBSP}: {'y' * 30}."
        self.assertIn("allows 100", self.errors({**GOOD, "description": long}))

    def test_two_colours_in_a_family_fail(self):
        other = label("C-other · autre", "000000", GOOD["description"])
        self.assertIn("has 2 colours", self.errors(GOOD, other))

    def test_a_repeated_name_fails(self):
        self.assertIn("used twice", self.errors(GOOD, GOOD))

    def test_a_straight_apostrophe_fails(self):
        self.assertIn("typographic", self.errors({**GOOD, "description": f"Category: it's. / Catégorie{NBSP}: brisé."}))

    def test_a_family_whose_descriptions_disagree_fails(self):
        other = label("C-other · autre", "f5f1fd", f"Kind: x. / Genre{NBSP}: y.")
        self.assertIn("different names", self.errors(GOOD, other))

    def test_a_missing_kept_label_fails(self):
        self.assertIn("would delete it", "\n".join(check_labels([GOOD])))

    def test_an_unknown_family_fails(self):
        self.assertIn("no family Z-", self.errors({**GOOD, "name": "Z-bug · bogue"}))


FORM_BODY = [
    {
        "type": "dropdown",
        "id": "page",
        "attributes": {"label": "Which page? / Quelle page?", "options": list(PAGE_LABELS)},
        "validations": {"required": True},
    },
    {
        "type": "dropdown",
        "id": "browser",
        "attributes": {"label": "Browser / Navigateur", "options": list(BROWSER_LABELS)},
        "validations": {"required": True},
    },
    {"type": "textarea", "id": "what", "attributes": {"label": "What? / Quoi?"}},
]
FORM = {
    "name": "Bug / Bogue",
    "description": "x / y",
    "title": "[Bug / Bogue] ",
    "labels": ["C-bug · bogue", "needs-triage · à trier"],
    "body": FORM_BODY,
}


def exact(heading: str, answer: str) -> str:
    return "/(?:^|\\n)### " + re.escape(heading) + r"\r?\n\r?\n" + re.escape(answer) + r"(?:\r?\n|$)/"


def good_rules() -> dict[str, list[str]]:
    rules: dict[str, list[str]] = {}
    for table, field in ((PAGE_LABELS, FORM_BODY[0]), (BROWSER_LABELS, FORM_BODY[1])):
        for option, name in table.items():
            if name:
                rules[name] = [exact(field["attributes"]["label"], option)]
    return rules


class TheFormAndRuleChecks(unittest.TestCase):
    NAMES = {"C-bug · bogue", "needs-triage · à trier", *good_rules()}

    def rule_errors(self, rules: dict) -> str:
        return "\n".join(check_rules(rules, self.NAMES, {"f.yml": FORM}))

    def test_good_rules_pass(self):
        self.assertEqual("", self.rule_errors(good_rules()))

    def test_a_rule_that_matches_anywhere_in_the_text_fails(self):
        rules = good_rules()
        rules["A-map · carte"] = ["/Map \\/ Carte/"]
        self.assertIn("labels ['A-map · carte'], expected []", self.rule_errors(rules))

    def test_a_rule_that_matches_a_longer_answer_fails(self):
        rules = good_rules()
        rules["O-chrome · chrome"] = [exact("Browser / Navigateur", "Chrome").replace("(?:\\r?\\n|$)", "")]
        self.assertIn("Chrome (Android)", self.rule_errors(rules))

    def test_a_rule_that_misses_its_own_answer_fails(self):
        rules = good_rules()
        rules["A-map · carte"] = [exact("Which page? / Quelle page?", "Map / Cartes")]
        self.assertIn("expected ['A-map · carte']", self.rule_errors(rules))

    def test_a_rule_that_misses_windows_line_ends_fails(self):
        rules = good_rules()
        rules["A-map · carte"] = ["/### Which page\\? \\/ Quelle page\\?\\n\\nMap \\/ Carte\\n/"]
        self.assertIn("'\\r\\n'", self.rule_errors(rules))

    def test_a_missing_rule_fails(self):
        rules = good_rules()
        del rules["A-street · rue"]
        self.assertIn("expected ['A-street · rue']", self.rule_errors(rules))

    def test_a_rule_for_a_label_not_in_labels_fails(self):
        rules = good_rules()
        rules["A-nowhere · nulle part"] = ["/x/"]
        self.assertIn("not in labels.yml", self.rule_errors(rules))

    def test_a_rule_for_a_priority_fails(self):
        rules = {**good_rules(), "P-high · élevée": ["/x/"]}
        self.assertIn("only A- and O-", "\n".join(check_rules(rules, self.NAMES | {"P-high · élevée"}, {"f.yml": FORM})))

    def test_a_rule_with_flags_fails(self):
        rules = {**good_rules(), "A-map · carte": ["/Map/i"]}
        self.assertIn("no flags", self.rule_errors(rules))

    def test_an_option_with_no_declared_label_fails(self):
        form = {**FORM, "body": [{**FORM_BODY[0], "attributes": {**FORM_BODY[0]["attributes"], "options": ["Moon / Lune"]}}, *FORM_BODY[1:]]}
        self.assertIn("no declared label", "\n".join(check_rules(good_rules(), self.NAMES, {"f.yml": form})))

    def form_errors(self, form: dict) -> str:
        return "\n".join(check_forms({"f.yml": form}, self.NAMES))

    def test_a_good_form_passes(self):
        self.assertEqual("", self.form_errors(FORM))

    def test_a_form_label_not_in_labels_fails(self):
        self.assertIn("not in labels.yml", self.form_errors({**FORM, "labels": ["C-nope · non", "needs-triage · à trier"]}))

    def test_a_form_without_needs_triage_fails(self):
        self.assertIn("needs-triage is missing", self.form_errors({**FORM, "labels": ["C-bug · bogue"]}))

    def test_a_form_field_in_one_language_fails(self):
        body = [*FORM_BODY[:2], {"type": "textarea", "id": "what", "attributes": {"label": "What?"}}]
        self.assertIn("not in both languages", self.form_errors({**FORM, "body": body}))

    def test_an_optional_page_dropdown_fails(self):
        body = [{**FORM_BODY[0], "validations": {}}, *FORM_BODY[1:]]
        self.assertIn("is not required", self.form_errors({**FORM, "body": body}))


class TheYamlReader(unittest.TestCase):
    def test_reads_the_shapes_the_files_use(self):
        text = (
            "# comment\n"
            "name: \"A \\\"quoted\\\" · text\"\n"
            "items:\n"
            "  - plain\n"
            "  - 'it''s'\n"
            "  - id: x\n"
            "    attributes:\n"
            "      options:\n"
            "        - \"a: b\"\n"
            "flag: true\n"
            "same:\n"
            "- one\n"
        )
        self.assertEqual(
            {
                "name": 'A "quoted" · text',
                "items": ["plain", "it's", {"id": "x", "attributes": {"options": ["a: b"]}}],
                "flag": True,
                "same": ["one"],
            },
            read_yaml(text),
        )

    def test_refuses_what_it_does_not_read(self):
        for text in ("a: |\n  x\n", "a: [1, 2]\n", "a: b # note\n", "a: 1\na: 2\n", "\ta: 1\n", "a: 'open\n"):
            with self.assertRaises(YamlError, msg=text):
                read_yaml(text)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""The faults of the software factory's tests flow (CLM-100).

An issue that asks only for tests of working code cannot show its tests failing first, so the tests gate
(`gate.py tests`) proves each new test another way: it puts a fault into the code the plan names, runs the new tests,
and takes the fault back. This module makes those faults from the plan's `## Faults` block:

- **rust**: the faults `cargo-mutants` lists for a named function (`cargo mutants --list --json`), each the span it
  gives replaced by its replacement. Its diffs are not patches, so they are not used.
- **markup**: in the `view!` of a named function, one attribute removed, one string literal changed, or two
  neighbouring elements of one parent swapped. Read with a tokenizer (`code_mask`), not by parsing Rust: a fault it
  misreads does not compile, and the gate drops it.
- **ftl**: a Fluent message removed, one of its placeables removed, or one of its variables renamed.
- **css**: a declaration's value set to the value the plan gives (no value fails every contrast, so the planner says).

A fault is one span of one file replaced, applied with `applied`, which puts the file back byte for byte whatever
happens. Standard library only; scripts/include/tests/test_factory_faults.py tests it.
"""

from __future__ import annotations

import hashlib
import json
import re
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, Iterator, List, Optional, Tuple

KINDS = ("rust", "markup", "ftl", "css")


class SpecError(Exception):
    """A plan's faults name something that is not there, or yield no fault."""


@dataclass(frozen=True)
class Fault:
    kind: str
    file: str
    entry: str  # the named function, message or declaration it comes from
    what: str  # what it changes, in words
    start: int  # the span of the file's text it replaces, in characters
    end: int
    replacement: str
    package: str = ""  # rust: the crate cargo-mutants names
    digest: str = ""  # the file's text it was made from

    @property
    def name(self) -> str:
        return f"{self.file}: {self.what}"


@dataclass(frozen=True)
class Entry:
    """One named thing of the spec: a function, a message or a declaration. The limits count these."""

    kind: str
    file: str
    name: str
    data: Dict = field(default_factory=dict, compare=False, hash=False)


def _digest(text: str) -> str:
    return hashlib.sha1(text.encode("utf-8")).hexdigest()


def read_text(path: Path) -> str:
    """A file's text with its line ends as they are, so offsets and the text put back are exact."""
    with open(path, encoding="utf-8", newline="") as f:
        return f.read()


def _squash(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


# ---- the spec -------------------------------------------------------------------------------------------------------

_KEYS = {
    "rust": ({"file", "functions"}, set()),
    "markup": ({"file", "functions"}, set()),
    "ftl": ({"file", "messages"}, set()),
    "css": ({"file", "rule", "property", "to"}, {"nth"}),
}
_FENCE = re.compile(r"```json\s*\n(.*?)\n```", re.S)


def read_spec(text: str) -> dict:
    """The faults of a plan: its `## Faults` block's JSON, or the JSON itself. Raises SpecError on a malformed one."""
    if "## Faults" in text:
        section = text.split("## Faults", 1)[1].split("\n## ", 1)[0]
        fence = _FENCE.search(section)
        if not fence:
            raise SpecError("the plan's ## Faults has no ```json block")
        text = fence.group(1)
    try:
        spec = json.loads(text)
    except ValueError as why:
        raise SpecError(f"the faults are not JSON: {why}") from None
    if not isinstance(spec, dict):
        raise SpecError("the faults are not a JSON object")
    for kind, items in spec.items():
        if kind not in KINDS:
            raise SpecError(f"unknown kind `{kind}` (one of {', '.join(KINDS)})")
        if not isinstance(items, list):
            raise SpecError(f"`{kind}` is not a list")
        required, optional = _KEYS[kind]
        for item in items:
            if not isinstance(item, dict):
                raise SpecError(f"an entry of `{kind}` is not an object")
            for key in sorted(set(item) - required - optional):
                raise SpecError(f"{kind}: unknown key `{key}`")
            for key in sorted(required - set(item)):
                raise SpecError(f"{kind}: an entry has no `{key}`")
            for key in ("functions", "messages"):
                if key in item and (
                    not isinstance(item[key], list) or not item[key] or not all(isinstance(n, str) and n for n in item[key])
                ):
                    raise SpecError(f"{kind}: `{key}` must be a list of names, not empty")
            if "nth" in item and (not isinstance(item["nth"], int) or item["nth"] < 1):
                raise SpecError(f"{kind}: `nth` must be a whole number from 1")
    if not any(spec.get(kind) for kind in KINDS):
        raise SpecError("the faults names nothing to fault")
    return spec


def entries(spec: dict) -> List[Entry]:
    """Each named function, message and declaration of a spec, in the plan's order (a message `*` stays one)."""
    found = []
    for kind in KINDS:
        for item in spec.get(kind, []):
            if kind == "css":
                nth = f" (nth {item['nth']})" if "nth" in item else ""
                found.append(Entry(kind, item["file"], f"{item['rule']} {{ {item['property']} }}{nth}", item))
            else:
                for name in item["functions" if kind != "ftl" else "messages"]:
                    found.append(Entry(kind, item["file"], name, item))
    return found


# ---- Rust's text ----------------------------------------------------------------------------------------------------

_RAW = re.compile(r"b?r(#*)\"")


def code_mask(src: str) -> List[bool]:
    """True for each character of Rust code, False inside comments, strings and character literals."""
    mask = [True] * len(src)
    i, n = 0, len(src)

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            mask[k] = False

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            end = src.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c in "br" and _RAW.match(src, i) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            raw = _RAW.match(src, i)
            close = '"' + raw.group(1)
            end = src.find(close, raw.end())
            end = n if end < 0 else end + len(close)
            blank(i, end)
            i = end
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i, j + 1)
            i = j + 1
        elif c == "'":
            if i + 1 < n and src[i + 1] == "\\":
                end = src.find("'", i + 2)
                end = n if end < 0 else end + 1
                blank(i, end)
                i = end
            elif i + 2 < n and src[i + 2] == "'":
                blank(i, i + 3)
                i += 3
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return mask


def _strings(src: str, mask: List[bool], a: int, b: int) -> List[Tuple[int, int]]:
    """The string literals that start between a and b, each as its span."""
    found, i = [], a
    while i < b:
        if not mask[i] and (src[i] == '"' or _RAW.match(src, i)) and (i == 0 or mask[i - 1]):
            j = i
            while j < len(src) and not mask[j]:
                j += 1
            # A string ends at its closing quote; a comment directly after it is not part of it.
            end = src.rfind('"', i, j) + 1
            raw = _RAW.match(src, i)
            if raw:
                end = src.find('"' + raw.group(1), raw.end()) + 1 + len(raw.group(1))
            found.append((i, end))
            i = j
        else:
            i += 1
    return found


def item_end(src: str, mask: List[bool], start: int) -> int:
    """Where the item that starts at `start` (an attribute, then the item) ends: its closing brace or semicolon."""
    depth, j = 0, start
    while j < len(src):
        if mask[j]:
            ch = src[j]
            if ch in "{[(":
                depth += 1
            elif ch in "}])":
                depth -= 1
                if depth == 0 and ch == "}":
                    return j + 1
            elif ch == ";" and depth == 0:
                return j + 1
        j += 1
    return j


_CFG = re.compile(r"#\[cfg\((.*?)\)\]", re.S)


def _top_args(text: str) -> List[str]:
    args, depth, cur = [], 0, ""
    for ch in text:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            args.append(cur.strip())
            cur = ""
        else:
            cur += ch
    args.append(cur.strip())
    return args


def is_test_cfg(condition: str) -> bool:
    """Exactly `test`, or `all(…)` with `test` among its own arguments: code built only for tests. `any(test, …)` is
    product code (a fake provider built for tests and for a feature), and so is `not(test)`."""
    condition = condition.strip()
    if condition == "test":
        return True
    found = re.fullmatch(r"all\s*\((.*)\)", condition, re.S)
    return bool(found) and "test" in _top_args(found.group(1))


def test_items(src: str, mask: Optional[List[bool]] = None) -> List[Tuple[int, int]]:
    """The spans of the items built only for tests (`is_test_cfg`), from the attribute to the item's end."""
    mask = mask or code_mask(src)
    spans: List[Tuple[int, int]] = []
    for found in _CFG.finditer(src):
        if not mask[found.start()] or not is_test_cfg(found.group(1)):
            continue
        if spans and found.start() < spans[-1][1]:
            continue  # inside an item already counted
        spans.append((found.start(), item_end(src, mask, found.end())))
    return spans


def _functions(src: str, mask: List[bool], name: str) -> List[Tuple[int, int, int]]:
    """Each product function of that name: its `fn`, and its body's span (`{` to `}`); a body of -1 has none."""
    tests = test_items(src, mask)
    found = []
    for match in re.finditer(r"\bfn\s+" + re.escape(name) + r"\b", src):
        if not mask[match.start()] or any(a <= match.start() < b for a, b in tests):
            continue
        depth, j = 0, match.end()
        while j < len(src):
            if mask[j]:
                ch = src[j]
                if ch in "(<[":
                    depth += 1
                elif ch in ")]" or (ch == ">" and src[j - 1] != "-"):
                    depth -= 1
                elif ch == ";" and depth == 0:
                    j = -1
                    break
                elif ch == "{" and depth == 0:
                    break
            j += 1
        if j < 0 or j >= len(src):
            found.append((match.start(), -1, -1))
        else:
            found.append((match.start(), j, item_end(src, mask, j)))
    return found


# ---- markup ---------------------------------------------------------------------------------------------------------

_VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}
_NAME = re.compile(r"[A-Za-z_][\w\-]*(?::[\w\-]+)*")
_ATTR_AHEAD = re.compile(r"[A-Za-z_][\w\-]*(?::[\w\-]+)*(?:=(?!=)|(?=\s|/?>))")
_KEYWORDS = {"move", "if", "else", "match", "in", "as", "return", "mut", "ref", "let", "dyn", "impl", "async", "await"}


@dataclass
class _Element:
    name: str
    start: int
    end: int = 0
    children: List["_Element"] = field(default_factory=list)


class _View:
    """One `view!` body read as markup: its attributes, and its elements as a tree."""

    def __init__(self, src: str, mask: List[bool], start: int, end: int):
        self.src, self.mask, self.end = src, mask, end
        self.attributes: List[Tuple[int, int, int]] = []  # (whitespace before, start, end)
        self.root = _Element("", start)
        self._children(start, self.root, None)

    def _code(self, i: int, text: str) -> bool:
        return self.src.startswith(text, i) and self.mask[i]

    def _skip_space(self, i: int) -> int:
        while i < self.end and (self.src[i].isspace() or not self.mask[i] and self.src[i] != '"' and not _RAW.match(self.src, i)):
            i += 1
        return i

    def _balanced(self, i: int) -> int:
        """The end of the bracketed group that opens at i."""
        depth, j = 0, i
        while j < self.end:
            if self.mask[j]:
                ch = self.src[j]
                if ch in "{[(":
                    depth += 1
                elif ch in "}])":
                    depth -= 1
                    if depth == 0:
                        return j + 1
            j += 1
        return j

    def _string_end(self, i: int) -> int:
        j = i
        while j < self.end and not self.mask[j]:
            j += 1
        return j

    def _children(self, i: int, parent: _Element, closing: Optional[str]) -> int:
        while True:
            i = self._skip_space(i)
            if i >= self.end:
                return i
            if self._code(i, "</"):
                close = self.src.find(">", i)
                return self.end if close < 0 else close + 1
            if self._code(i, "<"):
                i = self._element(i, parent)
            elif self._code(i, "{") or self._code(i, "(") or self._code(i, "["):
                i = self._balanced(i)
            elif not self.mask[i]:
                i = self._string_end(i)
            else:
                i += 1

    def _element(self, i: int, parent: _Element) -> int:
        element = _Element("", i)
        name = _NAME.match(self.src, i + 1)
        j = i + 1
        if name:
            element.name = name.group(0)
            j = name.end()
        closed = self._attributes(j)
        if closed is None:
            element.end = self.end
            parent.children.append(element)
            return self.end
        j, self_closing = closed
        if not self_closing and element.name.lower() not in _VOID:
            j = self._children(j, element, element.name)
        element.end = j
        parent.children.append(element)
        return j

    def _attributes(self, i: int) -> Optional[Tuple[int, bool]]:
        """Reads a tag's attributes from i; where the tag ends and whether it closes itself."""
        while i < self.end:
            before = i
            i = self._skip_space(i)
            if self._code(i, "/>"):
                return i + 2, True
            if self._code(i, ">"):
                return i + 1, False
            start = i
            if self._code(i, "{"):
                i = self._balanced(i)  # a spread or a braced attribute
            else:
                name = _NAME.match(self.src, i)
                if not name:
                    return None
                i = name.end()
                if self._code(i, "=") and not self._code(i, "=="):
                    i = self._value(i + 1)
            self.attributes.append((before, start, i))
        return None

    def _value(self, i: int) -> int:
        i = self._skip_space(i)
        if i < self.end and not self.mask[i]:
            return self._string_end(i)
        if self._code(i, "{"):
            return self._balanced(i)
        # The open brackets, and `<` for a turbofish's generics, which only a `>` closes: any other `>` inside a
        # bracket is a comparison, and one outside every bracket ends the tag.
        opened: List[str] = []
        j = i
        while j < self.end:
            if not self.mask[j]:
                j += 1
                continue
            ch = self.src[j]
            depth = len(opened)
            if ch in "{[(":
                opened.append(ch)
            elif ch in "}])":
                while opened and opened[-1] == "<":
                    opened.pop()
                if opened:
                    opened.pop()
            elif self.src.startswith("::<", j):
                opened.append("<")
                j += 3
                continue
            elif ch == ">" and opened and opened[-1] == "<" and self.src[j - 1] not in "-=":
                opened.pop()
            elif ch in "-=" and self.src.startswith(">", j + 1):
                j += 2
                continue
            elif depth == 0 and (ch == ">" or self.src.startswith("/>", j)):
                return j
            elif depth == 0 and ch.isspace():
                ahead = self._skip_space(j)
                if self._ends_value(j, ahead):
                    return j
                j = ahead
                continue
            j += 1
        return j

    def _ends_value(self, space: int, ahead: int) -> bool:
        """At whitespace inside an unbraced attribute value: does the next attribute (or the tag's end) start after it?"""
        if self._code(ahead, ">") or self._code(ahead, "/>"):
            return True
        if not _ATTR_AHEAD.match(self.src, ahead):
            return False
        word = _NAME.match(self.src, ahead).group(0)
        if word in _KEYWORDS:
            return False
        before = self.src[space - 1]
        if not (before.isalnum() or before in "_)]}\"'"):
            return False  # an operator or a closure's `|args|` before: the expression goes on
        previous = re.search(r"([A-Za-z_]\w*)$", self.src[:space])
        return not (previous and previous.group(1) in _KEYWORDS)

    def elements(self) -> Iterator[_Element]:
        stack = [self.root]
        while stack:
            element = stack.pop()
            yield element
            stack.extend(reversed(element.children))


def _views(src: str, mask: List[bool], a: int, b: int) -> List[Tuple[int, int]]:
    """The bodies of the `view!` macros between a and b, each inside its delimiters."""
    found = []
    for match in re.finditer(r"\bview!\s*([{(\[])", src[a:b]):
        start = a + match.start(1)
        if not mask[start]:
            continue
        end = item_end(src, mask, start) if src[start] == "{" else _group_end(src, mask, start)
        found.append((start + 1, end - 1))
    return found


def _group_end(src: str, mask: List[bool], i: int) -> int:
    depth, j = 0, i
    while j < len(src):
        if mask[j]:
            if src[j] in "{[(":
                depth += 1
            elif src[j] in "}])":
                depth -= 1
                if depth == 0:
                    return j + 1
        j += 1
    return j


def markup_faults(src: str, function: str, file: str = "") -> List[Fault]:
    """The markup faults of the `view!` of one product function: each attribute removed, each string literal changed,
    each two neighbouring elements of one parent swapped."""
    mask = code_mask(src)
    found = _functions(src, mask, function)
    if not found:
        raise SpecError(f"no function `{function}`")
    if len(found) > 1:
        raise SpecError(f"{len(found)} functions named `{function}`: a markup function must be the file's only one")
    _, body, end = found[0]
    views = _views(src, mask, body, end) if body >= 0 else []
    if not views:
        raise SpecError(f"`{function}` has no view!")
    digest, made = _digest(src), []

    def fault(what: str, start: int, stop: int, replacement: str) -> None:
        made.append(Fault("markup", file, function, what, start, stop, replacement, digest=digest))

    removed, changed, swapped = [], [], []
    for a, b in views:
        view = _View(src, mask, a, b)
        for before, start, stop in view.attributes:
            removed.append((start, f"remove the attribute {_squash(src[start:stop])}", before, stop, ""))
        for start, stop in _strings(src, mask, a, b):
            literal = src[start:stop]
            new = '""' if literal not in ('""',) else '"mutant"'
            changed.append((start, f"change the string {literal} to {new}", start, stop, new))
        for element in view.elements():
            for one, two in zip(element.children, element.children[1:]):
                first, second = src[one.start : one.end], src[two.start : two.end]
                if first == second:
                    continue
                between = src[one.end : two.start]
                swapped.append((one.start, f"swap <{one.name}> and <{two.name}>", one.start, two.end, second + between + first))
    for group in (removed, changed, swapped):
        for _, what, start, stop, replacement in sorted(group, key=lambda g: g[0]):
            fault(what, start, stop, replacement)
    return made


# ---- Fluent ---------------------------------------------------------------------------------------------------------

_MESSAGE = re.compile(r"^([A-Za-z][\w-]*)[ \t]*=", re.M)
_VARIABLE = re.compile(r"\$([A-Za-z][\w-]*)")


def _ftl_blocks(src: str) -> Dict[str, Tuple[int, int]]:
    """Each message's span: its first line to the end of its last indented line, with that line's end."""
    lines = src.splitlines(keepends=True)
    offsets, at = [], 0
    for line in lines:
        offsets.append(at)
        at += len(line)
    blocks: Dict[str, Tuple[int, int]] = {}
    for k, line in enumerate(lines):
        found = _MESSAGE.match(line)
        if not found:
            continue
        last = k
        for j in range(k + 1, len(lines)):
            if lines[j].strip() == "":
                continue
            if lines[j][0] in " \t":
                last = j
            else:
                break
        blocks.setdefault(found.group(1), (offsets[k], offsets[last] + len(lines[last])))
    return blocks


def ftl_messages(src: str) -> List[str]:
    return list(_ftl_blocks(src))


def _placeables(text: str) -> List[Tuple[int, int]]:
    found, depth, start, i = [], 0, 0, 0
    while i < len(text):
        ch = text[i]
        if ch == '"' and depth > 0:
            j = i + 1
            while j < len(text) and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            i = j + 1
            continue
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}" and depth > 0:
            depth -= 1
            if depth == 0:
                found.append((start, i + 1))
        i += 1
    return found


def ftl_faults(src: str, message: str, file: str = "") -> List[Fault]:
    """The faults of one Fluent message: the message removed, each of its placeables removed, each of its variables
    renamed wherever the message uses it."""
    blocks = _ftl_blocks(src)
    if message not in blocks:
        raise SpecError(f"no message `{message}`")
    start, end = blocks[message]
    text, digest = src[start:end], _digest(src)
    made = [Fault("ftl", file, message, f"remove the message {message}", start, end, "", digest=digest)]
    value = text.index("=") + 1
    for a, b in _placeables(text[value:]):
        placeable = text[value + a : value + b]
        shown = _squash(placeable)
        if "->" in shown:
            shown = shown.split("->", 1)[0] + "-> … }"
        made.append(Fault("ftl", file, message, f"remove the placeable {shown} of {message}", start + value + a, start + value + b, "", digest=digest))
    names: List[str] = []
    for found in _VARIABLE.finditer(text[value:]):
        if found.group(1) not in names:
            names.append(found.group(1))
    for name in names:
        renamed = re.sub(r"\$" + re.escape(name) + r"(?![\w-])", f"${name}-mutant", text)
        made.append(Fault("ftl", file, message, f"rename the variable ${name} of {message}", start, end, renamed, digest=digest))
    return made


# ---- CSS ------------------------------------------------------------------------------------------------------------


def _css_mask(src: str) -> List[bool]:
    mask, i, n = [True] * len(src), 0, len(src)
    while i < n:
        if src.startswith("/*", i):
            end = src.find("*/", i + 2)
            end = n if end < 0 else end + 2
            for k in range(i, end):
                mask[k] = False
            i = end
        elif src[i] in "\"'":
            quote, j = src[i], i + 1
            while j < n and src[j] != quote:
                j += 2 if src[j] == "\\" else 1
            for k in range(i, min(j + 1, n)):
                mask[k] = False
            i = j + 1
        else:
            i += 1
    return mask


def _selector(text: str) -> str:
    return re.sub(r"\s*,\s*", ", ", _squash(text))


def css_declarations(src: str) -> List[Tuple[str, str, int, int]]:
    """Every declaration: the selector of its innermost rule, its property, and its value's span (without
    `!important`)."""
    mask = _css_mask(src)
    stack: List[str] = []
    found, segment, depth, i = [], 0, 0, 0
    while i < len(src):
        ch = src[i]
        if not mask[i]:
            i += 1
            continue
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif depth == 0 and ch == "{":
            stack.append(_selector(re.sub(r"/\*.*?\*/", " ", src[segment:i], flags=re.S)))
            segment = i + 1
        elif depth == 0 and ch in ";}":
            text = src[segment:i]
            colon = next((k for k in range(segment, i) if mask[k] and src[k] == ":"), -1)
            if stack and colon >= 0 and text.strip():
                prop = re.sub(r"/\*.*?\*/", "", src[segment:colon], flags=re.S).strip()
                a, b = colon + 1, i
                while a < b and src[a].isspace():
                    a += 1
                while b > a and src[b - 1].isspace():
                    b -= 1
                important = re.search(r"\s*!\s*important$", src[a:b])
                if important:
                    b = a + important.start()
                if re.fullmatch(r"--?[\w-]+|[a-zA-Z-]+", prop):
                    found.append((stack[-1], prop, a, b))
            if ch == "}" and stack:
                stack.pop()
            segment = i + 1
        i += 1
    return found


def css_faults(src: str, rule: str, prop: str, to: str, nth: Optional[int] = None, file: str = "") -> List[Fault]:
    """The fault of one declaration: its value becomes `to`."""
    wanted = _selector(rule)
    matches = [(a, b) for selector, p, a, b in css_declarations(src) if selector == wanted and p == prop]
    if not matches:
        raise SpecError(f"no declaration {prop} in {rule}")
    if nth is None and len(matches) > 1:
        raise SpecError(f'{rule} {{ {prop} }} matches {len(matches)} declarations: give "nth" (from 1)')
    if nth is not None and nth > len(matches):
        raise SpecError(f"nth {nth}, but {rule} has {len(matches)} {prop} declaration(s)")
    a, b = matches[(nth or 1) - 1]
    was = src[a:b]
    if _squash(was) == _squash(to):
        raise SpecError(f"{rule} {{ {prop} }} is already {to}: the fault would change nothing")
    entry = f"{rule} {{ {prop} }}" + (f" (nth {nth})" if nth is not None else "")
    return [Fault("css", file, entry, f"set {prop} of {rule} to {to} (was {_squash(was)})", a, b, to, digest=_digest(src))]


# ---- Rust, from cargo-mutants ---------------------------------------------------------------------------------------


def _offset(lines: List[int], line: int, column: int) -> int:
    return lines[line - 1] + column - 1


def rust_faults(listing: List[dict], src: str, file: str, function: str) -> List[Fault]:
    """The faults cargo-mutants lists for one function (named as it prints it) of one file."""
    starts, at = [], 0
    for line in src.split("\n"):
        starts.append(at)
        at += len(line) + 1
    digest, made = _digest(src), []
    for mutant in listing:
        if mutant.get("file") != file or (mutant.get("function") or {}).get("function_name") != function:
            continue
        span = mutant["span"]
        a = _offset(starts, span["start"]["line"], span["start"]["column"])
        b = _offset(starts, span["end"]["line"], span["end"]["column"])
        what = mutant["name"].split(": ", 1)[1] if ": " in mutant["name"] else mutant["name"]
        made.append(Fault("rust", file, function, what, a, b, mutant["replacement"], mutant.get("package", ""), digest))
    if not made:
        last = re.sub(r"^.*::", "", function)
        mask = code_mask(src)
        if not any(mask[m.start()] for m in re.finditer(r"\bfn\s+" + re.escape(last) + r"\b", src)):
            raise SpecError(f"no function `{function}`")
        raise SpecError(f"cargo-mutants lists no fault for `{function}` (name it as `cargo mutants --list` prints it)")
    return made


# ---- a whole spec ---------------------------------------------------------------------------------------------------

Lister = Callable[[List[str]], List[dict]]


def make(repo: Path, spec: dict, lister: Lister) -> Tuple[List[Tuple[Entry, List[Fault]]], List[str]]:
    """Every entry's faults, made from the files of `repo`, and the problems of the entries that yield none.
    `lister` gives cargo-mutants' listing for the Rust files (called once, only when there are any)."""
    made: List[Tuple[Entry, List[Fault]]] = []
    problems: List[str] = []
    texts: Dict[str, Optional[str]] = {}

    def text(path: str) -> Optional[str]:
        if path not in texts:
            full = repo / path
            texts[path] = read_text(full) if full.is_file() else None
        return texts[path]

    rust_files = sorted({e.file for e in entries(spec) if e.kind == "rust" and text(e.file) is not None})
    listing = lister(rust_files) if rust_files else []
    missing = set()
    for entry in entries(spec):
        src = text(entry.file)
        if src is None:
            if (entry.kind, entry.file) not in missing:
                problems.append(f"{entry.kind} {entry.file}: the file does not exist")
                missing.add((entry.kind, entry.file))
            continue
        names = ftl_messages(src) if entry.kind == "ftl" and entry.name == "*" else [entry.name]
        for name in names:
            try:
                if entry.kind == "rust":
                    found = rust_faults(listing, src, entry.file, name)
                elif entry.kind == "markup":
                    found = markup_faults(src, name, entry.file)
                elif entry.kind == "ftl":
                    found = ftl_faults(src, name, entry.file)
                else:
                    d = entry.data
                    found = css_faults(src, d["rule"], d["property"], d["to"], d.get("nth"), entry.file)
            except SpecError as why:
                problems.append(f"{entry.kind} {entry.file}: {why}")
                continue
            if not found:
                problems.append(f"{entry.kind} {entry.file}: `{name}` yields no fault")
                continue
            made.append((Entry(entry.kind, entry.file, name, entry.data), found))
    return made, problems


def take(groups: List[List[Fault]], limit: int) -> List[Fault]:
    """Up to `limit` faults, taken in turn: one from each entry, in the plan's order, then the next of each."""
    chosen, k = [], 0
    while len(chosen) < limit and any(k < len(g) for g in groups):
        for group in groups:
            if k < len(group) and len(chosen) < limit:
                chosen.append(group[k])
        k += 1
    return chosen


@contextmanager
def applied(repo: Path, fault: Fault) -> Iterator[None]:
    """The fault in its file while the block runs; the file put back byte for byte after, whatever happens."""
    path = repo / fault.file
    original = path.read_bytes()
    text = read_text(path)
    if fault.digest and _digest(text) != fault.digest:
        raise SpecError(f"{fault.file} changed since the faults were made")
    path.write_bytes((text[: fault.start] + fault.replacement + text[fault.end :]).encode("utf-8"))
    try:
        yield
    finally:
        path.write_bytes(original)

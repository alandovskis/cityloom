"""The faults of the software factory's tests flow (.claude/factory/faults.py): `just test-scripts`.

The tests gate proves that a new test of working code can fail by putting a fault into the code the plan names and
seeing the test fail. These tests pin which faults each kind of entry makes (markup, Fluent messages, CSS declarations
and the Rust faults cargo-mutants lists), that a fault changes only its own span and is put back exactly, and what a
plan's `## Faults` block may say.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / ".claude" / "factory"))

import faults  # noqa: E402

VIEW = '''\
use leptos::prelude::*;

fn word(key: &str) -> String {
    key.to_string()
}

fn question(vm: Vm, ask: Ask) -> impl IntoView {
    let label = "not in the view";
    view! {
        <section class="timeline" aria-label=move || word("timeline-heading")>
            <h2>{move || word("timeline-heading")}</h2>
            <p id=ask.question_id>"Keep it?"</p>
            <button type="button" class=if ask.danger { "btn danger" } else { "btn" } on:click=move |_| {
                vm.with_value(|v| v.yes());
            }>
                {move || word("timeline-yes")}
            </button>
            <input type="checkbox" checked/>
        </section>
    }
}

#[cfg(test)]
mod tests {
    fn question() {}
}
'''


def faulted(text: str, fault: faults.Fault) -> str:
    return text[: fault.start] + fault.replacement + text[fault.end :]


class Markup(unittest.TestCase):
    def setUp(self):
        self.made = faults.markup_faults(VIEW, "question")

    def whats(self) -> list[str]:
        return [f.what for f in self.made]

    def test_each_attribute_of_the_view_can_be_removed(self):
        removed = [w for w in self.whats() if w.startswith("remove the attribute")]
        self.assertEqual(
            removed,
            [
                'remove the attribute class="timeline"',
                'remove the attribute aria-label=move || word("timeline-heading")',
                "remove the attribute id=ask.question_id",
                'remove the attribute type="button"',
                'remove the attribute class=if ask.danger { "btn danger" } else { "btn" }',
                "remove the attribute on:click=move |_| { vm.with_value(|v| v.yes()); }",
                'remove the attribute type="checkbox"',
                "remove the attribute checked",
            ],
        )

    def test_removing_an_attribute_leaves_the_tag_well_formed(self):
        fault = next(f for f in self.made if f.what == "remove the attribute id=ask.question_id")
        self.assertIn('<p>"Keep it?"</p>', faulted(VIEW, fault))
        fault = next(f for f in self.made if f.what.startswith("remove the attribute on:click"))
        self.assertIn('<button type="button" class=if ask.danger { "btn danger" } else { "btn" }>\n', faulted(VIEW, fault))

    def test_each_string_literal_of_the_view_can_be_changed(self):
        changed = [w for w in self.whats() if w.startswith("change the string")]
        self.assertEqual(
            changed,
            [
                'change the string "timeline" to ""',
                'change the string "timeline-heading" to ""',
                'change the string "timeline-heading" to ""',
                'change the string "Keep it?" to ""',
                'change the string "button" to ""',
                'change the string "btn danger" to ""',
                'change the string "btn" to ""',
                'change the string "timeline-yes" to ""',
                'change the string "checkbox" to ""',
            ],
        )
        # A string outside the view! is the code's, not the markup's.
        self.assertNotIn('change the string "not in the view" to ""', changed)

    def test_an_empty_string_becomes_a_word(self):
        made = faults.markup_faults('fn f() -> impl IntoView {\n    view! { <p title="">"a"</p> }\n}\n', "f")
        self.assertIn('change the string "" to "mutant"', [f.what for f in made])

    def test_neighbouring_elements_of_one_parent_can_be_swapped(self):
        swaps = [f for f in self.made if f.what.startswith("swap")]
        self.assertEqual([f.what for f in swaps], ["swap <h2> and <p>", "swap <p> and <button>", "swap <button> and <input>"])
        text = faulted(VIEW, swaps[0])
        self.assertLess(text.index('<p id=ask.question_id>'), text.index("<h2>"))
        self.assertEqual(text.count("<h2>"), 1)

    def test_identical_neighbours_make_no_swap(self):
        src = "fn f() -> impl IntoView {\n    view! { <ul><li>\"a\"</li><li>\"a\"</li></ul> }\n}\n"
        self.assertFalse([f for f in faults.markup_faults(src, "f") if f.what.startswith("swap")])

    def test_a_fragment_and_nested_views_are_markup_too(self):
        src = (
            "fn f(items: Vec<String>) -> impl IntoView {\n"
            "    view! {\n"
            "        <>\n"
            "            <h3>\"Title\"</h3>\n"
            "            {items.into_iter().map(|i| view! { <li class=\"item\">{i}</li> }).collect_view()}\n"
            "            <p>\"End\"</p>\n"
            "        </>\n"
            "    }\n"
            "}\n"
        )
        whats = [f.what for f in faults.markup_faults(src, "f")]
        self.assertIn("swap <h3> and <p>", whats)
        self.assertIn('remove the attribute class="item"', whats)
        self.assertIn('change the string "item" to ""', whats)

    def test_a_turbofish_or_arrow_in_an_attribute_does_not_end_the_tag(self):
        src = (
            "fn f() -> impl IntoView {\n"
            '    view! { <input prop:value=move || parse::<f64>(&v()) on:input=move |e| -> () { set(e) } type="number"/> }\n'
            "}\n"
        )
        whats = [f.what for f in faults.markup_faults(src, "f")]
        self.assertEqual(
            [w for w in whats if w.startswith("remove")],
            [
                "remove the attribute prop:value=move || parse::<f64>(&v())",
                "remove the attribute on:input=move |e| -> () { set(e) }",
                'remove the attribute type="number"',
            ],
        )

    def test_a_comparison_inside_parentheses_does_not_end_the_tag(self):
        src = 'fn f(n: u32) -> impl IntoView {\n    view! { <td class=(n > 0).then_some("bad") id="x">{n}</td> }\n}\n'
        whats = [f.what for f in faults.markup_faults(src, "f")]
        self.assertEqual(
            [w for w in whats if w.startswith("remove")],
            ['remove the attribute class=(n > 0).then_some("bad")', 'remove the attribute id="x"'],
        )

    def test_a_test_function_of_the_same_name_does_not_count(self):
        # `question` is also a function of the #[cfg(test)] module, which is no product code.
        self.assertTrue(self.made)

    def test_a_function_that_is_not_there_or_has_no_view_is_an_error(self):
        with self.assertRaisesRegex(faults.SpecError, "no function `missing`"):
            faults.markup_faults(VIEW, "missing")
        with self.assertRaisesRegex(faults.SpecError, "`word` has no view!"):
            faults.markup_faults(VIEW, "word")

    def test_a_name_held_by_two_functions_is_an_error(self):
        src = "fn f() -> impl IntoView { view! { <p/> } }\nmod m {\n    fn f() -> impl IntoView { view! { <p/> } }\n}\n"
        with self.assertRaisesRegex(faults.SpecError, "2 functions named `f`"):
            faults.markup_faults(src, "f")


FTL = """\
# The timeline's messages.
timeline-heading = Milestones
timeline-count =
    { $count ->
        [one] One milestone
       *[other] { $count } milestones
    }
timeline-named = { $name } of { -brand }, on { $day }
    .title = { $name }

-brand = CityLoom
timeline-last = Last
"""


class Fluent(unittest.TestCase):
    def whats(self, message: str) -> list[str]:
        return [f.what for f in faults.ftl_faults(FTL, message)]

    def test_a_message_can_be_removed_whole(self):
        made = faults.ftl_faults(FTL, "timeline-count")
        self.assertEqual(made[0].what, "remove the message timeline-count")
        self.assertEqual(
            faulted(FTL, made[0]),
            FTL.replace("timeline-count =\n    { $count ->\n        [one] One milestone\n       *[other] { $count } milestones\n    }\n", ""),
        )

    def test_each_placeable_can_be_removed_and_each_variable_renamed(self):
        self.assertEqual(
            self.whats("timeline-named"),
            [
                "remove the message timeline-named",
                "remove the placeable { $name } of timeline-named",
                "remove the placeable { -brand } of timeline-named",
                "remove the placeable { $day } of timeline-named",
                "remove the placeable { $name } of timeline-named",
                "rename the variable $name of timeline-named",
                "rename the variable $day of timeline-named",
            ],
        )

    def test_a_renamed_variable_is_renamed_everywhere_in_the_message(self):
        fault = next(f for f in faults.ftl_faults(FTL, "timeline-named") if f.what == "rename the variable $name of timeline-named")
        self.assertIn("timeline-named = { $name-mutant } of { -brand }, on { $day }\n    .title = { $name-mutant }\n", faulted(FTL, fault))

    def test_a_select_is_one_placeable(self):
        self.assertEqual(
            self.whats("timeline-count"),
            [
                "remove the message timeline-count",
                "remove the placeable { $count -> … } of timeline-count",
                "rename the variable $count of timeline-count",
            ],
        )

    def test_a_message_with_no_placeable_can_only_be_removed(self):
        self.assertEqual(self.whats("timeline-heading"), ["remove the message timeline-heading"])
        made = faults.ftl_faults(FTL, "timeline-last")
        self.assertEqual(faulted(FTL, made[0]), FTL.replace("timeline-last = Last\n", ""))

    def test_every_message_of_the_file(self):
        self.assertEqual(faults.ftl_messages(FTL), ["timeline-heading", "timeline-count", "timeline-named", "timeline-last"])

    def test_a_message_that_is_not_there_is_an_error(self):
        with self.assertRaisesRegex(faults.SpecError, "no message `timeline-gone`"):
            faults.ftl_faults(FTL, "timeline-gone")


CSS = """\
/* :root { --danger: red; } is a comment */
:root {
  --danger: #b00020;
  --panel: light-dark(#fff, #1c1c1c);
  --url: url("data:image/svg+xml;utf8,<svg/>");
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --danger: #ff6b6b;
  }
}

.btn,
.btn.danger {
  color: var(--danger) !important;
}

:root {
  --danger: #c00;
}
"""


class Css(unittest.TestCase):
    def test_a_declaration_takes_the_planned_value(self):
        made = faults.css_faults(CSS, ":root", "--panel", "light-dark(#000, #1c1c1c)")
        self.assertEqual([f.what for f in made], ["set --panel of :root to light-dark(#000, #1c1c1c) (was light-dark(#fff, #1c1c1c))"])
        self.assertIn("--panel: light-dark(#000, #1c1c1c);", faulted(CSS, made[0]))

    def test_a_rule_inside_a_media_query_and_a_selector_list(self):
        made = faults.css_faults(CSS, ':root:not([data-theme="light"])', "--danger", "#000")
        self.assertIn("    --danger: #000;\n", faulted(CSS, made[0]))
        made = faults.css_faults(CSS, ".btn, .btn.danger", "color", "inherit")
        self.assertIn("color: inherit !important;", faulted(CSS, made[0]))

    def test_a_semicolon_inside_a_string_or_url_is_no_end(self):
        made = faults.css_faults(CSS, ":root", "--url", "none")
        self.assertIn("  --url: none;\n}", faulted(CSS, made[0]))

    def test_a_comment_before_a_declaration_is_not_its_property(self):
        src = ":root {\n  --r: 16px;\n  /* Fixed on purpose: the box is pale. */\n  --ring: #0b1220;\n}\n"
        made = faults.css_faults(src, ":root", "--ring", "#fff")
        self.assertEqual(faulted(src, made[0]), src.replace("#0b1220", "#fff"))

    def test_two_matching_declarations_need_nth(self):
        with self.assertRaisesRegex(faults.SpecError, 'matches 2 declarations: give "nth"'):
            faults.css_faults(CSS, ":root", "--danger", "#000")
        made = faults.css_faults(CSS, ":root", "--danger", "#000", nth=2)
        self.assertIn(":root {\n  --danger: #000;\n}\n", faulted(CSS, made[0]))
        self.assertIn("--danger: #b00020;", faulted(CSS, made[0]))

    def test_no_such_declaration_or_the_same_value_is_an_error(self):
        with self.assertRaisesRegex(faults.SpecError, "no declaration --gone in :root"):
            faults.css_faults(CSS, ":root", "--gone", "0")
        with self.assertRaisesRegex(faults.SpecError, "already #b00020"):
            faults.css_faults(CSS, ":root", "--danger", "#b00020", nth=1)
        with self.assertRaisesRegex(faults.SpecError, "nth 3"):
            faults.css_faults(CSS, ":root", "--danger", "#000", nth=3)


RUST = """\
fn word(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.word(key))
}

impl Page for BrowserPage {
    fn copy_text(&self, text: &str) -> bool {
        let done = clip(text);
        done
    }
}
"""

LISTING = [
    {
        "file": "src/a.rs",
        "package": "cityloom_editor",
        "function": {"function_name": "word"},
        "name": "src/a.rs:2:5: replace word -> String with String::new()",
        "replacement": "String::new()",
        "span": {"start": {"line": 2, "column": 5}, "end": {"line": 2, "column": 35}},
    },
    {
        "file": "src/a.rs",
        "package": "cityloom_editor",
        "function": {"function_name": "<impl Page for BrowserPage>::copy_text"},
        "name": "src/a.rs:7:9: replace <impl Page for BrowserPage>::copy_text -> bool with true",
        "replacement": "true",
        "span": {"start": {"line": 7, "column": 9}, "end": {"line": 8, "column": 13}},
    },
    {
        "file": "src/b.rs",
        "package": "cityloom_editor",
        "function": {"function_name": "word"},
        "name": "src/b.rs:1:1: replace word with ()",
        "replacement": "()",
        "span": {"start": {"line": 1, "column": 1}, "end": {"line": 1, "column": 2}},
    },
]


class Rust(unittest.TestCase):
    def test_the_named_functions_faults_replace_their_span(self):
        made = faults.rust_faults(LISTING, RUST, "src/a.rs", "<impl Page for BrowserPage>::copy_text")
        self.assertEqual([f.what for f in made], ["replace <impl Page for BrowserPage>::copy_text -> bool with true"])
        self.assertEqual(made[0].package, "cityloom_editor")
        self.assertIn("    fn copy_text(&self, text: &str) -> bool {\n        true\n    }\n", faulted(RUST, made[0]))

    def test_only_the_named_files_function(self):
        made = faults.rust_faults(LISTING, RUST, "src/a.rs", "word")
        self.assertEqual(len(made), 1)
        self.assertEqual(faulted(RUST, made[0]).splitlines()[1], "    String::new()")

    def test_a_function_that_is_not_in_the_file_is_an_error(self):
        with self.assertRaisesRegex(faults.SpecError, "no function `gone`"):
            faults.rust_faults(LISTING, RUST, "src/a.rs", "gone")

    def test_a_function_cargo_mutants_lists_nothing_for_yields_no_fault(self):
        with self.assertRaisesRegex(faults.SpecError, "cargo-mutants lists no fault for `copy_text`"):
            faults.rust_faults(LISTING, RUST, "src/a.rs", "copy_text")

    def test_columns_count_characters_not_bytes(self):
        text = 'fn s() -> &str {\n    "é"\n}\n'
        listing = [
            {
                "file": "s.rs",
                "package": "p",
                "function": {"function_name": "s"},
                "name": 's.rs:2:5: replace s -> &str with ""',
                "replacement": '""',
                "span": {"start": {"line": 2, "column": 5}, "end": {"line": 2, "column": 8}},
            }
        ]
        self.assertEqual(faulted(text, faults.rust_faults(listing, text, "s.rs", "s")[0]), 'fn s() -> &str {\n    ""\n}\n')


class Spec(unittest.TestCase):
    def test_a_plans_faults_block_is_read_from_the_plan(self):
        plan = (
            "## Tests\n- model\n\n## Faults\n\n```json\n"
            '{ "rust": [{ "file": "a.rs", "functions": ["f"] }] }\n'
            "```\n\n## Version\npatch\n"
        )
        self.assertEqual(faults.read_spec(plan), {"rust": [{"file": "a.rs", "functions": ["f"]}]})
        css = {"css": [{"file": "web/app.css", "rule": ":root", "property": "--x", "to": "0", "nth": 2}]}
        self.assertEqual(faults.read_spec(json.dumps(css)), css)

    def test_a_malformed_spec_is_an_error(self):
        for text, why in [
            ("## Faults\nnone\n", "no ```json block"),
            ('{"perl": []}', "unknown kind `perl`"),
            ('{"rust": [{"file": "a.rs"}]}', "`functions`"),
            ('{"ftl": [{"file": "a.ftl", "messages": []}]}', "`messages`"),
            ('{"css": [{"file": "a.css", "rule": ":root", "property": "--x"}]}', "`to`"),
            ('{"markup": [{"file": "a.rs", "functions": ["f"], "extra": 1}]}', "unknown key `extra`"),
            ("{}", "names nothing"),
        ]:
            with self.subTest(text):
                with self.assertRaisesRegex(faults.SpecError, why):
                    faults.read_spec(text)

    def test_entries_are_counted_one_per_function_message_or_declaration(self):
        spec = {
            "rust": [{"file": "a.rs", "functions": ["f", "g"]}],
            "ftl": [{"file": "a.ftl", "messages": ["m"]}],
            "css": [{"file": "a.css", "rule": ":root", "property": "--x", "to": "0"}],
        }
        self.assertEqual(
            [(e.kind, e.file, e.name) for e in faults.entries(spec)],
            [("rust", "a.rs", "f"), ("rust", "a.rs", "g"), ("ftl", "a.ftl", "m"), ("css", "a.css", ":root { --x }")],
        )


class Make(unittest.TestCase):
    """The faults of a whole spec, made from the files of a checkout."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        for name, text in {"src/a.rs": RUST, "src/view.rs": VIEW, "web/app.css": CSS, "i18n/en.ftl": FTL}.items():
            (self.repo / name).parent.mkdir(parents=True, exist_ok=True)
            (self.repo / name).write_text(text, encoding="utf-8")
        self.listed: list[list[str]] = []

    def tearDown(self):
        self.tmp.cleanup()

    def lister(self, files: list[str]) -> list[dict]:
        self.listed.append(files)
        return [m for m in LISTING if m["file"] in files]

    def test_each_entry_gets_its_faults_and_each_problem_is_named(self):
        spec = {
            "rust": [{"file": "src/a.rs", "functions": ["word", "gone"]}],
            "markup": [{"file": "src/view.rs", "functions": ["question"]}],
            "ftl": [{"file": "i18n/en.ftl", "messages": ["*"]}, {"file": "i18n/fr.ftl", "messages": ["x"]}],
            "css": [{"file": "web/app.css", "rule": ":root", "property": "--panel", "to": "red"}],
        }
        made, problems = faults.make(self.repo, spec, self.lister)
        self.assertEqual(self.listed, [["src/a.rs"]])
        self.assertEqual([e.name for e, _ in made], ["word", "question", "timeline-heading", "timeline-count", "timeline-named", "timeline-last", ":root { --panel }"])
        self.assertTrue(all(found for _, found in made))
        self.assertEqual(
            problems,
            ["rust src/a.rs: no function `gone`", "ftl i18n/fr.ftl: the file does not exist"],
        )

    def test_faults_are_taken_in_turn_one_from_each_entry(self):
        a = [faults.Fault("css", "a", "e1", f"a{i}", 0, 0, "") for i in range(3)]
        b = [faults.Fault("css", "b", "e2", f"b{i}", 0, 0, "") for i in range(1)]
        c = [faults.Fault("css", "c", "e3", f"c{i}", 0, 0, "") for i in range(2)]
        self.assertEqual([f.what for f in faults.take([a, b, c], 5)], ["a0", "b0", "c0", "a1", "c1"])
        self.assertEqual([f.what for f in faults.take([a, b, c], 100)], ["a0", "b0", "c0", "a1", "c1", "a2"])

    def test_a_fault_is_applied_and_put_back_exactly(self):
        path = self.repo / "web/app.css"
        path.write_bytes(CSS.replace("\n", "\r\n").encode("utf-8"))
        before = path.read_bytes()
        made, problems = faults.make(self.repo, {"css": [{"file": "web/app.css", "rule": ":root", "property": "--panel", "to": "red"}]}, self.lister)
        self.assertEqual(problems, [])
        fault = made[0][1][0]
        with faults.applied(self.repo, fault):
            self.assertIn(b"--panel: red;\r\n", path.read_bytes())
        self.assertEqual(path.read_bytes(), before)

    def test_a_fault_is_put_back_when_the_run_fails(self):
        before = (self.repo / "src/a.rs").read_bytes()
        made, _ = faults.make(self.repo, {"rust": [{"file": "src/a.rs", "functions": ["word"]}]}, self.lister)
        with self.assertRaises(RuntimeError):
            with faults.applied(self.repo, made[0][1][0]):
                raise RuntimeError("the build broke")
        self.assertEqual((self.repo / "src/a.rs").read_bytes(), before)

    def test_a_file_changed_since_the_faults_were_made_is_refused(self):
        made, _ = faults.make(self.repo, {"rust": [{"file": "src/a.rs", "functions": ["word"]}]}, self.lister)
        (self.repo / "src/a.rs").write_text("// changed\n" + RUST, encoding="utf-8")
        with self.assertRaisesRegex(faults.SpecError, "changed since"):
            with faults.applied(self.repo, made[0][1][0]):
                pass

    def test_a_fault_names_its_file_entry_and_change(self):
        fault = faults.Fault("css", "web/app.css", ":root { --panel }", "set --panel of :root to red", 0, 0, "")
        self.assertEqual(fault.name, "web/app.css: set --panel of :root to red")
        self.assertEqual(json.loads(json.dumps(fault.name)), fault.name)


if __name__ == "__main__":
    unittest.main()

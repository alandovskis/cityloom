---
id: issues
title: Reporting a problem
sidebar_position: 4
---

# Reporting a problem

You do not need to be a developer to report a problem or share an idea. Choose
[New issue](https://github.com/alandovskis/cityloom/issues/new/choose) on GitHub, then the form that fits: something is
broken, a street or junction is drawn wrong, you cannot make the design you want, a check is wrong, or you have an idea.
Every form is in English and French, and you can write in either language.

Each form asks which page you were on and which browser you use. Pick the closest answer; "Not sure" is fine. Then say,
in your own words, what you saw. A link to the page (copy it from the address bar) helps a lot.

## How an issue gets its labels

1. **The form.** What you pick for the page and the browser becomes a label right away, by a rule that reads only that
   answer. Nothing you write in the text is used to guess a label. The form also gives your issue its category and the
   `needs-triage` label.
2. **Triage.** A maintainer reads the issue, checks the labels, and sets the priority and the status by hand. They may
   ask you for more information.

## What the labels mean

Each label has an English and a French name, as in `C-bug · bogue`, and its description says its family. The first
letter is the family.

| Family                 | Set by                | Meaning                                                                                                  |
| ---------------------- | --------------------- | -------------------------------------------------------------------------------------------------------- |
| `A-` Area              | The form, then triage | The part of CityLoom: the street or junction editor, the map and so on.                                  |
| `C-` Category          | The form              | The kind of issue: a bug, a street drawn wrong, an idea and so on.                                       |
| `O-` Browser           | The form              | The browser or device: Chrome, Edge, Firefox, Safari, iPhone or iPad, Android.                           |
| `P-` Priority          | The maintainer        | How soon it should be fixed: critical, high, medium or low.                                              |
| `S-` Status            | The maintainer        | What it is waiting for: more information, steps to reproduce, something else, or a fix in OpenStreetMap. |
| `needs-triage`         | The form              | No maintainer has looked at it yet.                                                                      |
| `regression-untriaged` | The maintainer        | It worked before, and no one has looked into why yet.                                                    |

The categories:

- `C-bug`: something is broken.
- `C-drawn-wrong`: a street or junction does not look like the real one. The streets come from OpenStreetMap, so the
  fix may be there: the maintainer then adds `S-upstream-osm`.
- `C-design-request`: the editor cannot draw the design you want to show.
- `C-unsafe-check`: a check calls a design safe when it is not, or the reverse.
- `C-enhancement`: an idea. `C-question`: a question. `C-tracking-issue`: an issue that follows several others.

The labels are defined in `.github/labels.yml` in the repository and updated from it.

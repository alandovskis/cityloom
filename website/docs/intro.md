---
id: intro
slug: /
title: Introduction
sidebar_position: 1
---

# Introduction

CityLoom is a street, junction and city-map editor. It is written in Rust, compiled to WebAssembly and drawn with
[Leptos](https://leptos.dev/) 0.8 (client-side rendering). It runs in the browser; there is no server of its own.

The streets come from [OpenStreetMap](https://www.openstreetmap.org/). The rules, rates and thresholds of the
catalogue (what a check measures a street against) are synthetic placeholders, not a standard.

The app is available in English and Canadian French.

## The four pages

| Page | File | What it is for |
| --- | --- | --- |
| Home | `web/index.html` | Find a place: search for a city or a neighbourhood and load its streets. |
| Map | `web/map.html` | The city's streets and junctions over a basemap. Open a place to edit it, and declare areas with rules (the Area rules panel). |
| Street | `web/street.html` | Edit one street's cross-section: arrange its pieces, read the checks, and view it in 2.5D. |
| Junction | `web/intersection.html` | Edit one junction in plan: its arms, corners, crossings and how it is controlled. |

The street and junction pages are opened from the map: the map gives each place an address such as `?street=7` or
`?junction=3`. A page opened that way edits one place of the city, and its edits are kept in the browser's storage.

## Where to go next

- [Getting started](getting-started.md) builds and serves the app on your machine.
- [Architecture](architecture.md) explains how the code is organised.

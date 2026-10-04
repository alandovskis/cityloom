#!/usr/bin/env node
// Writes web/basemap-light.json and web/basemap-dark.json: the basemap's MapLibre styles over the OpenMapTiles
// schema, from one palette per theme so that the two do not drift. The colours follow web/app.css's tokens.
// Run `node scripts/make_basemap_style.mjs` after changing the palettes or the layers, and commit the result.

import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const palettes = {
  light: {
    land: "#e2e7ec", water: "#b8d2e6", park: "#cde2c8", wood: "#bcd8b6", building: "#d3d9df", buildingLine: "#c2c9d1",
    road: "#ffffff", minor: "#f7f9fb", casing: "#c4ccd4", major: "#fde3b4", majorCasing: "#d9b574", path: "#aab3bd",
    label: "#4b5563", halo: "#ffffff", place: "#111827", waterLabel: "#4a77a1",
  },
  dark: {
    land: "#0f1317", water: "#142230", park: "#15261b", wood: "#122015", building: "#192027", buildingLine: "#222b33",
    road: "#303941", minor: "#262e35", casing: "#0b0f13", major: "#5d4d30", majorCasing: "#2a2418", path: "#3b454f",
    label: "#9aa6b2", halo: "#0f1317", place: "#e8edf2", waterLabel: "#6f9bc2",
  },
};

const REGULAR = ["Noto Sans Regular"];
const BOLD = ["Noto Sans Bold"];
const ROADS = ["motorway", "trunk", "primary", "secondary", "tertiary", "minor", "service"];
const BIG = ["motorway", "trunk", "primary"];

/** A width that grows with the zoom, by the class of the road. */
const widths = (z0, z1, byClass, fallback) => [
  "interpolate", ["exponential", 1.5], ["zoom"],
  z0, ["match", ["get", "class"], ...Object.entries(byClass[0]).flat(), fallback[0]],
  z1, ["match", ["get", "class"], ...Object.entries(byClass[1]).flat(), fallback[1]],
];

const roadWidth = (extra) =>
  widths(10, 20, [
    { motorway: 1.5 + extra, trunk: 1.5 + extra, primary: 1.2 + extra, secondary: 1 + extra, tertiary: 0.8 + extra },
    { motorway: 22 + extra, trunk: 22 + extra, primary: 20 + extra, secondary: 18 + extra, tertiary: 16 + extra, minor: 12 + extra, service: 6 + extra },
  ], [0.4 + extra, 12 + extra]);

function layers(c) {
  const isRoad = ["in", ["get", "class"], ["literal", ROADS]];
  return [
    { id: "background", type: "background", paint: { "background-color": c.land } },
    { id: "landcover", type: "fill", source: "basemap", "source-layer": "landcover", filter: ["in", ["get", "class"], ["literal", ["grass", "wood"]]],
      paint: { "fill-color": ["match", ["get", "class"], "wood", c.wood, c.park], "fill-opacity": 0.7 } },
    { id: "park", type: "fill", source: "basemap", "source-layer": "park", paint: { "fill-color": c.park, "fill-opacity": 0.8 } },
    { id: "water", type: "fill", source: "basemap", "source-layer": "water", paint: { "fill-color": c.water } },
    { id: "waterway", type: "line", source: "basemap", "source-layer": "waterway",
      paint: { "line-color": c.water, "line-width": ["interpolate", ["linear"], ["zoom"], 8, 0.5, 16, 3] } },
    { id: "building", type: "fill", source: "basemap", "source-layer": "building", minzoom: 14,
      paint: { "fill-color": c.building, "fill-outline-color": c.buildingLine } },
    { id: "path", type: "line", source: "basemap", "source-layer": "transportation", minzoom: 14,
      filter: ["in", ["get", "class"], ["literal", ["path", "pedestrian", "track"]]],
      paint: { "line-color": c.path, "line-width": ["interpolate", ["linear"], ["zoom"], 14, 0.6, 20, 2], "line-dasharray": [2, 2] } },
    { id: "road-casing", type: "line", source: "basemap", "source-layer": "transportation", filter: isRoad,
      layout: { "line-cap": "round", "line-join": "round" },
      paint: { "line-color": ["match", ["get", "class"], ...BIG.flatMap((k) => [k, c.majorCasing]), c.casing], "line-width": roadWidth(1.2) } },
    { id: "road", type: "line", source: "basemap", "source-layer": "transportation", filter: isRoad,
      layout: { "line-cap": "round", "line-join": "round" },
      paint: { "line-color": ["match", ["get", "class"], "minor", c.minor, "service", c.minor, ...BIG.flatMap((k) => [k, c.major]), c.road], "line-width": roadWidth(0) } },
    { id: "road-name", type: "symbol", source: "basemap", "source-layer": "transportation_name", minzoom: 14,
      layout: { "symbol-placement": "line", "text-field": ["get", "name"], "text-font": REGULAR, "text-size": 11, "text-letter-spacing": 0.02 },
      paint: { "text-color": c.label, "text-halo-color": c.halo, "text-halo-width": 1.5 } },
    { id: "water-name", type: "symbol", source: "basemap", "source-layer": "water_name",
      layout: { "text-field": ["get", "name"], "text-font": REGULAR, "text-size": 12 },
      paint: { "text-color": c.waterLabel, "text-halo-color": c.halo, "text-halo-width": 1.2 } },
    { id: "place", type: "symbol", source: "basemap", "source-layer": "place", minzoom: 10,
      layout: { "text-field": ["get", "name"], "text-font": BOLD, "text-size": ["match", ["get", "class"], "city", 16, "suburb", 13, 12] },
      paint: { "text-color": c.place, "text-halo-color": c.halo, "text-halo-width": 1.5 } },
  ];
}

const style = (theme) => ({
  version: 8,
  name: `CityLoom ${theme}`,
  // The adapter replaces these two with absolute addresses: MapLibre needs them absolute.
  glyphs: "GLYPHS",
  sources: {
    basemap: {
      type: "vector",
      url: "pmtiles://TILES",
      attribution: '© <a href="https://openmaptiles.org/">OpenMapTiles</a> © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>',
    },
  },
  layers: layers(palettes[theme]),
});

for (const theme of Object.keys(palettes)) {
  const file = fileURLToPath(new URL(`../web/basemap-${theme}.json`, import.meta.url));
  writeFileSync(file, JSON.stringify(style(theme), null, 2) + "\n");
  console.log(file);
}

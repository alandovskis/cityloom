// Draws the city the Rust core returns: every street and junction as a place
// on the map, with whether it works. Pressing a junction opens its plan,
// pressing a street opens its cross-section; both are plain links to the two
// editors, which read the same city. No rules live here: what is a street,
// where it runs and what is wrong with it all come from the WebAssembly model.

import init, { catalogue, materials } from "./pkg/cityloom_editor.js";
import { HATCH } from "./symbols.js";
import { openCity, regionIndex, resetCity } from "./city.js";
import { initAccountMenu, initDrawingStyle, initPanels, initRegion, initTheme, initUnits } from "./shell.js";

await init();

const KINDS = JSON.parse(catalogue());
const MATERIALS = JSON.parse(materials());
const KIND_NAME = Object.fromEntries(KINDS.map((k) => [k.id, k.name]));
const KIND_ORDER = KINDS.map((k) => k.id);

let region = regionIndex(MATERIALS.regions);
let city = openCity();
let view = JSON.parse(city.view(region));
let units = "m";

const $ = (id) => document.getElementById(id);
const el = {
  box: $("map-view"),
  svg: $("map"),
  overlay: $("map-overlay"),
  inspector: $("inspector"),
  reset: $("reset"),
  live: $("live"),
};

// ---- formatting -----------------------------------------------------------

const MM_PER_FT = 304.8;
const fmt = (mm) => `${(units === "m" ? mm / 1000 : mm / MM_PER_FT).toFixed(1)} ${units}`;
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const r2 = (n) => Math.round(n * 100) / 100;
const plural = (n, one, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

let liveTimer = 0;
function say(text) {
  clearTimeout(liveTimer);
  el.live.textContent = "";
  liveTimer = setTimeout(() => (el.live.textContent = text), 40);
}

const ICON = {
  bad: `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 3l10 10M13 3 3 13"/></svg>`,
  changed: `<svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4.5" y="4.5" width="7" height="7"/></svg>`,
};

// ---- the places -------------------------------------------------------------

const nodes = () => new Map(view.nodes.map((n) => [n.uid, n]));
const junctions = () => view.nodes.filter((n) => n.junction).sort((a, b) => junctionNumber(a) - junctionNumber(b));
const junctionNumber = (n) => n.name.replace(/^\D+/, "");
const streetHref = (e) => `index.html?street=${e.uid}`;
const junctionHref = (n) => `intersection.html?junction=${n.uid}`;
const endsOf = (e) => e.name.split(" · ")[1] ?? "";

function describe(place, isStreet) {
  const state = place.ok ? "" : ` Needs attention: ${place.failing.join(", ").toLowerCase()}.`;
  const changed = place.edited ? " Changed." : "";
  if (isStreet) {
    return `${place.kind}, ${endsOf(place)}, ${fmt(place.row_mm)} wide.${state}${changed} Opens the street cross-section.`;
  }
  return `${place.name}, ${place.control?.toLowerCase()}, ${plural(place.arms, "street")}.${state}${changed} Opens the junction plan.`;
}

// ---- the camera ---------------------------------------------------------------

// The map is drawn in metres. `k` is pixels to the metre; (cx, cy) is the point
// at the middle of the window.
const cam = { k: 1, cx: 0, cy: 0 };
let fitK = 1;
let size = { w: 800, h: 520 };
let moved = false; // the person has zoomed or panned, so a resize keeps their view

const MARGIN_M = 70;
const world = () => {
  const [x0, y0, x1, y1] = view.bounds_mm.map((v) => v / 1000);
  // Extra room at the left keeps the scale bar clear of the streets.
  return { x0: x0 - MARGIN_M - 190, y0: y0 - MARGIN_M, x1: x1 + MARGIN_M, y1: y1 + MARGIN_M };
};

function fit() {
  const b = world();
  fitK = Math.min(size.w / (b.x1 - b.x0), size.h / (b.y1 - b.y0));
  cam.k = fitK;
  cam.cx = (b.x0 + b.x1) / 2;
  cam.cy = (b.y0 + b.y1) / 2;
  moved = false;
}

function clampCam() {
  const b = world();
  cam.k = Math.min(Math.max(cam.k, fitK * 0.8), fitK * 10);
  cam.cx = Math.min(Math.max(cam.cx, b.x0), b.x1);
  cam.cy = Math.min(Math.max(cam.cy, b.y0), b.y1);
}

function setViewBox() {
  const w = size.w / cam.k;
  const h = size.h / cam.k;
  el.svg.setAttribute("viewBox", `${r2(cam.cx - w / 2)} ${r2(cam.cy - h / 2)} ${r2(w)} ${r2(h)}`);
  renderOverlay();
}

let frame = 0;
let kShown = 0;
function update() {
  clampCam();
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    // Widths that stay a few pixels wide are set in metres, so a new zoom redraws.
    if (cam.k !== kShown) renderMap();
    setViewBox();
  });
}

function zoomAt(k, clientX, clientY) {
  const r = el.box.getBoundingClientRect();
  const sx = clientX - r.left - size.w / 2;
  const sy = clientY - r.top - size.h / 2;
  const wx = cam.cx + sx / cam.k;
  const wy = cam.cy + sy / cam.k;
  cam.k = Math.min(Math.max(k, fitK * 0.8), fitK * 10);
  cam.cx = wx - sx / cam.k;
  cam.cy = wy - sy / cam.k;
  moved = true;
  update();
}

const zoomBy = (f) => {
  const r = el.box.getBoundingClientRect();
  zoomAt(cam.k * f, r.left + size.w / 2, r.top + size.h / 2);
};

// ---- the drawing --------------------------------------------------------------

const TINT_KINDS = new Set(KIND_ORDER);

// Hatch for the map: the same textures as the section, kept to the same size on
// screen at every zoom.
function mapHatch(px) {
  return Object.entries(HATCH)
    .map(([kind, svg]) => {
      const t = `scale(${r2(px)})`;
      const withId = svg.replace(`id="h-${kind}"`, `id="mh-${kind}"`);
      return withId.includes('patternTransform="') ? withId.replace('patternTransform="', `patternTransform="${t} `) : withId.replace("<pattern ", `<pattern patternTransform="${t}" `);
    })
    .join("");
}

function geometry(e, byId) {
  const a = byId.get(e.a);
  const b = byId.get(e.b);
  const ax = a.x_mm / 1000;
  const ay = a.y_mm / 1000;
  const dx = b.x_mm / 1000 - ax;
  const dy = b.y_mm / 1000 - ay;
  const len = Math.hypot(dx, dy);
  const ux = dx / len;
  const uy = dy / len;
  return { ax, ay, ux, uy, nx: -uy, ny: ux, len };
}

// A line along a street, shifted `off` metres to its right, from `t0` to `t1` along it.
const line = (g, off, t0, t1) =>
  `x1="${r2(g.ax + g.ux * t0 + g.nx * off)}" y1="${r2(g.ay + g.uy * t0 + g.ny * off)}" x2="${r2(g.ax + g.ux * t1 + g.nx * off)}" y2="${r2(g.ay + g.uy * t1 + g.ny * off)}"`;

function renderMap() {
  const k = cam.k;
  kShown = k;
  const px = (n) => n / k;
  const byId = nodes();
  const edges = view.edges.map((e) => {
    const g = geometry(e, byId);
    const row = e.row_mm / 1000;
    const road = e.pieces.filter((p) => p.kind !== "sidewalk" && p.kind !== "planting");
    const lo = Math.min(...road.map((p) => p.offset_mm - p.width_mm / 2)) / 1000;
    const hi = Math.max(...road.map((p) => p.offset_mm + p.width_mm / 2)) / 1000;
    return { e, g, row, roadOff: (lo + hi) / 2, roadW: hi - lo, t0: e.trim_a_mm / 1000, t1: g.len - e.trim_b_mm / 1000 };
  });
  const places = junctions();
  const rad = (n) => n.radius_mm / 1000;

  const casing =
    edges.map(({ e, g, row }) => `<line id="hl-s-${e.uid}" class="m-hl" ${line(g, 0, 0, g.len)} stroke-width="${r2(row + px(14))}"/>`).join("") +
    places.map((n) => `<circle id="hl-j-${n.uid}" class="m-hl" cx="${n.x_mm / 1000}" cy="${n.y_mm / 1000}" r="${r2(rad(n) + px(8))}"/>`).join("");

  const bad =
    edges.filter(({ e }) => !e.ok).map(({ e, g, row }) => `<line class="m-bad" ${line(g, 0, 0, g.len)} stroke-width="${r2(row + px(7))}"/>`).join("") +
    places.filter((n) => !n.ok).map((n) => `<circle class="m-bad-disc" cx="${n.x_mm / 1000}" cy="${n.y_mm / 1000}" r="${r2(rad(n) + px(4))}"/>`).join("");

  const outline = edges.map(({ g, row }) => `<line class="m-out" ${line(g, 0, 0, g.len)} stroke-width="${r2(row)}"/>`).join("");
  const walk = edges
    .map(({ e, g, row }) => `<line class="${e.freeway ? "m-shoulder" : "m-walk"}" ${line(g, 0, 0, g.len)} stroke-width="${r2(row - px(2))}"/>`)
    .join("");
  const road = edges.map(({ g, roadOff, roadW }) => `<line class="m-road" ${line(g, roadOff, 0, g.len)} stroke-width="${r2(roadW)}"/>`).join("");
  const strips = (cls, paint) =>
    edges
      .flatMap(({ e, g, t0, t1 }) =>
        t1 <= t0 ? [] : e.pieces.filter((p) => TINT_KINDS.has(p.kind)).map((p) => `<line class="${cls(p)}" ${paint(p)} ${line(g, p.offset_mm / 1000, t0, t1)} stroke-width="${r2(p.width_mm / 1000)}"/>`),
      )
      .join("");
  const tint = strips((p) => `m-t m-k-${p.kind}`, () => "");
  const hatch = strips(() => "m-h", (p) => `stroke="url(#mh-${p.kind})"`);

  // Names sit beside a street, on the upper side, where it is long enough to hold one.
  const names = edges
    .filter(({ g, t0, t1 }) => (t1 - t0) * k >= 150)
    .map(({ e, g, row, t0, t1 }) => {
      const tm = (t0 + t1) / 2;
      let ang = (Math.atan2(g.uy, g.ux) * 180) / Math.PI;
      if (ang > 90 || ang < -90) ang += 180;
      const x = g.ax + g.ux * tm;
      const y = g.ay + g.uy * tm;
      return `<text class="m-name" transform="translate(${r2(x)} ${r2(y)}) rotate(${r2(ang)})" dy="${r2(-(row / 2 + px(6)))}" text-anchor="middle" font-size="${r2(px(13))}" stroke-width="${r2(px(4))}">${esc(e.kind)}${e.edited ? " · changed" : ""}</text>`;
    })
    .join("");
  const badge = (x, y) =>
    `<g class="m-badge" transform="translate(${r2(x)} ${r2(y)})"><circle r="${r2(px(8))}" stroke-width="${r2(px(2))}"/><path d="M${r2(-px(3))} ${r2(-px(3))}l${r2(px(6))} ${r2(px(6))}m0 ${r2(-px(6))}l${r2(-px(6))} ${r2(px(6))}" stroke-width="${r2(px(2))}"/></g>`;
  const edgeBadges = edges
    .filter(({ e }) => !e.ok)
    .map(({ g, t0, t1 }) => badge(g.ax + g.ux * ((t0 + t1) / 2), g.ay + g.uy * ((t0 + t1) / 2)))
    .join("");
  const marks = places
    .map((n) => {
      const x = n.x_mm / 1000;
      const y = n.y_mm / 1000;
      return (
        `<g transform="translate(${x} ${y})"><circle class="m-jc" r="${r2(px(11))}" stroke-width="${r2(px(1.5))}"/>` +
        `<text class="m-jn" text-anchor="middle" dy="0.35em" font-size="${r2(px(13))}">${esc(junctionNumber(n))}</text>` +
        (n.edited ? `<text class="m-tag" text-anchor="middle" y="${r2(px(26))}" font-size="${r2(px(12))}" stroke-width="${r2(px(4))}">changed</text>` : "") +
        `</g>` +
        (n.ok ? "" : badge(x + px(12), y - px(12)))
      );
    })
    .join("");

  const streetLinks = edges
    .map(
      ({ e, g, row, t0, t1 }) =>
        `<a class="place" href="${streetHref(e)}" data-hl="s-${e.uid}" aria-label="${esc(describe(e, true))}"><title>${esc(e.name)}</title><line class="m-hit" ${line(g, 0, t0, Math.max(t1, t0 + 0.01))} stroke-width="${r2(Math.max(row, px(18)))}"/></a>`,
    )
    .join("");
  const junctionLinks = places
    .map(
      (n) =>
        `<a class="place" href="${junctionHref(n)}" data-hl="j-${n.uid}" aria-label="${esc(describe(n, false))}"><title>${esc(n.name)}</title><circle class="m-hit" cx="${n.x_mm / 1000}" cy="${n.y_mm / 1000}" r="${r2(Math.max(rad(n), px(16)))}"/></a>`,
    )
    .join("");

  el.svg.innerHTML = `<defs>${mapHatch(1 / k)}</defs>${casing}${bad}${outline}${walk}${road}${tint}${hatch}${names}${edgeBadges}${marks}${streetLinks}${junctionLinks}`;
  syncHot();
}

// North and the scale bar are drawn on the window, not on the map, so they keep their size.
const NICE_M = [10, 20, 50, 100, 200, 500, 1000];
const NICE_FT = [50, 100, 200, 500, 1000, 2000, 5000];
function renderOverlay() {
  const perUnit = units === "m" ? cam.k : cam.k * 0.3048;
  const options = units === "m" ? NICE_M : NICE_FT;
  const len = options.filter((n) => n * perUnit <= 170).at(-1) ?? options[0];
  const w = len * perUnit;
  const x = 20;
  const y = size.h - 34;
  const blocks = Array.from({ length: 5 }, (_, i) => `<rect class="${i % 2 ? "sb-paper" : "sb-ink"}" x="${r2(x + (w / 5) * i)}" y="${y}" width="${r2(w / 5)}" height="7"/>`).join("");
  el.overlay.innerHTML =
    `<g class="north" transform="translate(24 22)"><path d="M0 -12 L7 10 L0 5 L-7 10 Z"/><text y="26" text-anchor="middle">N</text></g>` +
    `<g class="scale">${blocks}<text x="${x}" y="${y - 6}">0</text><text x="${r2(x + w)}" y="${y - 6}" text-anchor="end">${len} ${units}</text></g>`;
}

// ---- hover and focus ----------------------------------------------------------

let hot = "";
function syncHot() {
  for (const n of el.svg.querySelectorAll(".m-hl.on")) n.classList.remove("on");
  for (const n of el.inspector.querySelectorAll(".place-row.on")) n.classList.remove("on");
  if (!hot) return;
  el.svg.querySelector(`#hl-${hot}`)?.classList.add("on");
  el.inspector.querySelector(`[data-hl="${hot}"]`)?.classList.add("on");
}
const hotOf = (t) => t.closest?.("[data-hl]")?.dataset.hl ?? "";
function setHot(id) {
  if (id === hot) return;
  hot = id;
  syncHot();
}
for (const root of [el.box, el.inspector]) {
  root.addEventListener("pointerover", (e) => setHot(hotOf(e.target)));
  root.addEventListener("pointerleave", () => setHot(""));
  root.addEventListener("focusin", (e) => setHot(hotOf(e.target)));
  root.addEventListener("focusout", () => setHot(""));
}

// ---- panning and zooming ------------------------------------------------------

const pointers = new Map();
let drag = null;
let pinch = null;
let swallowClick = false;

el.box.addEventListener("pointerdown", (e) => {
  if (e.pointerType === "mouse" && e.button !== 0) return;
  pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
  if (pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    pinch = { d: Math.hypot(a.x - b.x, a.y - b.y), k: cam.k };
    drag = null;
  } else if (pointers.size === 1) {
    drag = { id: e.pointerId, x: e.clientX, y: e.clientY, cx: cam.cx, cy: cam.cy, active: false };
  }
});

el.box.addEventListener("pointermove", (e) => {
  if (!pointers.has(e.pointerId)) return;
  pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
  if (pinch && pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    zoomAt((pinch.k * Math.hypot(a.x - b.x, a.y - b.y)) / pinch.d, (a.x + b.x) / 2, (a.y + b.y) / 2);
    swallowClick = true;
    return;
  }
  if (!drag || drag.id !== e.pointerId) return;
  const dx = e.clientX - drag.x;
  const dy = e.clientY - drag.y;
  if (!drag.active && Math.hypot(dx, dy) < 5) return;
  if (!drag.active) {
    drag.active = true;
    el.box.setPointerCapture(e.pointerId);
    el.box.classList.add("panning");
  }
  cam.cx = drag.cx - dx / cam.k;
  cam.cy = drag.cy - dy / cam.k;
  moved = true;
  update();
});

function endPointer(e) {
  pointers.delete(e.pointerId);
  if (drag?.id === e.pointerId) {
    swallowClick = swallowClick || drag.active;
    drag = null;
  }
  if (pointers.size < 2) pinch = null;
  el.box.classList.remove("panning");
}
el.box.addEventListener("pointerup", endPointer);
el.box.addEventListener("pointercancel", endPointer);
// A drag that ends over a place must not open it.
el.box.addEventListener(
  "click",
  (e) => {
    if (!swallowClick) return;
    swallowClick = false;
    e.preventDefault();
    e.stopPropagation();
  },
  true,
);

el.box.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    zoomAt(cam.k * Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015)), e.clientX, e.clientY);
  },
  { passive: false },
);

el.box.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const pan = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
  if (pan) {
    cam.cx += (pan[0] * 80) / cam.k;
    cam.cy += (pan[1] * 80) / cam.k;
    moved = true;
    update();
  } else if (e.key === "+" || e.key === "=") zoomBy(1.4);
  else if (e.key === "-" || e.key === "_") zoomBy(1 / 1.4);
  else if (e.key === "0") {
    fit();
    update();
  } else return;
  e.preventDefault();
});

$("zoom-in").addEventListener("click", () => zoomBy(1.4));
$("zoom-out").addEventListener("click", () => zoomBy(1 / 1.4));
$("zoom-fit").addEventListener("click", () => {
  fit();
  update();
});

new ResizeObserver(() => {
  const r = el.box.getBoundingClientRect();
  if (!r.width || !r.height) return;
  const keep = moved ? { ...cam } : null;
  const before = fitK;
  size = { w: r.width, h: r.height };
  fit();
  if (keep) {
    // Keep the view the person chose, at the same zoom relative to the whole city.
    cam.k = keep.k * (fitK / before);
    cam.cx = keep.cx;
    cam.cy = keep.cy;
    moved = true;
  }
  clampCam();
  renderMap();
  setViewBox();
}).observe(el.box);

// ---- the lists around the map ---------------------------------------------------

function renderKey() {
  const used = [...new Set(view.edges.flatMap((e) => e.pieces.map((p) => p.kind)))].sort((a, b) => KIND_ORDER.indexOf(a) - KIND_ORDER.indexOf(b));
  $("plan-key").innerHTML = used
    .map((k) => `<li><svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect class="k-${k}" width="44" height="22" stroke="none"/><rect width="44" height="22" fill="url(#h-${k})" stroke="none"/></svg>${esc(KIND_NAME[k])}</li>`)
    .join("");
}

function stateTag(p) {
  if (!p.ok) return `<span class="st bad">Needs attention</span>`;
  return p.edited ? `<span class="st">Changed</span>` : "";
}

function renderPlaces() {
  const js = junctions();
  const row = (href, hl, name, sub, p) =>
    `<li><a class="place-row" href="${href}" data-hl="${hl}"><b>${esc(name)}</b><small>${esc(sub)}</small>${stateTag(p)}</a></li>`;
  el.inspector.innerHTML = `
    <div class="insp-head"><div><h2 class="insp-name">Places</h2><p class="insp-sub">${plural(js.length, "junction")}, ${plural(view.edges.length, "street")}</p></div></div>
    <section class="insp-sec"><h3 class="note-h">Junctions</h3><ul class="places">${js.map((n) => row(junctionHref(n), `j-${n.uid}`, n.name, `${n.control}, ${plural(n.arms, "street")}`, n)).join("")}</ul></section>
    <section class="insp-sec"><h3 class="note-h">Streets</h3><ul class="places">${view.edges.map((e) => row(streetHref(e), `s-${e.uid}`, e.kind, `${endsOf(e)} · ${fmt(e.row_mm)}`, e)).join("")}</ul></section>`;
  syncHot();
}

function placeItems(list, cls, icon, detail) {
  return list
    .map(({ href, name, p }) => `<li class="${cls}">${icon}<div><b><a href="${href}">${esc(name)}</a></b><span>${esc(detail(p))}</span></div></li>`)
    .join("");
}

function everyPlace() {
  const all = [
    ...view.nodes.filter((n) => n.junction).map((n) => ({ href: junctionHref(n), name: n.name, p: n })),
    ...view.edges.map((e) => ({ href: streetHref(e), name: `${e.kind}, ${endsOf(e)}`, p: e })),
  ];
  return all;
}

function renderNotes() {
  const all = everyPlace();
  const failing = all.filter(({ p }) => !p.ok);
  const changed = all.filter(({ p }) => p.edited);
  $("checks-lead").textContent = failing.length
    ? `${plural(failing.length, "place needs", "places need")} attention. Open one to see what is wrong and fix it.`
    : `Every check passes in all ${all.length} places.`;
  $("checks").innerHTML = placeItems(failing, "bad", ICON.bad, (p) => p.failing.join("; "));
  $("changes-lead").textContent = changed.length
    ? `${plural(changed.length, "place")} changed from the city as first laid out.`
    : "Nothing changed yet. Open a street or a junction to change it.";
  $("changes").innerHTML = placeItems(changed, "", ICON.changed, (p) => (p.ok ? "Still works" : `Needs attention: ${p.failing.join("; ").toLowerCase()}`));
}

function renderHead() {
  const js = junctions().length;
  $("street-name").textContent = view.name;
  $("city-count").textContent = `${plural(js, "junction")}, ${plural(view.edges.length, "street")}`;
  $("tb-street").textContent = view.name;
  $("tb-places").textContent = String(view.places);
  $("tb-changes").textContent = String(view.edited);
  el.reset.disabled = !view.edited;
  const bad = everyPlace().filter(({ p }) => !p.ok);
  const fit = $("fit");
  fit.className = "fit" + (bad.length ? " bad" : "");
  fit.textContent = bad.length
    ? `${plural(bad.length, "place needs", "places need")} attention: ${bad.slice(0, 3).map(({ name }) => name).join(", ")}${bad.length > 3 ? " and more" : ""}.`
    : `${view.places} places. Every check passes.`;
}

function render() {
  renderHead();
  renderPlaces();
  renderNotes();
  renderKey();
  renderMap();
  setViewBox();
}

// What the editors have written is read again when the page is shown, when
// another tab writes, and when the region changes.
function reload() {
  city = openCity();
  view = JSON.parse(city.view(region));
  render();
}
addEventListener("storage", (e) => {
  if (e.key === "cityloom-city" || e.key === null) reload();
});
addEventListener("pageshow", (e) => {
  if (e.persisted) reload();
});

// ---- start over ---------------------------------------------------------------

// Start over undoes every change in the whole city, and cannot itself be undone,
// so it asks to be pressed twice.
let armed = 0;
const label = $("reset-label");
function disarm() {
  clearTimeout(armed);
  armed = 0;
  label.textContent = "Start over";
}
el.reset.addEventListener("click", () => {
  if (!view.edited) return;
  if (!armed) {
    label.textContent = "Press again to start over";
    armed = setTimeout(disarm, 4000);
    say("This puts every street and junction back as first laid out. Press again to confirm.");
    return;
  }
  disarm();
  resetCity();
  reload();
  say("The city is back as it was first laid out.");
});
el.reset.addEventListener("blur", disarm);

// ---- shell ----------------------------------------------------------------------

initUnits((u) => {
  units = u;
  render();
});
initRegion({
  regions: MATERIALS.regions,
  apply: (i) => {
    region = i;
    return true;
  },
  current: () => MATERIALS.regions[region].id,
  onChange: reload,
  say,
});
initTheme(say);
initDrawingStyle(say, renderMap);
initAccountMenu();
initPanels({ say, detailsWord: "places" });

$("defs").innerHTML = `<defs>${Object.values(HATCH).join("")}</defs>`;

{
  const r = el.box.getBoundingClientRect();
  if (r.width && r.height) size = { w: r.width, h: r.height };
  fit();
  render();
}

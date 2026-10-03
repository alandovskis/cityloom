// Draws the junction the Rust core returns and relays pointer and keyboard
// input. Every rule and every piece of plan geometry comes from the
// WebAssembly model; this file only turns millimetres into pixels.

import init, { Plan, catalogue, junction_catalogue, materials, mount_notes } from "./pkg/cityloom_editor.js";
import { CURB_HATCH, HATCH, MATERIAL_HATCH } from "./symbols.js";
import { NOT_KEPT, keeper, openCity, placeParam, regionIndex, writeCity } from "./city.js";
import { initAccountMenu, initPanels, initRegion, initTheme, initUnits, typing } from "./shell.js";

await init();

const KINDS = JSON.parse(catalogue());
const CAT = JSON.parse(junction_catalogue());
const MATERIALS = JSON.parse(materials());
// Opened from the map (`?junction=3`) the page edits that junction of the city,
// reading its streets from the city, and writes each change back; otherwise it
// is a sandbox on the sample junctions.
const placeId = placeParam("junction");
const city = placeId ? openCity() : null;
const placeName = city?.junction_name(placeId);
const held = city?.junction(placeId, regionIndex(MATERIALS.regions));
if (placeId && !placeName) {
  location.replace("map.html");
  await new Promise(() => {});
}
if (placeId && !held) {
  // The streets here have been changed so that the junction cannot be drawn.
  initAccountMenu();
  initTheme(() => {});
  document.getElementById("street-name").textContent = placeName;
  document.querySelector(".tools").hidden = true;
  document.querySelector(".sheet-body").innerHTML = `<div class="stuck"><h2 class="note-h">This junction cannot be drawn</h2><p>The streets that meet here have been changed so that they no longer make a junction. Give the streets their room back, or start the city over from the map.</p><p><a class="back" href="map.html">City map</a></p></div>`;
  await new Promise(() => {});
}
const plan = held ?? new Plan(0);
let view = JSON.parse(plan.view());
let units = "m";


const $ = (id) => document.getElementById(id);
const el = {
  wrap: $("wrap"),
  palette: $("palette"),
  samples: $("samples"),
  undo: $("undo"),
  redo: $("redo"),
  reset: $("reset"),
  live: $("live"),
};

// ---- formatting -----------------------------------------------------------

const MM_PER_FT = 304.8;
const num = (mm) => (units === "m" ? mm / 1000 : mm / MM_PER_FT);
const fmtN = (mm) => num(mm).toFixed(1);
const fmt = (mm) => `${fmtN(mm)} ${units}`;
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const f1 = (n) => Math.round(n * 10) / 10;
const COMPASS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
const compass = (b) => COMPASS[Math.round(((b % 360) + 360) % 360 / 45) % 8];

const keepSoon = held
  ? keeper(
      () => writeCity((c) => c.keep_junction(placeId, plan)),
      () => say(NOT_KEPT),
    )
  : () => {};
if (held) {
  document.title = `${view.name} · CityLoom`;
  $("street-sub").innerHTML = `<a class="back" href="map.html"><svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>City map</a> <span aria-hidden="true">·</span> <span>Junction plan</span> <span aria-hidden="true">·</span> <span><b id="arm-count" class="fig"></b></span>`;
  document.querySelector(".lower").hidden = true;
  document.querySelector('.surface[href="index.html"]').hidden = true;
}

let liveTimer = 0;
function say(text) {
  clearTimeout(liveTimer);
  el.live.textContent = "";
  liveTimer = setTimeout(() => (el.live.textContent = text), 40);
}

const arm = (uid) => view.arms.find((a) => a.uid === uid);
const corner = (uid) => view.corners.find((c) => c.uid === uid);
const selArm = () => (view.selected.kind ? arm(view.selected.uid) : null);

// ---- icons ----------------------------------------------------------------

const ICON = {
  grip: `<svg class="grip-ico" viewBox="0 0 10 14" aria-hidden="true"><path d="M2 2h.01M8 2h.01M2 7h.01M8 7h.01M2 12h.01M8 12h.01"/></svg>`,
};

const streetSwatch = (s) => {
  let x = 0;
  const w = 44 / s.row_mm;
  const rects = s.pieces
    .map(([id, mm]) => {
      const r = `<rect class="k-${id}" x="${f1(x)}" width="${f1(mm * w)}" height="22" stroke="none"/><rect x="${f1(x)}" width="${f1(mm * w)}" height="22" fill="url(#h-${id})" stroke="none"/>`;
      x += mm * w;
      return r;
    })
    .join("");
  return `<svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">${rects}</svg>`;
};

$("defs").innerHTML = `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}
  <marker id="mv-head" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="mv-tip" d="M1 1 9 5 1 9"/></marker></defs>`;

// ---- notes ----------------------------------------------------------------

function renderNotes() {
  $("tb-changes").textContent = String(view.revisions.length);
}

// A key to the strips: the tint and hatch of each piece that appears in the plan.
function renderKey() {
  const ids = [...new Set(view.arms.flatMap((a) => a.pieces.map((p) => p.kind)))].sort((x, y) => x - y);
  $("plan-key").innerHTML = ids
    .map((k) => `<li><svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect class="k-${KINDS[k].id}" width="44" height="22" stroke="none"/><rect width="44" height="22" fill="url(#h-${KINDS[k].id})" stroke="none"/></svg>${esc(KINDS[k].name)}</li>`)
    .join("");
}

function renderHead() {
  const v = view;
  $("street-name").textContent = v.name;
  $("arm-count").textContent = `${v.arms.length} streets`;
  $("tb-street").textContent = v.name;
  $("tb-row").textContent = String(v.arms.length);
  el.undo.disabled = !v.can_undo;
  el.redo.disabled = !v.can_redo;
  el.reset.disabled = !v.changed;
  const bad = v.checks.filter((c) => !c.ok);
  const fit = $("fit");
  fit.className = "fit" + (bad.length ? " bad" : "");
  const control = CAT.controls[v.control_index].name.toLowerCase();
  fit.textContent = bad.length
    ? `${bad.length === 1 ? "One check needs" : `${bad.length} checks need`} attention: ${bad.map((c) => c.label.toLowerCase()).join(", ")}.`
    : `${v.arms.length} streets, ${control}. Every check passes.`;
}

// ---- add a street, sample junctions ----------------------------------------

function renderPalette() {
  el.palette.innerHTML = CAT.streets
    .map((s, i) => (s.freeway ? "" : `<li><button type="button" class="chip" data-street="${i}">${streetSwatch(s)}<span><b>${esc(s.name)}</b><small>${fmt(s.row_mm)} wide</small></span>${ICON.grip}</button></li>`))
    .join("");
  el.samples.innerHTML = CAT.samples
    .map((s, i) => `<li><button type="button" class="chip plain" data-sample="${i}" aria-pressed="${view.sample === i}"><span></span><span><b>${esc(s.name)}</b><small>${s.arms} streets</small></span></button></li>`)
    .join("");
}

// ---- rendering ------------------------------------------------------------

function render() {
  renderHead();
  renderKey();
  renderNotes();
  for (const b of el.samples.querySelectorAll("[data-sample]")) b.setAttribute("aria-pressed", String(Number(b.dataset.sample) === view.sample));
}

function refresh() {
  view = JSON.parse(plan.view());
  keepSoon();
  render();
}

// The notes are drawn, and the turn table edited, by the page's Rust components;
// what they change is drawn and announced here as any other edit is.
plan.on_edit((ok, what) => {
  refresh();
  if (what === "refresh") return;
  if (what === "select") announceSelection();
  else if (ok) announceEdit();
  else say(plan.refusal());
});
mount_notes(plan);

function announceEdit() {
  const last = view.revisions.at(-1);
  if (last) say(`${last.label}. ${$("fit").textContent}`);
}

function act(fn) {
  const ok = fn();
  refresh();
  if (ok) announceEdit();
  else say(plan.refusal());
  return ok;
}

// ---- selection ------------------------------------------------------------

function select(kind, uid, lane = 0) {
  plan.select(kind ?? "", uid || 0, lane);
  view = JSON.parse(plan.view());
  renderNotes();
  announceSelection();
}

function announceSelection() {
  const s = view.selected;
  if (!s.kind) return;
  const a = arm(s.uid);
  if (s.kind === "bus") return say("Bus lane across the middle");
  if (s.kind === "cycle") return say(`Cycle track, ${fmt(view.ring.cycle_mm)} wide`);
  if (s.kind === "lane") return say(`Lane ${s.lane + 1} of ${a.lanes.length}, ${a.label}`);
  say(s.kind === "arm" ? `${a.label}, ${a.bearing} degrees` : s.kind === "corner" ? `Corner after ${a.label}, ${fmt(corner(s.uid).radius_mm)} radius` : `Crossing on ${a.label}`);
}

// A stepper button or a typed value changes one number, by 0.5 m, 0.1 m or 5°.
function step(key, dir) {
  const s = view.selected;
  const tries = {
    bearing: () => plan.step_bearing(s.uid, dir),
    offset: () => plan.step_offset(s.uid, dir),
    corner: () => plan.step_corner(s.uid, dir),
    setback: () => plan.step_setback(s.uid, dir),
    cwidth: () => plan.step_crossing_width(s.uid, dir),
    ring: () => plan.step_ring(dir),
    cycle: () => plan.step_cycle(dir),
    alen: () => plan.step_approach_len(s.uid, dir),
  };
  act(tries[key]);
}

function removeSelected() {
  const s = view.selected;
  if (s.kind === "bus") return void act(() => plan.set_bus(0, 0));
  if (s.kind === "cycle") return void act(() => plan.set_cycle(0));
  if (s.kind === "crossing") return void act(() => plan.set_crossing(s.uid, false));
  if (s.kind === "arm") act(() => plan.remove_arm(s.uid));
}

// ---- add a street -------------------------------------------------------------

let chip = null;

el.palette.addEventListener("pointerdown", (e) => {
  const b = e.target.closest("[data-street]");
  if (!b || e.button !== 0) return;
  chip = { street: Number(b.dataset.street), x: e.clientX, y: e.clientY, active: false, ghost: null, name: CAT.streets[Number(b.dataset.street)].name };
  el.palette.setPointerCapture(e.pointerId);
});

el.palette.addEventListener("pointermove", (e) => {
  if (!chip) return;
  if (!chip.active && Math.hypot(e.clientX - chip.x, e.clientY - chip.y) > 6) {
    chip.active = true;
    const g = document.createElement("div");
    g.className = "drag-chip";
    g.textContent = chip.name;
    document.body.append(g);
    chip.ghost = g;
  }
  if (chip.active) {
    chip.ghost.style.left = `${e.clientX}px`;
    chip.ghost.style.top = `${e.clientY}px`;
  }
});

function endChip(commit, e) {
  if (!chip) return;
  const c = chip;
  chip = null;
  c.ghost?.remove();
  if (!c.active) return;
  const r = $("drawing").getBoundingClientRect();
  const over = e && e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
  if (commit && over) {
    act(() => plan.add_arm_toward(c.street, ...plan.point_at(e.clientX, e.clientY)) !== 0);
  }
}
el.palette.addEventListener("pointerup", (e) => endChip(true, e));
el.palette.addEventListener("pointercancel", (e) => endChip(false, e));

el.palette.addEventListener("click", (e) => {
  const b = e.target.closest("[data-street]");
  if (!b || e.detail > 1 || chipDidDrag) return;
  act(() => plan.add_arm(Number(b.dataset.street), -1) !== 0);
});
let chipDidDrag = false;
el.palette.addEventListener("pointerdown", () => (chipDidDrag = false));
el.palette.addEventListener("pointermove", () => {
  if (chip?.active) chipDidDrag = true;
});

el.samples.addEventListener("click", (e) => {
  const b = e.target.closest("[data-sample]");
  if (!b) return;
  plan.load_sample(Number(b.dataset.sample));
  refresh();
  say(`${view.name}. ${$("fit").textContent}`);
});

// ---- keyboard -------------------------------------------------------------------

el.wrap.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const s = view.selected;
  if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
    const dir = e.key === "ArrowRight" ? 1 : -1;
    e.preventDefault();
    if (e.shiftKey && s.kind === "arm") return void step("bearing", dir);
    plan.select_relative(dir);
    view = JSON.parse(plan.view());
        renderNotes();
    announceSelection();
  } else if ((e.key === "+" || e.key === "=" || e.key === "-" || e.key === "_") && ["arm", "corner", "crossing", "cycle"].includes(s.kind)) {
    e.preventDefault();
    const dir = e.key === "+" || e.key === "=" ? 1 : -1;
    step({ arm: "bearing", corner: "corner", crossing: "setback", cycle: "cycle" }[s.kind], dir);
  } else if (e.key === "Delete" || e.key === "Backspace") {
    e.preventDefault();
    removeSelected();
  } else if (e.key === "Escape" && s.kind) {
    select(null, 0);
  }
});

function undo() {
  if (plan.undo()) {
    refresh();
    say("Undone.");
  }
}
function redo() {
  if (plan.redo()) {
    refresh();
    announceEdit();
  }
}

document.addEventListener("keydown", (e) => {
  if (!(e.metaKey || e.ctrlKey) || typing(e.target)) return;
  const k = e.key.toLowerCase();
  if (k === "z") {
    e.preventDefault();
    e.shiftKey ? redo() : undo();
  } else if (k === "y") {
    e.preventDefault();
    redo();
  }
});

// ---- header controls -------------------------------------------------------------

el.undo.addEventListener("click", undo);
el.redo.addEventListener("click", redo);
el.reset.addEventListener("click", () => {
  if (plan.reset()) {
    refresh();
    say("Started over from the junction as it is today.");
  }
});
initUnits((u) => {
  units = u;
  plan.set_units(u);
  renderPalette();
  render();
});
initRegion({
  regions: MATERIALS.regions,
  apply: (i) => plan.set_region(i),
  current: () => view.region,
  onChange: refresh,
  say,
});
initTheme(say);


renderPalette();
render();

initAccountMenu();
initPanels({ say, detailsWord: "details" });

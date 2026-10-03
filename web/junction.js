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
const LIM = CAT.limits;
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

const CLASS_WORD = { left: "left", through: "straight on", right: "right" };
const CLASS_NAME = { left: "Left", through: "Straight on", right: "Right" };

const $ = (id) => document.getElementById(id);
const el = {
  svg: $("drawing"),
  wrap: $("wrap"),
  scroll: $("scroll"),
  palette: $("palette"),
  samples: $("samples"),
  inspector: $("inspector"),
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
const fromInput = (v) => Math.round(units === "m" ? v * 1000 : v * MM_PER_FT);
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
  minus: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8"/></svg>`,
  plus: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8M7 3v8"/></svg>`,
  remove: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 3l8 8M11 3l-8 8"/></svg>`,
  tick: `<svg class="tick" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg>`,
  grip: `<svg class="grip-ico" viewBox="0 0 10 14" aria-hidden="true"><path d="M2 2h.01M8 2h.01M2 7h.01M8 7h.01M2 12h.01M8 12h.01"/></svg>`,
};

const BTN = {
  plus: `<svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3 8h10M8 3v10"/></svg>`,
  remove: `<svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9"/></svg>`,
};

// A turn arrow, pointing up, in a 16 by 16 box: the stem and its branch.
const turnGlyph = (cls, size = 16) => {
  const branch = { left: "M8 9Q8 5 3.5 5M6 3 3.5 5 6 7", right: "M8 9Q8 5 12.5 5M10 3l2.5 2L10 7", through: "M8 9V2M5.5 4.5 8 2l2.5 2.5" }[cls];
  return `<svg class="turn-ico" viewBox="0 0 16 16" width="${size}" height="${size}" aria-hidden="true" focusable="false"><path d="M8 14V9"/><path d="${branch}"/></svg>`;
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

// ---- plan -----------------------------------------------------------------

const S = { s: 1, ox: 0, oy: 0, w: 0, h: 0 };
const T = (p) => [S.ox + p[0] * S.s, S.oy + p[1] * S.s];
const pt = (p) => T(p).map(f1).join(" ");
const mmOf = (e) => {
  const r = el.svg.getBoundingClientRect();
  return [(e.clientX - r.left - S.ox) / S.s, (e.clientY - r.top - S.oy) / S.s];
};
const dirOf = (b) => [Math.sin((b * Math.PI) / 180), -Math.cos((b * Math.PI) / 180)];

function pathD(cmds) {
  return cmds
    .map((c) => {
      switch (c[0]) {
        case "M":
        case "L":
          return `${c[0]}${pt([c[1], c[2]])}`;
        case "A":
          return `A${f1(c[1] * S.s)} ${f1(c[1] * S.s)} 0 ${c[2]} ${c[3]} ${pt([c[4], c[5]])}`;
        case "Q":
          return `Q${pt([c[1], c[2]])} ${pt([c[3], c[4]])}`;
        default:
          return "Z";
      }
    })
    .join("");
}

const hatchFor = (kindId) => `url(#h-${kindId})`;
const sel = () => view.selected;
const isSel = (kind, uid) => sel().kind === kind && sel().uid === uid;
const isLane = (uid, i) => sel().kind === "lane" && sel().uid === uid && sel().lane === i;

// A turn arrow lying on a lane, pointing along the lane's travel.
function laneArrow(l, size) {
  const uses = l.uses;
  const h = size;
  const stem = uses.includes("through") ? -h / 2 : 0;
  const bw = h * 0.42;
  const parts = [`M0 ${h / 2}V${stem}`];
  if (uses.includes("through")) parts.push(`M${-h * 0.18} ${-h / 2 + h * 0.2}L0 ${-h / 2}L${h * 0.18} ${-h / 2 + h * 0.2}`);
  if (uses.includes("left")) parts.push(`M0 ${-h * 0.05}Q0 ${-h * 0.3} ${-bw} ${-h * 0.3}M${-bw + h * 0.16} ${-h * 0.3 - h * 0.16}L${-bw} ${-h * 0.3}L${-bw + h * 0.16} ${-h * 0.3 + h * 0.16}`);
  if (uses.includes("right")) parts.push(`M0 ${-h * 0.05}Q0 ${-h * 0.3} ${bw} ${-h * 0.3}M${bw - h * 0.16} ${-h * 0.3 - h * 0.16}L${bw} ${-h * 0.3}L${bw - h * 0.16} ${-h * 0.3 + h * 0.16}`);
  const [x, y] = T(l.at);
  const d = parts.join("");
  return `<g class="lane-arrow${l.bad ? " bad" : ""}" transform="translate(${f1(x)} ${f1(y)}) rotate(${l.heading})"><path class="halo" d="${d}"/><path d="${d}"/></g>`;
}

function grip(x, y, angle, role, uid, label) {
  return (
    `<g class="handle" data-role="${role}" data-uid="${uid}" transform="translate(${f1(x)} ${f1(y)}) rotate(${f1(angle)})">` +
    `<title>${esc(label)}</title><rect class="hit" x="-16" y="-15" width="32" height="30"/>` +
    `<rect class="grip" x="-10" y="-8" width="20" height="16" rx="2"/>` +
    `<path class="grip-arrow" d="M-6 0H6M-3 -3 -6 0l3 3M3 -3 6 0 3 3"/></g>`
  );
}

function controlMarker(a) {
  if (a.role === "free" || !a.stop_line) return "";
  const [p0, p1] = a.stop_line.map(T);
  const mid = [(p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2];
  // The end of the stop line nearest the kerb is the one farthest from the axis.
  const ax = T(a.mouth_at);
  const far = Math.hypot(p0[0] - ax[0], p0[1] - ax[1]) > Math.hypot(p1[0] - ax[0], p1[1] - ax[1]) ? p0 : p1;
  const d = Math.hypot(far[0] - mid[0], far[1] - mid[1]) || 1;
  const out = [(far[0] - mid[0]) / d, (far[1] - mid[1]) / d];
  const c = [far[0] + out[0] * 15, far[1] + out[1] * 15];
  const line = `<path class="stop-line${a.role === "yield" ? " yield" : ""}" d="M${f1(p0[0])} ${f1(p0[1])}L${f1(p1[0])} ${f1(p1[1])}"/>`;
  const glyph = {
    signal: `<g class="sign" transform="translate(${f1(c[0])} ${f1(c[1])}) rotate(${a.bearing})"><rect x="-4.5" y="-10" width="9" height="20" rx="2"/><circle cx="0" cy="-5" r="1.7"/><circle cx="0" cy="0" r="1.7"/><circle cx="0" cy="5" r="1.7"/></g>`,
    stop: `<g class="sign" transform="translate(${f1(c[0])} ${f1(c[1])})"><path d="M-3.5 -8.5h7l5 5v7l-5 5h-7l-5 -5v-7z"/><path d="M-4 0h8"/></g>`,
    yield: `<g class="sign" transform="translate(${f1(c[0])} ${f1(c[1])}) rotate(${a.bearing})"><path d="M-8 -7H8L0 8z"/></g>`,
  }[a.role];
  return line + glyph;
}

// The distance across as a dimension string with ticks, on the junction side
// of the crossing where nothing else is drawn, its figure over a paper halo.
function crossingDim(a) {
  const c = a.crossing;
  const n = dirOf(a.bearing);
  const off = -800;
  const [q0, q1] = [0, 1].map((i) => T([c.poly[i][1] + n[0] * off, c.poly[i][2] + n[1] * off]));
  const ang = Math.atan2(q1[1] - q0[1], q1[0] - q0[0]);
  const tk = [Math.cos(ang + 0.785) * 5, Math.sin(ang + 0.785) * 5];
  const tick = ([x, y]) => `M${f1(x - tk[0])} ${f1(y - tk[1])}L${f1(x + tk[0])} ${f1(y + tk[1])}`;
  const text = c.stages > 1 ? `${c.stages} × ${fmtN(c.stage_mm)}` : fmtN(c.distance_mm);
  return (
    `<path class="dim${c.too_far ? " dim-warn" : ""}" d="M${f1(q0[0])} ${f1(q0[1])}L${f1(q1[0])} ${f1(q1[1])}${tick(q0)}${tick(q1)}"/>` +
    `<text class="t-dim t-halo${c.too_far ? " t-warn" : ""}" x="${f1((q0[0] + q1[0]) / 2)}" y="${f1((q0[1] + q1[1]) / 2 + 5)}" text-anchor="middle">${text}</text>`
  );
}

// A code tag, as the Atlas names the measure, on a paper halo.
const codeTag = (p, code) => {
  const [x, y] = T(p);
  return `<text class="t-note t-halo measure-tag" x="${f1(x)}" y="${f1(y + 4)}" text-anchor="middle">${code}</text>`;
};

// The transit priority measures on one arm: gates, stops, filters, caps and
// their Atlas codes. The bus lane and queue jumps are drawn with the arm itself.
function measureMarks(a) {
  const t = a.transit;
  const out = [];
  if (t.virtual_loop) out.push(`<path class="measure-loop" d="${pathD(t.virtual_loop)}"/>`);
  if (t.gate) {
    const [p0, p1] = t.gate.map(T);
    out.push(`<path class="stop-line${t.gate_yields ? " yield" : ""}" d="M${f1(p0[0])} ${f1(p0[1])}L${f1(p1[0])} ${f1(p1[1])}"/>`);
  }
  if (t.stop_poly) {
    const d = pathD(t.stop_poly);
    out.push(`<path class="bulb measure-stop k-sidewalk" d="${d}"/><path class="hatch" fill="${hatchFor("sidewalk")}" d="${d}"/>`);
  }
  for (const b of t.bollards) {
    const [x, y] = T(b);
    out.push(`<circle class="bollard" cx="${f1(x)}" cy="${f1(y)}" r="3.2"/>`);
  }
  if (t.cap) {
    const [p0, p1] = t.cap.map(T);
    out.push(`<path class="dead-cap" d="M${f1(p0[0])} ${f1(p0[1])}L${f1(p1[0])} ${f1(p1[1])}"/>`);
  }
  if (t.island) {
    const d = pathD(t.island);
    out.push(`<path class="island k-sidewalk" d="${d}"/><path class="hatch" fill="${hatchFor("sidewalk")}" d="${d}"/>`);
  }
  for (const g of t.tags) out.push(codeTag(g.at, g.code));
  return out.join("");
}

function renderPlan() {
  const v = view;
  const W = Math.max(el.wrap.clientWidth, 320);
  const narrow = W < 640;
  const H = Math.round(narrow ? Math.min(W * 1.5, 620) : Math.min(Math.max(window.innerHeight - 240, 520), 820));
  const [bx0, by0, bx1, by1] = v.bounds;
  const padX = narrow ? 24 : Math.min(150, W * 0.2);
  const padY = 64;
  S.s = Math.min((W - 2 * padX) / (bx1 - bx0), (H - 2 * padY) / (by1 - by0));
  S.ox = W / 2 - ((bx0 + bx1) / 2) * S.s;
  S.oy = H / 2 - ((by0 + by1) / 2) * S.s;
  S.w = W;
  S.h = H;
  el.svg.setAttribute("width", W);
  el.svg.setAttribute("height", H);
  el.svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  const zebra = Math.max(4, f1(600 * S.s));

  const defs = v.arms
    .filter((a) => a.crossing)
    .map((a) => `<pattern id="zb-${a.uid}" width="${zebra * 2}" height="${zebra * 2}" patternUnits="userSpaceOnUse" patternTransform="rotate(${a.bearing})"><rect class="zebra" width="${zebra}" height="${zebra * 2}"/></pattern>`)
    .join("");

  const layers = { wedge: [], arm: [], lane: [], measure: [], road: [], bulb: [], curb: [], cross: [], mark: [], sel: [], grip: [], move: [], label: [] };

  // pavement wedges between arms; pressing one selects the corner
  for (const c of v.corners) {
    layers.wedge.push(
      `<g class="wedge-g" data-role="${c.straight ? "none" : "corner"}" data-uid="${c.uid}"><path class="wedge k-sidewalk" d="${pathD(c.wedge)}"/><path class="hatch" fill="${hatchFor("sidewalk")}" d="${pathD(c.wedge)}"/></g>`,
    );
  }

  for (const a of v.arms) {
    const g = [];
    for (const p of a.pieces) {
      const k = KINDS[p.kind];
      g.push(`<path class="piece k-${k.id}" d="${pathD(p.poly)}"/><path class="hatch" fill="${hatchFor(k.id)}" d="${pathD(p.poly)}"/>`);
    }
    const tr = a.transit;
    if (tr.bus) g.push(`<path class="piece k-bus" d="${pathD(tr.bus)}"/><path class="hatch" fill="${hatchFor("bus")}" d="${pathD(tr.bus)}"/>`);
    if (tr.queue) g.push(`<path class="piece k-bus queue" d="${pathD(tr.queue)}"/><path class="hatch" fill="${hatchFor("bus")}" d="${pathD(tr.queue)}"/>`);
    layers.arm.push(`<g class="arm${isSel("arm", a.uid) ? " on" : ""}${tr.dead ? " dead" : ""}" data-role="arm" data-uid="${a.uid}">${g.join("")}</g>`);
    layers.measure.push(measureMarks(a));
    a.lanes.forEach((l, i) => {
      layers.lane.push(`<path class="lane-hit${isLane(a.uid, i) ? " on" : ""}" data-role="lane" data-uid="${a.uid}" data-lane="${i}" d="${pathD(l.poly)}"><title>Lane ${i + 1} of ${a.lanes.length}, ${esc(a.label)}</title></path>`);
    });
    for (const gp of a.gaps) layers.road.push(`<path class="road" d="${pathD(gp)}"/>`);
    a.bulbs.forEach((b, i) => {
      if (b) layers.bulb.push(`<g data-role="arm" data-uid="${a.uid}"><path class="bulb k-sidewalk" d="${pathD(b)}"/><path class="hatch" fill="${hatchFor("sidewalk")}" d="${pathD(b)}"/></g>`);
      void i;
    });
  }

  // the carriageway where the streets meet
  if (v.ring) {
    const r = v.ring.road_mm * S.s;
    const ro = v.ring.radius_mm * S.s;
    const [cx, cy] = T([0, 0]);
    const ri = v.ring.island_mm * S.s;
    layers.road.push(`<circle class="road ring" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(r)}"/>`);
    if (v.ring.cycle_mm) {
      const circle = (rad) => `M${f1(cx - rad)} ${f1(cy)}a${f1(rad)} ${f1(rad)} 0 1 0 ${f1(2 * rad)} 0a${f1(rad)} ${f1(rad)} 0 1 0 ${f1(-2 * rad)} 0Z`;
      const band = circle(ro) + circle(r);
      layers.road.push(`<g class="cycle-g" data-role="cycle"><path class="piece k-bike cycle-ring" fill-rule="evenodd" d="${band}"/><path class="hatch" fill-rule="evenodd" fill="${hatchFor("bike")}" d="${band}"/></g>`);
      if (sel().kind === "cycle") layers.sel.push(`<circle class="sel-line" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(ro)}"/><circle class="sel-line" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(r)}"/>`);
    }
    layers.road.push(`<circle class="island k-median" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(ri)}"/><circle class="hatch" fill="${hatchFor("median")}" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(ri)}"/>`);
    if (v.bus) {
      const d = pathD(v.bus.poly);
      const [bx, by] = T([0, 0]);
      layers.road.push(`<g class="bus-g" data-role="bus"><path class="piece k-bus bus-lane" d="${d}"/><path class="hatch" fill="${hatchFor("bus")}" d="${d}"/></g>`);
      if (sel().kind === "bus") layers.sel.push(`<path class="sel-box" d="${d}"/>`);
      layers.label.push(`<text class="t-mark t-halo" x="${f1(bx)}" y="${f1(by + 5)}" text-anchor="middle">Bus only</text>`);
    }
    // circulation arrows on the ring, between the streets
    const mid = (v.ring.road_mm - 3000) * S.s;
    const ccw = v.ring.circulation === "anticlockwise";
    const bs = v.arms.map((a) => a.bearing);
    bs.forEach((b, i) => {
      const nb = bs[(i + 1) % bs.length];
      const gapDeg = (nb - b + 360) % 360 || 360;
      const ang = b + gapDeg / 2;
      const [dx, dy] = dirOf(ang);
      const heading = ang + (ccw ? -90 : 90);
      const h = Math.min(20, mid * 0.5);
      layers.mark.push(
        `<g class="lane-arrow" transform="translate(${f1(cx + dx * mid)} ${f1(cy + dy * mid)}) rotate(${f1(heading)})"><path class="halo" d="M0 ${h / 2}V${-h / 2}M-4 ${-h / 2 + 4}L0 ${-h / 2}L4 ${-h / 2 + 4}"/><path d="M0 ${h / 2}V${-h / 2}M-4 ${-h / 2 + 4}L0 ${-h / 2}L4 ${-h / 2 + 4}"/></g>`,
      );
    });
  } else {
    layers.road.push(`<path class="road" d="${pathD(v.core)}"/>`);
  }

  for (const c of v.corners) {
    const bad = !c.ok || c.fast;
    layers.curb.push(`<path class="curb-line${bad ? " bad" : ""}" d="${pathD(c.curb)}"/>`);
  }

  for (const a of v.arms) {
    if (a.crossing) {
      const on = isSel("crossing", a.uid);
      layers.cross.push(
        `<g data-role="crossing" data-uid="${a.uid}"><path class="crossing" d="${pathD(a.crossing.poly)}"/><path fill="url(#zb-${a.uid})" class="zebra-fill" d="${pathD(a.crossing.poly)}"/></g>`,
      );
      if (a.crossing.island_poly) {
        layers.cross.push(`<g data-role="crossing" data-uid="${a.uid}"><path class="island k-sidewalk" d="${pathD(a.crossing.island_poly)}"/><path class="hatch" fill="${hatchFor("sidewalk")}" d="${pathD(a.crossing.island_poly)}"/></g>`);
      }
      if (on) layers.sel.push(`<path class="sel-box" d="${pathD(a.crossing.poly)}"/>`);
      layers.label.push(crossingDim(a));
    }
    layers.mark.push(controlMarker(a));
    for (const l of a.lanes) layers.mark.push(laneArrow(l, Math.max(16, Math.min(34, 2600 * S.s))));
    for (const l of a.leave_arrows) {
      const [x, y] = T(l.at);
      const h = Math.max(12, Math.min(22, 1800 * S.s));
      layers.mark.push(`<g class="lane-arrow out" transform="translate(${f1(x)} ${f1(y)}) rotate(${l.heading})"><path class="halo" d="M0 ${h / 2}V${-h / 2}M-3.5 ${-h / 2 + 3.5}L0 ${-h / 2}L3.5 ${-h / 2 + 3.5}"/><path d="M0 ${h / 2}V${-h / 2}M-3.5 ${-h / 2 + 3.5}L0 ${-h / 2}L3.5 ${-h / 2 + 3.5}"/></g>`);
    }

    // the street's name and width, outside the end of the arm
    const [lx, ly] = T(a.end);
    const ld = dirOf(a.bearing);
    const side = Math.abs(ld[0]) > 0.5;
    // On a narrow screen there is no room beside a sideways arm: set its name above it.
    const beside = side && !narrow;
    const anchor = side ? (ld[0] > 0 ? (narrow ? "end" : "start") : narrow ? "start" : "end") : "middle";
    const x = beside ? lx + ld[0] * 12 : lx;
    const y1 = beside ? ly - 2 : ld[1] < 0 || side ? ly - 32 : ly + 26;
    layers.label.push(
      `<text class="t-mark t-halo${isSel("arm", a.uid) ? " t-blue" : ""}" x="${f1(x)}" y="${f1(y1)}" text-anchor="${anchor}">${esc(a.street)}</text>` +
        `<text class="t-note t-halo t-soft" x="${f1(x)}" y="${f1(y1 + 17)}" text-anchor="${anchor}">${compass(a.bearing)} · ${fmt(a.road_mm)} road${a.offset_mm ? ` · shifted ${fmt(Math.abs(a.offset_mm))}` : ""}</text>`,
    );

    a.lanes.forEach((l, i) => {
      if (isLane(a.uid, i)) layers.sel.push(`<path class="sel-box" d="${pathD(l.poly)}"/>`);
    });
    if (isSel("arm", a.uid)) layers.sel.push(`<path class="sel-box" d="${pathD(a.outline)}"/>`);
    // the end grip: always there, since turning a street is the main move
    const [ex, ey] = T(a.end);
    const inward = dirOf(a.bearing);
    layers.grip.push(grip(ex - inward[0] * 20, ey - inward[1] * 20, a.bearing, "grip-arm", a.uid, `Turn ${a.label}`));
  }

  // grips and the outline of a selected corner or crossing
  for (const c of v.corners) {
    if (!isSel("corner", c.uid) || c.straight) continue;
    layers.sel.push(`<path class="sel-line" d="${pathD(c.curb)}"/>`);
    const [gx, gy] = T(c.handle);
    layers.grip.push(grip(gx, gy, (Math.atan2(c.bisector[1], c.bisector[0]) * 180) / Math.PI, "grip-corner", c.uid, "Change the corner radius"));
  }
  for (const a of v.arms) {
    if (!a.crossing || !isSel("crossing", a.uid)) continue;
    const [gx, gy] = T(a.crossing.handle);
    layers.grip.push(grip(gx, gy, a.bearing - 90, "grip-crossing", a.uid, "Move the crossing"));
  }

  // the turns of a selected street
  const sa = selArm();
  if (sa) {
    for (const m of v.movements.filter((m) => m.from === sa.uid)) {
      const d = pathD(m.path);
      layers.move.push(`<path class="mv${m.allowed ? "" : " no"}${m.allowed && !m.lane ? " bad" : ""}${m.indirect ? " indirect" : ""}" d="${d}"${m.allowed ? ' marker-end="url(#mv-head)"' : ""}><title>${esc(m.blocked ?? "")}</title></path>`);
      if (!m.allowed) {
        const a = m.path[0];
        const b = m.path[m.path.length - 1];
        const [x, y] = T([(a[1] + b[b.length - 2]) / 2, (a[2] + b[b.length - 1]) / 2]);
        layers.move.push(`<path class="mv-x" d="M${f1(x - 5)} ${f1(y - 5)}l10 10m0 -10l-10 10"/>`);
      }
    }
  }

  // north mark and scale bar
  const nx = 34;
  const ny = 44;
  const furniture =
    `<g class="north" transform="translate(${nx} ${ny})"><circle r="15"/><path d="M0 11V-9M-4 -4 0 -10l4 6"/><text class="t-label" y="-20" text-anchor="middle">N</text></g>` +
    scaleBar(20, H - 30);

  el.svg.innerHTML =
    `<defs>${defs}</defs>` +
    `<g class="plan">` +
    layers.wedge.join("") +
    layers.arm.join("") +
    layers.lane.join("") +
    layers.measure.join("") +
    layers.road.join("") +
    layers.bulb.join("") +
    layers.curb.join("") +
    layers.cross.join("") +
    layers.mark.join("") +
    layers.sel.join("") +
    layers.move.join("") +
    layers.label.join("") +
    layers.grip.join("") +
    `</g>` +
    furniture;
  el.svg.setAttribute("aria-label", `Plan of the junction, north up. ${v.arms.length} streets. ${v.control === "roundabout" ? "Roundabout." : ""}`);
}

function scaleBar(x, y) {
  const block = 4000 * S.s;
  const blocks = Array.from({ length: 5 }, (_, i) => `<rect class="${i % 2 ? "bar-w" : "bar-b"}" x="${f1(x + 36 + i * block)}" y="${y - 6}" width="${f1(block)}" height="6"/>`).join("");
  return (
    `<g class="scale"><text class="t-label" x="${x}" y="${y}">Scale</text>${blocks}` +
    `<text class="t-dim" x="${f1(x + 36)}" y="${y + 20}" text-anchor="middle">0</text>` +
    `<text class="t-dim" x="${f1(x + 36 + 5 * block)}" y="${y + 20}" text-anchor="middle">${units === "m" ? "20 m" : `${Math.round(20000 / MM_PER_FT)} ft`}</text></g>`
  );
}

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

// ---- inspector ------------------------------------------------------------

const option = (fid, checked, glyph, name, data, role = "radio") =>
  `<li><button type="button" class="opt" role="${role}" aria-checked="${checked}" tabindex="${role === "radio" ? (checked ? 0 : -1) : 0}" data-ifid="${fid}" ${data}>${glyph}<span>${esc(name)}</span>${ICON.tick}</button></li>`;

function numField(key, label, mm, opts = {}) {
  const { unit = units, value = num(mm).toFixed(units === "m" ? 1 : 1), minus, plus, hint = "", step = units === "m" ? "0.1" : "0.5", min = "", max = "", tag = unit } = opts;
  const id = `i-h-${key}`;
  return `<section class="insp-sec"><h3 class="note-h" id="${id}">${label}</h3>
    <div class="stepper">
      <button type="button" class="ico" data-istep="${key}" data-dir="-1" data-ifid="${key}-minus" aria-label="${esc(minus)}">${ICON.minus}</button>
      <span class="wfield"><input type="number" inputmode="decimal" data-iset="${key}" data-ifid="${key}" step="${step}" min="${min}" max="${max}" value="${value}" aria-labelledby="${id}"><span class="unit-tag" aria-hidden="true">${tag}</span></span>
      <button type="button" class="ico" data-istep="${key}" data-dir="1" data-ifid="${key}-plus" aria-label="${esc(plus)}">${ICON.plus}</button>
    </div>${hint ? `<p class="insp-range">${hint}</p>` : ""}</section>`;
}

function controlSection() {
  const v = view;
  const opts = CAT.controls.map((c, i) => `<option value="${i}"${i === v.control_index ? " selected" : ""}>${esc(c.name)}</option>`).join("");
  let ring = "";
  if (v.ring) {
    ring = numField("ring", "Size of the roundabout", v.ring.radius_mm, {
      minus: "Smaller by 0.5 m",
      plus: "Larger by 0.5 m",
      value: num(v.ring.radius_mm * 2).toFixed(1),
      hint: `Across the outside. No smaller than ${fmt(v.ring.floor_mm * 2)} with these streets.`,
      tag: `${units} across`,
    });
  }
  let cycle = "";
  if (v.ring) {
    cycle = `<section class="insp-sec"><h3 class="note-h" id="i-h-cycle">Cycle track</h3><ul class="opts">${option("cycle-on", !!v.ring.cycle_mm, "", "Track around the outside", `data-icycle="1"`, "checkbox")}</ul></section>`;
    if (v.ring.cycle_mm) {
      cycle += numField("cycle", "Track width", v.ring.cycle_mm, { minus: "Narrower by 0.5 m", plus: "Wider by 0.5 m", hint: `${fmt(LIM.cycle[0])} to ${fmt(LIM.cycle[1])}. It takes space from the carriageway inside the same circle.` });
    }
  }
  let bus = "";
  if (v.ring) {
    const cur = v.bus ? `${v.bus.from}-${v.bus.to}` : "";
    const pairs = v.bus_options.map((o) => `<option value="${o.a}-${o.b}"${`${o.a}-${o.b}` === cur ? " selected" : ""}>${esc(o.label)}</option>`).join("");
    bus = `<section class="insp-sec"><h3 class="note-h" id="i-h-bus">Bus lane through the middle</h3><select data-ibus data-ifid="bus" aria-labelledby="i-h-bus"><option value=""${cur ? "" : " selected"}>No bus lane</option>${pairs}</select><p class="insp-range">${v.bus ? `A ${fmt(v.bus.width_mm)} bus-only lane straight across the island. It crosses the ring where it enters and leaves.` : "Lets buses cut across the island between two streets."}</p></section>`;
  }
  return `<section class="insp-sec"><h3 class="note-h" id="i-h-control">Junction control</h3><select data-icontrol data-ifid="control" aria-labelledby="i-h-control">${opts}</select></section>${ring}${cycle}${bus}`;
}

function renderJunctionPanel() {
  el.inspector.innerHTML = `<p class="insp-empty">Select a street, corner or crossing to change it.</p>${controlSection()}`;
}

const laneNote = (a, i) => (a.lanes.length === 1 ? "the only lane" : i === 0 ? "nearest the middle" : i === a.lanes.length - 1 ? "nearest the curb" : "");

function laneRows(a) {
  return a.lanes
    .map((l, i) => {
      const btns = l.dests
        .map((d) => {
          const bad = l.bad && d.on;
          return `<button type="button" class="turn dest${d.on ? " on" : ""}${bad ? " bad" : ""}" data-lane="${i}" data-to="${d.uid}" data-ifid="ln-${i}-${d.uid}" aria-pressed="${d.on}" ${d.open ? "" : "disabled"} title="${esc(d.label)}" aria-label="Lane ${i + 1} to ${esc(d.label)}, ${CLASS_WORD[d.class]}">${turnGlyph(d.class)}<span aria-hidden="true">${compass(arm(d.uid).bearing)}</span></button>`;
        })
        .join("");
      return `<li class="lane-row${l.bad ? " bad" : ""}"><button type="button" class="lane-pick" data-pick="${i}" data-ifid="pick-${i}"><span>Lane ${i + 1}<small>${laneNote(a, i)}</small></span></button><span class="turns-btns">${btns}</span></li>`;
    })
    .join("");
}

// The transit priority measures of the Atlas that sit on one approach.
function transitSection(a) {
  const t = a.transit;
  const sel = (key, list, cur, label) =>
    `<label class="fld"><span id="i-h-${key}">${label}</span><select data-i${key} data-ifid="${key}" aria-labelledby="i-h-${key}">${CAT[list]
      .map((o, i) => `<option value="${i}"${i === cur ? " selected" : ""}>${o.code ? `${o.code} ` : ""}${esc(o.name)}</option>`)
      .join("")}</select></label>`;
  const check = (fid, on, name, data) => `<ul class="opts">${option(fid, on, "", name, data, "checkbox")}</ul>`;
  const lenField =
    t.approach_kind === "queue" || t.approach_kind === "gate"
      ? numField("alen", t.approach_kind === "queue" ? "Length of the queue jump" : "Gate distance upstream", t.approach_mm, {
          minus: "Shorter by 5 m",
          plus: "Longer by 5 m",
          hint: `${fmt(LIM.approach[0])} to ${fmt(LIM.approach[1])}`,
        })
      : "";
  const problems = t.problems.length ? `<ul class="problems">${t.problems.map((p) => `<li>${esc(p)}</li>`).join("")}</ul>` : "";
  return `<section class="insp-sec"><h3 class="note-h" id="i-h-transit">Transit priority</h3>
    ${check("buslane", t.bus_lane, "Bus lane along the way in", 'data-ibuslane="1"')}
    ${sel("approach", "approaches", t.approach, "At the approach")}
    ${sel("stop", "stops", t.stop, "Bus stop")}
    ${sel("rule", "rules", t.rule, "Turns")}
    ${check("filter", t.filter, "N1 Transit modal filter", 'data-ifilter="1"')}
    ${problems}</section>${lenField}`;
}

function crossingSection(a) {
  const c = a.crossing;
  if (!c) {
    return `<section class="insp-sec"><h3 class="note-h" id="i-h-cross">Crossing</h3>
      <button type="button" class="btn" data-icross="1" data-ifid="cross-add">${BTN.plus}Mark a crossing</button></section>`;
  }
  const bulbs = [0, 1]
    .map((i) => {
      const has = a.can_bulb[i];
      return option(`bulb-${i}`, a.bulbs[i] !== null, "", `${i === 0 ? "Left" : "Right"} curb bulge${has ? "" : " (no parking there)"}`, `data-ibulb="${i}" ${has ? "" : "disabled"}`, "checkbox");
    })
    .join("");
  return (
    `<section class="insp-sec"><h3 class="note-h" id="i-h-cross">Crossing</h3>
      <p class="insp-range">${c.stages > 1 ? `${c.stages} stages of ${fmt(c.stage_mm)}` : `${fmt(c.distance_mm)} to cross`}${c.too_far ? ". Too far in one go." : ""}</p>
      <button type="button" class="btn" data-icross="0" data-ifid="cross-off">${BTN.remove}Remove the crossing</button></section>` +
    numField("setback", "Set back from the junction", c.setback_mm, { minus: "Closer by 0.5 m", plus: "Farther by 0.5 m", hint: `${fmt(LIM.setback[0])} to ${fmt(LIM.setback[1])}` }) +
    numField("cwidth", "Crossing width", c.width_mm, { minus: "Narrower by 0.5 m", plus: "Wider by 0.5 m", hint: `${fmt(LIM.crossing[0])} to ${fmt(LIM.crossing[1])}` }) +
    `<section class="insp-sec"><h3 class="note-h" id="i-h-refuge">Halfway island</h3><ul class="opts">${option("island", c.island, "", a.can_island ? "Refuge island in the middle" : "Refuge island (road too narrow)", `data-iisland="1" ${a.can_island ? "" : "disabled"}`, "checkbox")}</ul></section>` +
    `<section class="insp-sec"><h3 class="note-h" id="i-h-bulb">Shorten the crossing</h3><ul class="opts">${bulbs}</ul></section>`
  );
}

function renderInspector() {
  const active = document.activeElement;
  const focusId = active && el.inspector.contains(active) ? active.dataset.ifid : null;
  const v = view;
  const a = selArm();
  el.inspector.style.removeProperty("--kc");
  if (v.selected.kind === "bus" && v.bus) {
    const o = v.bus_options.find((p) => p.a === Math.min(v.bus.from, v.bus.to) && p.b === Math.max(v.bus.from, v.bus.to));
    el.inspector.innerHTML = `
      <div class="insp-head"><div><h2 class="insp-name">Bus lane</h2><p class="insp-sub">${esc(o?.label ?? "")}</p></div></div>
      <section class="insp-sec"><p class="insp-range">A ${fmt(v.bus.width_mm)} bus-only lane straight across the island. It crosses the ring where it enters and leaves.</p></section>
      ${controlSection()}
      <section class="insp-sec"><button type="button" class="btn danger" data-ibusoff="1" data-ifid="busoff">${BTN.remove}Remove the bus lane</button></section>`;
  } else if (v.selected.kind === "cycle" && v.ring?.cycle_mm) {
    el.inspector.innerHTML = `
      <div class="insp-head"><div><h2 class="insp-name">Cycle track</h2><p class="insp-sub">Round the outside of the roundabout</p></div></div>
      ${numField("cycle", "Track width", v.ring.cycle_mm, { minus: "Narrower by 0.5 m", plus: "Wider by 0.5 m", hint: `${fmt(LIM.cycle[0])} to ${fmt(LIM.cycle[1])}. Cyclists cross every street where it meets the ring.` })}
      ${controlSection()}
      <section class="insp-sec"><button type="button" class="btn danger" data-icycleoff="1" data-ifid="cycleoff">${BTN.remove}Remove the cycle track</button></section>`;
  } else if (!a) {
    renderJunctionPanel();
  } else if (v.selected.kind === "lane") {
    const i = v.selected.lane;
    const l = a.lanes[i];
    const opts = l.dests
      .map((d) => option(`ln-${d.uid}`, d.on, turnGlyph(d.class, 22), `${CLASS_NAME[d.class]} to ${d.label}${d.open ? "" : " (one way in)"}`, `data-lane="${i}" data-to="${d.uid}" ${d.open ? "" : "disabled"}`, "checkbox"))
      .join("");
    el.inspector.innerHTML = `
      <div class="insp-head"><div><h2 class="insp-name">Lane ${i + 1} of ${a.lanes.length}</h2><p class="insp-sub">${esc(a.label)}${laneNote(a, i) ? ` · ${laneNote(a, i)}` : ""}</p></div></div>
      <section class="insp-sec"><h3 class="note-h" id="i-h-serves">Where this lane goes</h3><ul class="opts">${opts}</ul>
      <p class="insp-range">${l.bad ? "Every street it goes to is banned. Add a street, or allow a turn." : `${fmt(l.width_mm)} wide. A lane has to go to at least one street.`}</p></section>
      <section class="insp-sec"><button type="button" class="btn" data-pickarm="1" data-ifid="pickarm">Select the whole street</button></section>
      ${controlSection()}`;
  } else if (v.selected.kind === "corner") {
    const c = corner(a.uid);
    const next = arm(c.next_uid);
    el.inspector.innerHTML = `
      <div class="insp-head"><div><h2 class="insp-name">Corner</h2><p class="insp-sub">${esc(compass(a.bearing))} to ${esc(compass(next.bearing))}</p></div></div>
      ${numField("corner", "Curb radius", c.radius_mm, { minus: "Tighter by 0.5 m", plus: "Wider by 0.5 m", hint: `${fmt(LIM.corner[0])} to ${fmt(LIM.corner[1])}. Cars turn here at about ${Math.round(c.speed_kmh)} km/h.` })}
      <p class="insp-empty">A tight corner slows turning cars and shortens the walk across. A wide one lets them swing through faster.</p>${controlSection()}`;
  } else {
    const crossingOnly = v.selected.kind === "crossing";
    const streets = CAT.streets.map((s, i) => (s.freeway ? "" : `<option value="${i}"${i === a.street_index ? " selected" : ""}>${esc(s.name)}</option>`)).join("");
    const head = `<div class="insp-head"><div><h2 class="insp-name">${esc(a.street)}</h2><p class="insp-sub">${esc(compass(a.bearing))}, ${a.bearing}° · ${fmt(a.road_mm)} road</p></div></div>`;
    if (crossingOnly) {
      el.inspector.innerHTML = head + crossingSection(a) + controlSection();
    } else {
      const dirSec = numField("bearing", "Direction", a.bearing, {
        minus: "Turn anticlockwise by 5°",
        plus: "Turn clockwise by 5°",
        value: a.bearing,
        step: LIM.bearing_step,
        min: 0,
        max: 355,
        tag: "°",
        hint: "Clockwise from north. At least 30° from its neighbours.",
      });
      const offSec = numField("offset", "Shift sideways", a.offset_mm, {
        minus: "Shift left by 0.1 m",
        plus: "Shift right by 0.1 m",
        value: num(a.offset_mm).toFixed(2),
        step: units === "m" ? "0.1" : "0.5",
        hint: `Up to ${fmt(a.max_offset_mm)} either way. Looking out from the junction.`,
      });
      el.inspector.innerHTML =
        head +
        `<section class="insp-sec"><h3 class="note-h" id="i-h-lanes">Lanes coming in</h3>${a.enters ? `<ul class="lanes">${laneRows(a)}</ul>` : `<p class="insp-range">One way out. No lanes come in.</p>`}</section>` +
        crossingSection(a) +
        transitSection(a) +
        dirSec +
        offSec +
        (v.linked
          ? `<section class="insp-sec"><h3 class="note-h" id="i-h-street">Street</h3><a class="btn" href="index.html?street=${a.edge}" data-ifid="street-link">Open the cross-section</a><p class="insp-range">This street belongs to the city. Its layout is edited in the street editor, and changes there show here.</p></section>`
          : `<section class="insp-sec"><h3 class="note-h" id="i-h-street">Street</h3><select data-istreet data-ifid="street" aria-labelledby="i-h-street">${streets}</select><p class="insp-range">The street's own layout is edited in the street editor.</p></section>`) +
        controlSection() +
        (v.linked
          ? ""
          : `<section class="insp-sec"><button type="button" class="btn danger" data-iremove="1" data-ifid="remove" ${v.can_remove ? "" : "disabled"}>${BTN.remove}Remove this street</button>${v.can_remove ? "" : `<p class="insp-range">A junction needs at least ${LIM.min_arms} streets.</p>`}</section>`);
    }
  }
  if (focusId) {
    const t = el.inspector.querySelector(`[data-ifid="${focusId}"]`);
    if (t && !t.disabled) t.focus({ preventScroll: true });
  }
}

// ---- rendering ------------------------------------------------------------

function render() {
  renderHead();
  renderKey();
  renderPlan();
  renderInspector();
  renderNotes();
  for (const b of el.samples.querySelectorAll("[data-sample]")) b.setAttribute("aria-pressed", String(Number(b.dataset.sample) === view.sample));
}

function refresh() {
  view = JSON.parse(plan.view());
  keepSoon();
  render();
}

function refreshDrawing() {
  view = JSON.parse(plan.view());
  keepSoon();
  renderHead();
  renderPlan();
}

// The notes are drawn, and the turn table edited, by the page's Rust components;
// what they change is drawn and announced here as any other edit is.
plan.on_edit((ok) => {
  refresh();
  if (ok) announceEdit();
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
  renderPlan();
  renderInspector();
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

// ---- inspector events -------------------------------------------------------

el.inspector.addEventListener("click", (e) => {
  const b = e.target.closest("button");
  if (!b || b.disabled) return;
  const d = b.dataset;
  const s = view.selected;
  const uid = s.uid;
  if (d.istep) return void step(d.istep, Number(d.dir));
  if (d.ibuslane !== undefined) return void act(() => plan.set_bus_lane(uid, b.getAttribute("aria-checked") !== "true"));
  if (d.ifilter !== undefined) return void act(() => plan.set_filter(uid, b.getAttribute("aria-checked") !== "true"));
  if (d.icycle !== undefined) return void act(() => plan.set_cycle_track(!view.ring.cycle_mm));
  if (d.icycleoff !== undefined) return void act(() => plan.set_cycle(0));
  if (d.ibusoff !== undefined) return void act(() => plan.set_bus(0, 0));
  if (d.pickarm !== undefined) return void select("arm", uid);
  if (d.pick !== undefined) return void select("lane", uid, Number(d.pick));
  if (d.lane !== undefined) return void act(() => plan.set_lane_dest(uid, Number(d.lane), Number(d.to), (b.getAttribute("aria-pressed") ?? b.getAttribute("aria-checked")) !== "true"));
  if (d.icross !== undefined) return void act(() => plan.set_crossing(uid, d.icross === "1"));
  if (d.iisland !== undefined) return void act(() => plan.set_island(uid, b.getAttribute("aria-checked") !== "true"));
  if (d.ibulb !== undefined) return void act(() => plan.set_bulb(uid, Number(d.ibulb), b.getAttribute("aria-checked") !== "true"));
  if (d.iremove) return void removeSelected();
});

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

el.inspector.addEventListener("change", (e) => {
  const t = e.target;
  const s = view.selected;
  if (t.dataset.iapproach !== undefined) return void act(() => plan.set_approach(s.uid, Number(t.value)));
  if (t.dataset.istop !== undefined) return void act(() => plan.set_stop(s.uid, Number(t.value)));
  if (t.dataset.irule !== undefined) return void act(() => plan.set_rule(s.uid, Number(t.value)));
  if (t.dataset.ibus !== undefined) {
    const [a, b] = t.value ? t.value.split("-").map(Number) : [0, 0];
    return void act(() => plan.set_bus(a, b));
  }
  if (t.dataset.icontrol !== undefined) return void act(() => plan.set_control(Number(t.value)));
  if (t.dataset.istreet !== undefined) return void act(() => plan.set_street(s.uid, Number(t.value)));
  const key = t.dataset.iset;
  if (!key) return;
  const v = parseFloat(t.value);
  if (!Number.isFinite(v)) return renderInspector();
  const set = {
    bearing: () => plan.set_bearing(s.uid, Math.round(v)),
    offset: () => plan.set_offset(s.uid, fromInput(v)),
    corner: () => plan.set_corner(s.uid, fromInput(v)),
    setback: () => plan.set_setback(s.uid, fromInput(v)),
    cwidth: () => plan.set_crossing_width(s.uid, fromInput(v)),
    ring: () => plan.set_ring_radius(fromInput(v / 2)),
    cycle: () => plan.set_cycle(fromInput(v)),
    alen: () => plan.set_approach_len(s.uid, fromInput(v)),
  }[key];
  act(set);
});

// Radio groups: arrows move the choice, as native radios do.
el.inspector.addEventListener("keydown", (e) => {
  const cur = e.target.closest('[role="radio"]');
  if (!cur || e.metaKey || e.ctrlKey || e.altKey) return;
  const stepBy = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
  if (!stepBy) return;
  e.preventDefault();
  const group = [...cur.closest('[role="radiogroup"]').querySelectorAll('[role="radio"]')];
  const next = group[(group.indexOf(cur) + stepBy + group.length) % group.length];
  next.focus();
  next.click();
});

function removeSelected() {
  const s = view.selected;
  if (s.kind === "bus") return void act(() => plan.set_bus(0, 0));
  if (s.kind === "cycle") return void act(() => plan.set_cycle(0));
  if (s.kind === "crossing") return void act(() => plan.set_crossing(s.uid, false));
  if (s.kind === "arm") act(() => plan.remove_arm(s.uid));
}

// ---- pointer ----------------------------------------------------------------

let drag = null;

el.svg.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  const t = e.target.closest("[data-role]");
  const role = t?.dataset.role;
  const uid = Number(t?.dataset.uid);
  if (role === "grip-arm" || role === "grip-corner" || role === "grip-crossing") {
    if (role === "grip-arm") plan.select("arm", uid, 0);
    drag = { type: role, uid, moved: false };
    plan.begin_gesture();
    el.svg.setPointerCapture(e.pointerId);
    e.preventDefault();
    return;
  }
  if (role === "cycle") select("cycle", 0);
  else if (role === "bus") select("bus", 0);
  else if (role === "lane") select("lane", uid, Number(t.dataset.lane));
  else if (role === "arm") select("arm", uid);
  else if (role === "crossing") select("crossing", uid);
  else if (role === "corner") select("corner", uid);
  else if (sel().kind) select(null, 0);
  el.wrap.focus({ preventScroll: true });
});

el.svg.addEventListener("pointermove", (e) => {
  if (!drag) return;
  const [x, y] = mmOf(e);
  const ok = { "grip-arm": plan.drag_arm_to, "grip-corner": plan.drag_corner_to, "grip-crossing": plan.drag_crossing_to }[drag.type].call(plan, drag.uid, x, y);
  if (ok) drag.moved = true;
  refreshDrawing();
});

function finishPointer(commit) {
  if (!drag) return;
  const d = drag;
  drag = null;
  if (commit) {
    const changed = plan.end_gesture();
    refresh();
    if (changed) announceEdit();
    else announceSelection();
  } else {
    plan.cancel_gesture();
    refresh();
  }
  void d;
}
el.svg.addEventListener("pointerup", () => finishPointer(true));
el.svg.addEventListener("pointercancel", () => finishPointer(false));

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
  const r = el.svg.getBoundingClientRect();
  const over = e && e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
  if (commit && over) {
    act(() => plan.add_arm_toward(c.street, ...mmOf(e)) !== 0);
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
    renderPlan();
    renderInspector();
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

new ResizeObserver(() => renderPlan()).observe(el.scroll);
window.addEventListener("resize", () => renderPlan());

renderPalette();
render();

initAccountMenu();
initPanels({ say, detailsWord: "details" });

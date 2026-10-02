// Draws the view the Rust core returns and relays pointer and keyboard input.
// No editing rules live here: widths, snapping, limits, history and checks all
// come from the WebAssembly model.

import init, { Sheet, atlas, catalogue, materials, samples } from "./pkg/cityloom_editor.js";
import { CURB_HATCH, HATCH, MATERIAL_HATCH, symbol } from "./symbols.js";
import { NOT_KEPT, keeper, openCity, placeParam, regionIndex, writeCity } from "./city.js";
import { engineering, initAccountMenu, initDrawingStyle, initPanels, initRegion, initTheme, initUnits, remember, typing } from "./shell.js";

await init();

const KINDS = JSON.parse(catalogue());
const MATERIALS = JSON.parse(materials());
const SAMPLES = JSON.parse(samples());
const ATLAS = JSON.parse(atlas());
// Opened from the map (`?street=7`) the page edits that street of the city and
// writes each change back; otherwise it is a sandbox on the sample streets.
const placeId = placeParam("street");
const city = placeId ? openCity() : null;
const held = city?.street(placeId, regionIndex(MATERIALS.regions));
if (placeId && !held) {
  location.replace("map.html");
  await new Promise(() => {});
}
const sheet = held ?? new Sheet(0);
let view = JSON.parse(sheet.view());
let units = "m";

const $ = (id) => document.getElementById(id);
const el = {
  svg: $("drawing"),
  wrap: $("wrap"),
  scroll: $("scroll"),
  palette: $("palette"),
  space: $("space"),
  cap: $("cap"),
  checks: $("checks"),
  revs: $("revs"),
  live: $("live"),
  undo: $("undo"),
  redo: $("redo"),
  reset: $("reset"),
  inspector: $("inspector"),
};

// ---- formatting -----------------------------------------------------------

const MM_PER_FT = 304.8;
const num = (mm) => (units === "m" ? mm / 1000 : mm / MM_PER_FT);
const fmtN = (mm) => {
  const v = num(mm);
  return units === "m" ? v.toFixed(mm % 100 === 0 ? 1 : 2) : v.toFixed(1);
};
const fmt = (mm) => `${fmtN(mm)} ${units}`;
const signed = (mm) => {
  if (mm === 0) return "0";
  const s = fmtN(Math.abs(mm));
  return (mm > 0 ? "+" : "−") + s;
};
const fromInput = (v) => Math.round(units === "m" ? v * 1000 : v * MM_PER_FT);
const unitWord = () => (units === "m" ? "metres" : "feet");

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

// ---- static pieces --------------------------------------------------------

$("defs").innerHTML = `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

const swatch = (kindId) =>
  `<svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect class="k-${kindId}" width="44" height="22" stroke="none"/><rect width="44" height="22" fill="url(#h-${kindId})" stroke="none"/></svg>`;

const ICON = {
  left: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M12 7H2M6 3 2 7l4 4"/></svg>`,
  right: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10M8 3l4 4-4 4"/></svg>`,
  remove: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 3l8 8M11 3l-8 8"/></svg>`,
  minus: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8"/></svg>`,
  plus: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8M7 3v8"/></svg>`,
  tick: `<svg class="tick" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg>`,
  ok: `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg>`,
  bad: `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 3l10 10M13 3 3 13"/></svg>`,
};

// ---- state helpers --------------------------------------------------------

const seg = (uid) => view.segments.find((s) => s.uid === uid);
const idxOf = (uid) => view.segments.findIndex((s) => s.uid === uid);
const selIndex = () => (view.selected ? idxOf(view.selected) : -1);
const kindOf = (s) => KINDS[s.kind];

function fitText() {
  const d = view.delta_mm;
  if (d === 0) return "Every metre of the street is used";
  if (d < 0) return `${fmt(-d)} left to use`;
  return `${fmt(d)} too wide. Make a piece narrower or remove one`;
}

let liveTimer = 0;
function say(text) {
  clearTimeout(liveTimer);
  el.live.textContent = "";
  liveTimer = setTimeout(() => (el.live.textContent = text), 40);
}

function announceEdit() {
  const last = view.revisions.at(-1);
  if (last) say(`${last.label}. ${fitText()}.`);
}

function announceSelection() {
  const i = selIndex();
  if (i < 0) return;
  const s = view.segments[i];
  say(`${kindOf(s).name}, ${fmt(s.width_mm)}, ${i + 1} of ${view.segments.length}`);
}

function refresh() {
  view = JSON.parse(sheet.view());
  render();
}

// ---- drawing --------------------------------------------------------------

const Y0 = {
  exLabel: 16,
  ex: 26,
  exH: 36,
  propLabel: 92,
  dim: 156,
  top: 172,
  ground: 312,
  slab: 34,
  mark: 368,
  total: 408,
  total2: 434,
  scale: 466,
  height: 484,
};
const Y = { ...Y0 };
let F = 1;
function setGeometry(f) {
  F = f;
  for (const k in Y0) Y[k] = Math.round(Y0[k] * f);
}
const TB_H = 46; // title block height in the engineering view
const L = { padL: 30, padR: 40, scale: 0.05, width: 760 };
let drag = null;
let wasOver = false;

const X = (mm) => L.padL + mm * L.scale;
const mmAt = (clientX) => (clientX - el.svg.getBoundingClientRect().left - L.padL) / L.scale;

function dimLine(x0, x1, y, cls = "") {
  return `<path class="dim ${cls}" d="M${x0} ${y}H${x1}M${x0 - 4} ${y + 4}L${x0 + 4} ${y - 4}M${x1 - 4} ${y + 4}L${x1 + 4} ${y - 4}"/>`;
}

// A revision cloud: a rectangle outlined with outward arcs.
function cloud(x, y, w, h, r = 7, fresh = false) {
  const edge = (len) => Math.max(1, Math.round(len / (r * 2.2)));
  const nx = edge(w);
  const ny = edge(h);
  const sx = w / nx;
  const sy = h / ny;
  const arc = (dx, dy) => `a${r} ${r} 0 0 1 ${dx} ${dy}`;
  let d = `M${x} ${y}`;
  for (let i = 0; i < nx; i++) d += arc(sx, 0);
  for (let i = 0; i < ny; i++) d += arc(0, sy);
  for (let i = 0; i < nx; i++) d += arc(-sx, 0);
  for (let i = 0; i < ny; i++) d += arc(0, -sy);
  return `<path class="cloud${fresh ? " fresh" : ""}" pathLength="1" d="${d}Z"/>`;
}

// Standard view: the hatch and tint name the kind of piece, so the surface
// finish is drawn as a paving course along the top of the slab.
const surfaceCourse = (x, y, w, material) =>
  `<rect class="surface" x="${x}" y="${y}" width="${w}" height="10"/>` +
  `<rect class="hatch" x="${x}" y="${y}" width="${w}" height="10" fill="url(#m-${material})"/>`;

function renderDrawing() {
  const v = view;
  const W = Math.max(el.wrap.clientWidth, 680);
  L.width = W;
  setGeometry(Math.min(Math.max(W / 1050, 0.9), 1.1));
  // Room right of the right-of-way line for the redline when a street runs over.
  L.padR = Math.max(60, Math.round(W * 0.17));
  L.scale = (W - L.padL - L.padR) / v.row_mm;
  const pxPerM = L.scale * 1000;
  const G = Y.ground;
  const bodyBottom = G + Y.slab;
  const over = v.delta_mm > 0;
  const fresh = over && !wasOver;
  wasOver = over;

  const parts = [];

  // existing strip, same scale and origin as the proposal
  parts.push(`<text class="t-label t-soft" x="${L.padL}" y="${Y.exLabel}">Today</text>`);
  for (const s of v.existing) {
    const k = KINDS[s.kind];
    const x = X(s.x_mm);
    const w = s.width_mm * L.scale;
    parts.push(
      `<rect class="obj k-${k.id}" x="${x}" y="${Y.ex}" width="${w}" height="${Y.exH}"/>` +
        `<rect class="hatch" x="${x}" y="${Y.ex}" width="${w}" height="${Y.exH}" fill="url(#${engineering() ? `m-${s.material}` : `h-${k.id}`})"/>`,
    );
    const label = fmtN(s.width_mm);
    if (w > label.length * 8.5 + 10) {
      parts.push(`<text class="t-dim t-halo" x="${x + w / 2}" y="${Y.ex + Y.exH / 2 + 4.5}" text-anchor="middle">${label}</text>`);
    }
  }

  // proposal heading
  const rev = v.revisions.at(-1);
  parts.push(
    `<text class="t-label" x="${L.padL + 30}" y="${Y.propLabel}">Your design</text>` +
      (rev ? `<text class="t-label t-blue" x="${L.padL + 130}" y="${Y.propLabel}">Change ${rev.step}</text>` : ""),
  );

  // right-of-way lines
  const xL = X(0);
  const xR = X(v.row_mm);
  for (const x of [xL, xR]) {
    parts.push(`<line class="rw" x1="${x}" x2="${x}" y1="${Y.dim - 34}" y2="${Y.total2 + 6}"/>`);
  }
  parts.push(
    `<text class="t-label t-faint" x="${xL}" y="${Y.dim - 42}" text-anchor="start">Street edge</text>` +
      `<text class="t-label t-faint" x="${xR}" y="${Y.dim - 42}" text-anchor="end">Street edge</text>`,
  );

  // sky behind the section, within the right-of-way
  parts.push(`<rect class="sky" x="${xL}" y="${Y.top}" width="${xR - xL}" height="${G - Y.top}"/>`);

  // ground
  parts.push(`<line class="ground" x1="${xL - 14}" x2="${Math.min(Math.max(X(v.total_mm), xR) + 14, W - 4)}" y1="${G}" y2="${G}"/>`);

  // unassigned space
  if (v.delta_mm < 0) {
    const x = X(v.total_mm);
    const w = xR - x;
    parts.push(`<rect class="free" x="${x}" y="${Y.top}" width="${w}" height="${bodyBottom - Y.top}"/>`);
    const t = `Unused ${fmt(-v.delta_mm)}`;
    if (w >= 88) {
      parts.push(
        `<text class="t-label t-faint" x="${x + w / 2}" y="${G - 8}" text-anchor="middle">Unused</text>` +
          `<text class="t-dim t-soft" x="${x + w / 2}" y="${G + 10}" text-anchor="middle">${fmt(-v.delta_mm)}</text>`,
      );
    } else if (w >= 22) {
      parts.push(`<text class="t-label t-faint" transform="translate(${x + w / 2 + 4} ${G - 8}) rotate(-90)">${t}</text>`);
    }
  }

  // segments, dimension strings, marks
  for (const s of v.segments) {
    const k = KINDS[s.kind];
    const x = X(s.x_mm);
    const w = s.width_mm * L.scale;
    const cx = x + w / 2;
    const sel = s.uid === v.selected;
    const lifted = drag && drag.type === "move" && drag.active && drag.uid === s.uid;
    parts.push(
      `<g class="seg${lifted ? " lifted" : ""}" data-role="seg" data-uid="${s.uid}">` +
        `<rect class="hit" x="${x}" y="${Y.top}" width="${w}" height="${bodyBottom - Y.top}"/>` +
        symbol(k.id, cx, G, pxPerM, w, s.width_mm / 1000, F, { shelter: s.shelter, material: s.material, tram: s.tram }) +
        `<rect class="obj k-${k.id}" x="${x}" y="${G}" width="${w}" height="${Y.slab}"/>` +
        `<rect class="hatch" x="${x}" y="${G}" width="${w}" height="${Y.slab}" fill="url(#${engineering() ? `m-${s.material}` : `h-${k.id}`})"/>` +
        (engineering() ? "" : surfaceCourse(x, G, w, s.material)) +
        (s.direction && w >= 26 ? dirGlyph(s.direction, cx, G + Y.slab / 2) : "") +
        (s.variants.length && w >= 48 ? clockBadge(x + 24, G + Y.slab - 10) : "") +
        `<text class="t-mark t-halo${sel ? " t-blue" : ""}" x="${cx}" y="${Y.mark}" text-anchor="middle">${w > k.name.length * 8.6 + 10 ? esc(k.name) : w > 30 ? k.mark : ""}</text>` +
        `</g>`,
    );
    parts.push(dimLine(x, x + w, Y.dim));
    const label = fmtN(s.width_mm);
    if (w > label.length * 9 + 8) {
      parts.push(`<text class="t-dim t-halo${sel ? " t-blue" : ""}" x="${cx}" y="${Y.dim - 7}" text-anchor="middle">${label}</text>`);
    }
    if (sel) {
      parts.push(`<rect class="sel-box" x="${x}" y="${Y.dim - 28}" width="${w}" height="${Y.mark + 10 - (Y.dim - 28)}"/>`);
    }
  }

  // boundaries and their drag handles
  const n = v.segments.length;
  for (let i = 0; i <= n; i++) {
    const mm = i === 0 ? 0 : v.segments[i - 1].x_mm + v.segments[i - 1].width_mm;
    const x = X(mm);
    const interior = i > 0 && i < n;
    const edgeOfLast = i === n && n > 0;
    const line = `<line class="bound bound-line" x1="${x}" x2="${x}" y1="${Y.dim - 8}" y2="${bodyBottom}"/>`;
    if (!interior && !edgeOfLast) {
      parts.push(line);
      continue;
    }
    const attrs = interior ? `data-role="handle" data-i="${i - 1}"` : `data-role="edge" data-uid="${v.segments[n - 1].uid}"`;
    const active = drag && ((drag.type === "resize" && drag.i === i - 1) || (drag.type === "edge" && edgeOfLast)) ? " active" : "";
    const gy = G + Y.slab / 2;
    // A finger needs more to land on than a pointer does (WCAG 2.5.8: 24px).
    const hitHalf = matchMedia("(pointer: coarse)").matches ? 15 : 9;
    parts.push(
      `<g class="handle${active}" ${attrs}>${line}` +
        `<rect class="hit" x="${x - hitHalf}" y="${Y.dim - 8}" width="${hitHalf * 2}" height="${bodyBottom - (Y.dim - 8)}"/>` +
        `<rect class="grip" x="${x - 9}" y="${gy - 8}" width="18" height="16"/>` +
        `<path class="grip-arrow" d="M${x - 5} ${gy}H${x + 5}M${x - 5} ${gy}l3 -3M${x - 5} ${gy}l3 3M${x + 5} ${gy}l-3 -3M${x + 5} ${gy}l-3 3"/>` +
        `</g>`,
    );
  }

  // overflow: redline wash and revision cloud
  if (over) {
    const x = xR;
    const clipped = X(v.total_mm) > W - 10;
    const w = Math.min(X(v.total_mm), W - 10) - xR;
    const top = Y.dim - 38;
    const h = Y.mark + 18 - top;
    const inset = Math.min(14, w / 4);
    parts.push(`<rect class="over-wash" x="${x}" y="${top}" width="${w}" height="${h}"/>`);
    parts.push(cloud(x + inset, top, Math.max(w - inset * 2, 14), h, 6, fresh));
    parts.push(
      `<text class="t-over" x="${xR + 10}" y="${Y.dim - 62}">${fmt(v.delta_mm)} too wide${clipped ? " (more off-screen)" : ""}</text>`,
    );
  }

  // curbs: a block on each side of a curbed piece that faces a road piece
  {
    const road = new Set(["travel", "bus", "parking", "loading"]);
    v.segments.forEach((s, i) => {
      if (!s.curb) return;
      const x = X(s.x_mm);
      const w = s.width_mm * L.scale;
      // a planted curb is a strip of planting, not a kerb stone
      const planted = s.curb === "planted";
      // a Kassel kerb is wider and has a sloped road face, so a wheel can touch it safely
      const kassel = s.curb === "kassel";
      // a bus boarding island is a wide raised platform between the lane and the traffic
      const island = s.curb === "island";
      // a bike-friendly curb is a low ramp a wheel can ride up
      const ramp = s.curb === "bikefriendly";
      const cw = Math.min(planted ? Math.max(14, 600 * L.scale) : island ? Math.max(16, 900 * L.scale) : kassel ? Math.max(9, 250 * L.scale) : ramp ? Math.max(9, 300 * L.scale) : Math.max(5, 150 * L.scale), w / 2);
      const ch = island ? 16 : 12;
      const fill = planted ? "m-planted" : `c-${s.curb}`;
      const sides = [
        [v.segments[i - 1], x, false],
        [v.segments[i + 1], x + w - cw, true],
      ];
      // an island stands on one side only, the one facing the buses
      if (island) {
        const rank = (nb) => ["bus", "travel"].indexOf(KINDS[nb.kind].id);
        const facing = sides.filter(([nb]) => nb && road.has(KINDS[nb.kind].id)).sort((a, b) => rank(b[0]) - rank(a[0]));
        sides.splice(0, sides.length, ...facing.slice(0, 1));
      }
      for (const [nb, bx, roadRight] of sides) {
        if (!nb || !road.has(KINDS[nb.kind].id)) continue;
        if (kassel || ramp) {
          const [a, b] = roadRight ? [bx + cw, bx] : [bx, bx + cw]; // a: road-side foot, b: back
          const top = ramp ? b : roadRight ? bx + cw * 0.35 : bx + cw * 0.65;
          const h = ramp ? 7 : 12;
          const pts = `${a},${G} ${b},${G} ${b},${G - h} ${top},${G - h}`;
          parts.push(`<polygon class="curb" points="${pts}"/><polygon class="hatch" points="${pts}" fill="url(#${fill})"/>`);
          continue;
        }
        parts.push(
          `<rect class="curb" x="${bx}" y="${G - ch}" width="${cw}" height="${ch}"/>` +
            `<rect class="hatch" x="${bx}" y="${G - ch}" width="${cw}" height="${ch}" fill="url(#${fill})"/>`,
        );
      }
    });
  }

  // engineering view: lane markings as they cut through the section, a filled
  // block for a solid line and an outlined one for a broken line
  if (engineering()) {
    const road = new Set(["travel", "bus", "bike", "parking", "loading"]);
    for (let i = 1; i < n; i++) {
      const a = KINDS[v.segments[i - 1].kind].id;
      const b = KINDS[v.segments[i].kind].id;
      if (!road.has(a) || !road.has(b)) continue;
      // a bike-lane curb takes the place of the line on its side
      if ((a === "bike" && v.segments[i - 1].curb) || (b === "bike" && v.segments[i].curb)) continue;
      const broken = a === "travel" && b === "travel";
      const x = X(v.segments[i].x_mm);
      parts.push(`<rect class="lane-mark${broken ? " broken" : ""}" x="${x - 3.5}" y="${G - 9}" width="7" height="9"/>`);
    }
    const kx = xR - 236;
    if (kx > L.padL + 60 + 5 * (units === "m" ? 1000 : MM_PER_FT * 5) * L.scale + 30) {
      const ky = Y.scale;
      parts.push(
        `<text class="t-label t-soft" x="${kx}" y="${ky - 6}">Lane markings</text>` +
          `<rect class="lane-mark" x="${kx + 136}" y="${ky - 16}" width="7" height="10"/>` +
          `<text class="t-dim t-soft" x="${kx + 150}" y="${ky - 6}">Solid</text>` +
          `<rect class="lane-mark broken" x="${kx + 190}" y="${ky - 16}" width="7" height="10"/>` +
          `<text class="t-dim t-soft" x="${kx + 204}" y="${ky - 6}">Broken</text>`,
      );
    }
  }

  // engineering view: extension lines carry each boundary down to the overall strings
  if (engineering()) {
    const bounds = new Set([0, v.row_mm]);
    for (const s of v.segments) bounds.add(s.x_mm + s.width_mm);
    for (const mm of bounds) {
      if (mm > v.row_mm) continue;
      parts.push(`<line class="ext" x1="${X(mm)}" x2="${X(mm)}" y1="${bodyBottom + 4}" y2="${Y.total + 6}"/>`);
    }
  }

  // overall dimension strings
  parts.push(dimLine(xL, xR, Y.total));
  parts.push(`<text class="t-dim t-halo" x="${(xL + xR) / 2}" y="${Y.total - 7}" text-anchor="middle">Street width ${fmt(v.row_mm)}</text>`);
  if (v.delta_mm !== 0) {
    const cls = over ? "dim-red" : "";
    parts.push(dimLine(xL, X(v.total_mm), Y.total2, cls));
    parts.push(
      `<text class="t-dim t-halo ${over ? "t-red" : "t-soft"}" x="${(xL + X(v.total_mm)) / 2}" y="${Y.total2 - 7}" text-anchor="middle">Your design ${fmt(v.total_mm)}</text>`,
    );
  }

  // graphic scale bar: five equal parts, so the scale claim can be checked by eye
  {
    const unitMm = units === "m" ? 1000 : MM_PER_FT * 5;
    const parts5 = 5;
    const barW = unitMm * parts5 * L.scale;
    const y = Y.scale;
    parts.push(`<text class="t-label t-soft" x="${L.padL}" y="${y - 8}">Scale</text>`);
    for (let i = 0; i < parts5; i++) {
      parts.push(`<rect class="${i % 2 ? "bar-w" : "bar-b"}" x="${L.padL + 60 + i * unitMm * L.scale}" y="${y - 14}" width="${unitMm * L.scale}" height="8"/>`);
    }
    const step = units === "m" ? 1 : 5;
    for (let i = 0; i <= parts5; i += i === 0 ? 1 : 1) {
      if (i % (barW > 260 ? 1 : 5) !== 0 && i !== parts5) continue;
      parts.push(`<text class="t-dim t-soft" x="${L.padL + 60 + i * unitMm * L.scale}" y="${y + 12}" text-anchor="middle">${i * step}</text>`);
    }
    parts.push(`<text class="t-dim t-soft" x="${L.padL + 60 + barW + 8}" y="${y - 6}">${units}</text>`);
  }

  // engineering view: sheet border and title block
  const furniture = [];
  const H = engineering() ? Y.height + TB_H + 20 : Y.height;
  if (engineering()) {
    const top = H - TB_H - 4;
    const cells = [
      ["Street", v.name, 2.4],
      ["Width", fmt(v.row_mm), 1],
      ["Units", units === "m" ? "Metres" : "Feet", 1],
      ["Changes", String(v.revisions.length), 1],
      ["Sheet", "1 of 1", 1],
    ];
    const total = cells.reduce((a, c) => a + c[2], 0);
    const x0 = 4;
    const full = W - 8;
    furniture.push(`<rect class="sheet-frame" x="${x0}" y="4" width="${full}" height="${H - 8}"/>`);
    furniture.push(`<line class="tb-line" x1="${x0}" x2="${x0 + full}" y1="${top}" y2="${top}"/>`);
    let cx = x0;
    for (const [label, value, fr] of cells) {
      const w = (full * fr) / total;
      if (cx > x0) furniture.push(`<line class="tb-line" x1="${cx}" x2="${cx}" y1="${top}" y2="${H - 4}"/>`);
      furniture.push(
        `<svg x="${cx}" y="${top}" width="${w}" height="${TB_H}">` +
          `<text class="tb-l" x="10" y="16">${label}</text>` +
          `<text class="tb-v" x="10" y="${TB_H - 10}">${esc(value)}</text>` +
          `</svg>`,
      );
      cx += w;
    }
  }

  // drag feedback
  if (drag && drag.type === "move" && drag.active) {
    const s = seg(drag.uid);
    const w = s.width_mm * L.scale;
    parts.push(`<rect class="ghost" x="${drag.px - w / 2}" y="${G - 4}" width="${w}" height="${Y.slab + 8}"/>`);
    parts.push(`<text class="t-mark t-blue" x="${drag.px}" y="${G + Y.slab / 2 + 5}" text-anchor="middle">${kindOf(s).mark}</text>`);
  }
  if (drag && drag.idx != null && drag.type === "move" && drag.active) {
    const others = v.segments.filter((s) => s.uid !== (drag.uid || 0));
    const mm = others.slice(0, drag.idx).reduce((a, s) => a + s.width_mm, 0);
    const x = X(mm);
    parts.push(
      `<line class="caret" x1="${x}" x2="${x}" y1="${Y.dim - 14}" y2="${bodyBottom + 6}"/>` +
        `<path class="caret-head" d="M${x - 6} ${Y.dim - 24}L${x + 6} ${Y.dim - 24}L${x} ${Y.dim - 13}Z"/>`,
    );
  }

  el.svg.setAttribute("width", W);
  el.svg.setAttribute("height", H);
  el.svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  el.svg.setAttribute(
    "aria-label",
    `Cross-section of ${v.name}. ${n} segments, ${fmt(v.total_mm)} of ${fmt(v.row_mm)}. ${fitText()}.`,
  );
  // the drawing sits a little lower inside the border in the engineering view
  el.svg.innerHTML = engineering() ? `<g transform="translate(0 10)">${parts.join("")}</g>${furniture.join("")}` : parts.join("");
}

// ---- the Add menu ----------------------------------------------------------

// The pieces by what they are for, so twelve rows read as four lists.
const ADD_GROUPS = [
  ["Walk and plant", ["sidewalk", "planting", "median"]],
  ["Cycling", ["bike", "bikerack", "bikeshare"]],
  ["Roadway", ["travel", "bus", "parking", "loading", "shoulder"]],
  ["Furniture", ["pole"]],
];

function renderPalette() {
  const row = (k, i) =>
    `<li><button type="button" class="add-item" data-kind="${i}">${swatch(k.id)}<b>${esc(k.name)}</b><span class="dw">${fmt(k.default_mm)}</span></button></li>`;
  el.palette.innerHTML = ADD_GROUPS.map(([name, ids], g) => {
    const rows = ids.map((id) => KINDS.findIndex((k) => k.id === id)).filter((i) => i >= 0).map((i) => row(KINDS[i], i));
    return `<li role="presentation" class="add-group"><p class="add-group-h" id="add-g-${g}">${name}</p><ul class="add-sub" role="group" aria-labelledby="add-g-${g}">${rows.join("")}</ul></li>`;
  }).join("");
}

// ---- Atlas measures ----------------------------------------------------------

// Every measure of the Transit Priority Atlas toolbox: the lane arrangements can
// be recognised in this street and laid out from its width; the rest say where
// they are set, or why they are not modelled.
// What each group of the Atlas toolbox means here, in one line.
const MEASURE_GROUP_NOTE = {
  "Linear continuous measures": "Lane arrangements along the street. Arrange lays the roadway out as that measure.",
  "Localized measures": "Features at one junction. Set them on the intersection page.",
  "Area-wide measures": "They cover many streets, so they are not modelled here.",
};

function renderMeasures() {
  const found = new Map(view.measures.map((m) => [m.code, m]));
  const groups = [...new Set(ATLAS.map((m) => m.group))];
  $("measures").innerHTML = groups
    .map((g) => {
      const rows = ATLAS.filter((m) => m.group === g)
        .map((m) => {
          let state;
          let extra = "";
          if (m.place === "street") {
            const f = found.get(m.code);
            if (f.present) {
              state = `<b class="m-on">This street</b>`;
              extra = f.problems.map((p) => `<small class="m-problem">${esc(p)}</small>`).join("");
            } else if (f.can_apply) {
              state = `<button type="button" class="btn m-apply" data-measure="${m.code}" aria-label="Arrange the street as ${esc(m.code)} ${esc(m.name)}">Arrange</button>`;
            } else {
              state = `<span class="m-not">${m.code === "B2" || m.code === "E3" ? "Freeways only" : view.measures && SAMPLES[view.sample].freeway ? "Not for a freeway" : "Will not fit"}</span>`;
            }
          } else if (m.place === "junction") {
            state = "Set at a junction";
          } else {
            state = `<span class="m-not">Not modelled</span>`;
          }
          const note = m.place === "street" ? "" : m.note ? `<small>${esc(m.note)}</small>` : "";
          return `<tr><th scope="row"><span class="dirtag">${esc(m.code)}</span>${esc(m.name)}${note}${extra}</th><td>${state}</td></tr>`;
        })
        .join("");
      return `<tbody><tr class="m-group"><th colspan="2" scope="colgroup">${esc(g)}${MEASURE_GROUP_NOTE[g] ? `<small class="m-group-note">${MEASURE_GROUP_NOTE[g]}</small>` : ""}</th></tr>${rows}</tbody>`;
    })
    .join("");
}

$("measures").addEventListener("click", (e) => {
  const b = e.target.closest("[data-measure]");
  if (!b) return;
  if (sheet.apply_measure(b.dataset.measure)) {
    refresh();
    announceEdit();
  } else say("That measure does not suit this street.");
});

// ---- notes ----------------------------------------------------------------

const head = (cols) => `<thead><tr>${cols.map((c) => `<th scope="col">${c}</th>`).join("")}</tr></thead>`;
const changeCell = (d, text) => `<td class="${d > 0 ? "up" : d < 0 ? "down" : "zero"}">${text}</td>`;

function checkDetail(c) {
  if (c.id === "fits") {
    if (c.ok) return view.delta_mm === 0 ? "Every metre is used" : `${fmt(c.amount_mm)} left to use`;
    return `${fmt(c.amount_mm)} too wide. Narrow or remove a piece.`;
  }
  if (c.id === "access") return c.ok ? `A lane of ${fmt(c.amount_mm)} or more` : `No lane of ${fmt(c.amount_mm)} or more`;
  return c.detail;
}

function renderNotes() {
  const o = view.outcomes;
  el.space.innerHTML =
    `<caption class="sr-only">Width by use, in ${unitWord()}</caption>` +
    head(["Use", "Today", "Your design", "Change"]) +
    `<tbody>${o.share
      .map((s, i) => {
        const ex = o.existing_share[i].mm;
        return `<tr><td><span class="mc mc-${s.mode}" aria-hidden="true"></span>${s.label}</td><td>${fmtN(ex)}</td><td>${fmtN(s.mm)}</td>${changeCell(s.mm - ex, signed(s.mm - ex))}</tr>`;
      })
      .join("")}</tbody>`;

  const d = o.capacity_pph - o.existing_capacity_pph;
  const pct = o.existing_capacity_pph ? Math.round((d * 100) / o.existing_capacity_pph) : 0;
  const fmtInt = (n) => n.toLocaleString("en-US");
  el.cap.innerHTML =
    `<caption class="sr-only">People per hour, placeholder rates</caption>` +
    head(["", "Today", "Your design", "Change"]) +
    `<tbody><tr><td>People per hour</td><td>${fmtInt(o.existing_capacity_pph)}</td><td>${fmtInt(o.capacity_pph)}</td>${changeCell(d, pct === 0 ? "0" : (pct > 0 ? "+" : "−") + Math.abs(pct) + "%")}</tr></tbody>`;

  el.checks.innerHTML = view.checks
    .map(
      (c) =>
        `<li class="${c.ok ? "ok" : "bad"}">${c.ok ? ICON.ok : ICON.bad}<div><b>${esc(c.label)}<span class="sr-only">: ${c.ok ? "passes" : "fails"}</span></b><span>${esc(checkDetail(c))}</span></div></li>`,
    )
    .join("");

  if (view.revisions.length && !welcomePinned) dismissWelcome();
  const failing = view.checks.filter((c) => !c.ok).length;
  const fc = $("fit-checks");
  fc.hidden = failing === 0;
  fc.textContent = failing === 1 ? "1 check fails" : `${failing} checks fail`;
  $("checks-n").hidden = failing === 0;
  $("checks-n").innerHTML = failing ? `${failing}<span class="sr-only"> fail</span>` : "";

  el.revs.innerHTML =
    head(["Step", "What changed"]) +
    `<tbody><tr class="base${view.revisions.length ? "" : " now"}"><td>—</td><td>Street today</td></tr>${view.revisions
      .map((r, i) => `<tr${i === view.revisions.length - 1 ? ' class="now"' : ""}><td>${r.step}</td><td>${esc(r.label)}</td></tr>`)
      .join("")}</tbody>`;
  el.revs.scrollTop = el.revs.scrollHeight;

  $("tb-changes").textContent = String(view.revisions.length);
}

function renderFit() {
  const box = $("fit");
  const d = view.delta_mm;
  box.className = "fit" + (d > 0 ? " bad" : "");
  box.textContent = d === 0 ? "Every metre of the street is used." : d < 0 ? `${fmt(-d)} of the street is still unused.` : `${fmt(d)} too wide. Make a piece narrower or remove one.`;
}

// A street of the city says which junctions it runs between, and links to them.
const ends = held ? JSON.parse(city.street_ends(placeId)) : [];
const endLink = (e) => (e.junction ? `<a href="intersection.html?junction=${e.uid}">${esc(e.name)}</a>` : esc(e.name));
const keepSoon = held
  ? keeper(
      () => writeCity((c) => c.keep_street(placeId, sheet)),
      () => say(NOT_KEPT),
    )
  : () => {};
if (held) {
  document.title = `${view.name} between ${ends.map((e) => e.name).join(" and ")} · CityLoom`;
  $("street-sub").innerHTML = `<a class="back" href="map.html"><svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>City map</a> <span aria-hidden="true">·</span> <span>Street cross-section</span> <span aria-hidden="true">·</span> <span><b id="row-dim" class="fig"></b> wide</span> <span aria-hidden="true">·</span> <span>between ${endLink(ends[0])} and ${endLink(ends[1])}</span>`;
  document.querySelector('.surface[href="intersection.html"]').hidden = true;
}

function renderHead() {
  $("street-name").textContent = view.name;
  $("row-dim").textContent = fmt(view.row_mm);
  $("tb-street").textContent = view.name;
  $("tb-row").textContent = fmt(view.row_mm);
  el.undo.disabled = !view.can_undo;
  el.redo.disabled = !view.can_redo;
  el.reset.disabled = !view.changed;
}

// ---- time of day ----------------------------------------------------------

const hhmm = (min) => `${String(Math.floor(min / 60) % 24).padStart(2, "0")}:${String(min % 60).padStart(2, "0")}`;
const toMin = (text) => {
  const [h, m] = text.split(":").map(Number);
  return Number.isFinite(h) && Number.isFinite(m) ? h * 60 + m : NaN;
};

// The slider sets the time the sheet shows. Under it, a bar marks when the
// selected piece is something other than its usual type.
function renderClock() {
  // Time only matters once some piece changes type through the day.
  const timed = view.segments.some((s) => s.variants.length > 0);
  $("clock").hidden = !timed;
  const t = $("time");
  t.value = view.time_min / 15;
  t.setAttribute("aria-valuetext", hhmm(view.time_min));
  $("time-out").textContent = hhmm(view.time_min);
  const s = view.selected ? seg(view.selected) : null;
  const bands = [];
  for (const v of s?.variants ?? []) {
    const c = `var(--k-${KINDS[v.kind].id})`;
    const [a, b] = [(v.from_min / 1440) * 100, (v.to_min / 1440) * 100];
    if (v.from_min < v.to_min) bands.push([a, b, c]);
    else bands.push([a, 100, c], [0, b, c]);
  }
  $("clock-bar").innerHTML = bands
    .map(([a, b, c]) => `<i style="left:${a}%;width:${b - a}%;background:${c}"></i>`)
    .join("");
  $("clock-bar").classList.toggle("on", bands.length > 0);
  $("clock-note").textContent = s?.variants.length
    ? `${KINDS[s.base_kind].name} except ${s.variants.map((v) => `${KINDS[v.kind].name.toLowerCase()} ${hhmm(v.from_min)}\u2013${hhmm(v.to_min)}`).join(", ")}`
    : "";
  $("time-note").textContent = timed ? `Numbers are for ${hhmm(view.time_min)}.` : "";
}

// "N checks fail" opens the Checks tab, and the notes column if it is closed.
$("fit-checks").addEventListener("click", () => {
  if (document.documentElement.dataset.notes === "closed") $("notes-toggle").click();
  $("t-checks").click();
  $("t-checks").focus();
});

$("time").addEventListener("input", (e) => {
  if (sheet.set_time(Number(e.target.value) * 15)) refresh();
});

function render() {
  keepSoon();
  renderMeasures();
  renderClock();
  renderHead();
  renderFit();
  renderDrawing();
  renderInspector();
  renderNotes();
}


// ---- inspector ------------------------------------------------------------

const surfaceSwatch = (id) =>
  `<svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect width="44" height="22" fill="var(--sheet)" stroke="none"/><rect width="44" height="22" fill="url(#m-${id})" stroke="none"/></svg>`;
const curbSwatch = (id) =>
  id === "none"
    ? `<svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><line x1="4" x2="40" y1="11" y2="11"/></svg>`
    : `<svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect width="44" height="22" fill="var(--sheet)" stroke="none"/><rect width="44" height="22" fill="url(#c-${id})" stroke="none"/></svg>`;

// Traffic direction as an arrow: up runs away from you, down comes toward you.
// The paler outline under the arrow keeps it readable over hatching.
const dirGlyph = (id, cx, cy) => {
  const d = id === "toward" ? 1 : -1;
  const path = `M${cx},${cy - 7 * d} V${cy + 7 * d} M${cx - 4.5},${cy + 2.5 * d} L${cx},${cy + 7 * d} L${cx + 4.5},${cy + 2.5 * d}`;
  return `<path class="dir-halo" d="${path}"/><path class="dir" d="${path}"/>`;
};
// Marks a piece that is a different type at other times.
const clockBadge = (cx, cy) =>
  `<circle class="badge" cx="${cx}" cy="${cy}" r="7"/><path class="badge-hands" d="M${cx},${cy - 4} V${cy} L${cx + 3},${cy + 2}"/>`;
const dirSwatch = (id) =>
  id === "both"
    ? `<svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><path class="dir-x" d="M8,11 H36 M12,7 L8,11 L12,15 M32,7 L36,11 L32,15"/></svg>`
    : `<svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false">${dirGlyph(id, 22, 11)}</svg>`;

const option = (fid, checked, swatchHtml, name, data) =>
  `<li><button type="button" class="opt" role="radio" aria-checked="${checked}" tabindex="${checked ? 0 : -1}" data-ifid="${fid}" ${data}>${swatchHtml}<span>${esc(name)}</span>${ICON.tick}</button></li>`;

const stepMm = () => (units === "m" ? 100 : 305);

// The rarer settings (other times, direction, curb) sit under one disclosure.
// It stays open once opened, and is open for a piece that already has other
// times, since those change what the drawing shows.
let inspectorMoreOpen = false;
const moreForced = () => {
  const s = view.selected ? seg(view.selected) : null;
  return !!s && s.variants.length > 0;
};
el.inspector.addEventListener(
  "toggle",
  (e) => {
    if (e.target.classList?.contains("insp-more") && !moreForced()) inspectorMoreOpen = e.target.open;
  },
  true,
);

function renderInspector() {
  const active = document.activeElement;
  const focusId = active && el.inspector.contains(active) ? active.dataset.ifid : null;
  const s = view.selected ? seg(view.selected) : null;
  if (!s) {
    el.inspector.style.removeProperty("--kc");
    el.inspector.innerHTML = `<p class="insp-empty">Select a piece to change its width and surface.</p>`;
    return;
  }
  const k = kindOf(s);
  el.inspector.style.setProperty("--kc", `var(--k-${k.id})`);
  const i = idxOf(s.uid);
  const lo = num(s.min_mm).toFixed(2);
  const hi = num(s.max_mm).toFixed(2);
  const surfaces = k.materials
    .map((m) => {
      const mat = MATERIALS.surfaces[m];
      return option(`s-${mat.id}`, mat.id === s.material, surfaceSwatch(mat.id), mat.name, `data-isurface="${m}"`);
    })
    .join("");
  const vdirSelect = (v, vi) => {
    const rule = KINDS[v.kind].direction;
    if (rule === "none") return "";
    const opts = MATERIALS.directions.map((dd) => `<option value="${dd.id}"${dd.id === v.direction ? " selected" : ""}>${esc(dd.name)}</option>`);
    if (rule === "optional") opts.push(`<option value="both"${v.direction == null ? " selected" : ""}>Two-way</option>`);
    return `<select data-ivdir="${vi}" data-ifid="vd-${vi}" aria-label="Direction ${vi + 1}">${opts.join("")}</select>`;
  };
  const kindOptions = (sel) => s.alt_kinds.concat(sel).filter((v, j, a) => a.indexOf(v) === j).sort((a, b) => a - b);
  const rows = s.variants
    .map(
      (v, vi) => `<li class="var${s.active_variant === vi ? " now" : ""}">
        <select data-ivkind="${vi}" data-ifid="vk-${vi}" aria-label="Type ${vi + 1}">${kindOptions(v.kind).map((kk) => `<option value="${kk}"${kk === v.kind ? " selected" : ""}>${esc(KINDS[kk].name)}</option>`).join("")}</select>
        ${vdirSelect(v, vi)}
        <span class="var-times"><input type="time" step="900" data-ivfrom="${vi}" data-ifid="vf-${vi}" value="${hhmm(v.from_min)}" aria-label="From"><span aria-hidden="true">to</span><input type="time" step="900" data-ivto="${vi}" data-ifid="vt-${vi}" value="${hhmm(v.to_min)}" aria-label="To"></span>
        <button type="button" class="ico danger" data-ivremove="${vi}" data-ifid="vr-${vi}" aria-label="Remove ${esc(KINDS[v.kind].name.toLowerCase())} at ${hhmm(v.from_min)} to ${hhmm(v.to_min)}">${ICON.remove}</button>
      </li>`,
    )
    .join("");
  const times = s.alt_kinds.length || s.variants.length
    ? `<section class="insp-sec"><h3 class="note-h" id="i-h-times">Other times</h3>
        <p class="insp-range">${esc(KINDS[s.base_kind].name)} the rest of the day.</p>
        <ul class="vars">${rows}</ul>
        <button type="button" class="btn" data-ivadd data-ifid="va"${s.alt_kinds.length ? "" : " disabled"}>${ICON.plus}Add other times</button>
      </section>`
    : "";
  let dirs = "";
  if (k.direction !== "none") {
    const rows = MATERIALS.directions.map((d, di) => option(`d-${d.id}`, d.id === s.direction, dirSwatch(d.id), d.name, `data-idir="${di}"`));
    if (k.direction === "optional") rows.push(option("d-both", s.direction == null, dirSwatch("both"), "Two-way", `data-idir="-1"`));
    dirs = `<section class="insp-sec"><h3 class="note-h" id="i-h-dir">Direction</h3><p class="insp-range">Which way traffic goes.</p><ul class="opts" role="radiogroup" aria-labelledby="i-h-dir">${rows.join("")}</ul></section>`;
  }
  let curbs = "";
  if (k.has_curb) {
    const rows = k.curbs.map((ci) => {
      const c = MATERIALS.curbs[ci];
      return option(`c-${c.id}`, c.id === s.curb, curbSwatch(c.id), c.name, `data-icurb="${ci}"`);
    });
    rows.push(option("c-none", s.curb == null, curbSwatch("none"), "None (flush)", `data-icurb="-1"`));
    curbs = `<section class="insp-sec"><h3 class="note-h" id="i-h-curb">Curb</h3><p class="insp-range">The raised edge, if it has one.</p><ul class="opts" role="radiogroup" aria-labelledby="i-h-curb">${rows.join("")}</ul></section>`;
  }
  const stop = s.can_shelter
    ? `<section class="insp-sec"><h3 class="note-h" id="i-h-stop">Transit stop</h3><label class="check"><input type="checkbox" data-ishelter data-ifid="shelter"${s.shelter ? " checked" : ""}>Shelter on this sidewalk</label></section>`
    : "";
  const vehicle = k.id === "bus"
    ? `<section class="insp-sec"><h3 class="note-h" id="i-h-veh">Vehicle</h3><div class="units" role="group" aria-labelledby="i-h-veh"><button type="button" class="unit" data-itram="0" data-ifid="veh-bus" aria-pressed="${!s.tram}">Bus</button><button type="button" class="unit" data-itram="1" data-ifid="veh-tram" aria-pressed="${s.tram}">Tram</button></div></section>`
    : "";
  const moreBody = times + dirs + curbs;
  const more = moreBody
    ? `<details class="insp-more"${inspectorMoreOpen || moreForced() ? " open" : ""}><summary>More about this piece</summary>${moreBody}</details>`
    : "";
  el.inspector.innerHTML = `
    <div class="insp-head">${swatch(k.id)}<div><h2 class="insp-name">${esc(k.name)}</h2><p class="insp-sub">${fmt(s.width_mm)} wide · ${i + 1} of ${view.segments.length}${s.variants.length ? ` · ${hhmm(view.time_min)}` : ""}</p></div></div>
    <section class="insp-sec">
      <h3 class="note-h" id="i-h-width">Width</h3>
      <div class="stepper">
        <button type="button" class="ico" data-istep="-1" data-ifid="minus" aria-label="Narrower by ${fmtN(stepMm())} ${units}">${ICON.minus}</button>
        <span class="wfield"><input type="number" inputmode="decimal" data-ifid="width" step="${units === "m" ? "0.1" : "0.25"}" min="${lo}" max="${hi}" value="${num(s.width_mm).toFixed(2)}" aria-labelledby="i-h-width"><span class="unit-tag" aria-hidden="true">${units}</span></span>
        <button type="button" class="ico" data-istep="1" data-ifid="plus" aria-label="Wider by ${fmtN(stepMm())} ${units}">${ICON.plus}</button>
      </div>
      <p class="insp-range">Allowed ${lo} to ${hi} ${units}</p>
    </section>
    <section class="insp-sec"><h3 class="note-h" id="i-h-surface">${k.id === "planting" ? "Planting" : "Surface"}</h3><p class="insp-range">${k.id === "planting" ? "What is planted in it." : "What it is paved with."}</p><ul class="opts" role="radiogroup" aria-labelledby="i-h-surface">${surfaces}</ul></section>
    <section class="insp-sec"><h3 class="note-h" id="i-h-pos">Position</h3><div class="move-btns"><button type="button" class="btn" data-imove="-1" data-ifid="move-left"${i === 0 ? " disabled" : ""}>${ICON.left}Move left</button><button type="button" class="btn" data-imove="1" data-ifid="move-right"${i === view.segments.length - 1 ? " disabled" : ""}>Move right${ICON.right}</button></div></section>
    ${vehicle}
    ${stop}
    ${more}`;
  if (focusId) {
    const t = el.inspector.querySelector(`[data-ifid="${focusId}"]`);
    if (t && !t.disabled) t.focus({ preventScroll: true });
  }
}

el.inspector.addEventListener("click", (e) => {
  const uid = view.selected;
  const b = e.target.closest("button");
  if (!b || !uid) return;
  let ok = false;
  if (b.dataset.imove) moveBy(uid, Number(b.dataset.imove));
  else if (b.dataset.itram) ok = sheet.set_tram(uid, b.dataset.itram === "1");
  else if (b.dataset.istep) ok = sheet.nudge_width(uid, Number(b.dataset.istep) * stepMm());
  else if (b.dataset.isurface) ok = sheet.set_material(uid, Number(b.dataset.isurface));
  else if (b.dataset.icurb) ok = sheet.set_curb(uid, Number(b.dataset.icurb));
  else if (b.dataset.idir) ok = sheet.set_direction(uid, Number(b.dataset.idir));
  else if ("ivadd" in b.dataset) ok = sheet.add_variant(uid);
  else if (b.dataset.ivremove) ok = sheet.remove_variant(uid, Number(b.dataset.ivremove));
  if (ok) {
    refresh();
    announceEdit();
  }
});

el.inspector.addEventListener("change", (e) => {
  const input = e.target.closest("input, select");
  const uid = view.selected;
  if (!input || !uid) return;
  const d = input.dataset;
  if (d.ishelter !== undefined) {
    const ok = sheet.set_shelter(uid, input.checked);
    refresh();
    if (ok) announceEdit();
    return;
  }
  if (d.ivdir !== undefined) {
    const di = input.value === "both" ? -1 : MATERIALS.directions.findIndex((x) => x.id === input.value);
    const ok = sheet.set_variant_direction(uid, Number(d.ivdir), di);
    refresh();
    if (ok) announceEdit();
    return;
  }
  if (d.ivkind !== undefined || d.ivfrom !== undefined || d.ivto !== undefined) {
    const vi = Number(d.ivkind ?? d.ivfrom ?? d.ivto);
    const v = seg(uid).variants[vi];
    const ok =
      d.ivkind !== undefined
        ? sheet.set_variant_kind(uid, vi, Number(input.value))
        : sheet.set_variant_time(uid, vi, d.ivfrom !== undefined ? toMin(input.value) : v.from_min, d.ivto !== undefined ? toMin(input.value) : v.to_min);
    refresh();
    if (ok) announceEdit();
    return;
  }
  const v = parseFloat(input.value);
  if (!Number.isFinite(v)) return renderInspector();
  const before = seg(uid).width_mm;
  sheet.set_width(uid, fromInput(v));
  refresh();
  if (seg(uid)?.width_mm !== before) announceEdit();
});

// Radio groups: arrows move the choice, as native radios do.
el.inspector.addEventListener("keydown", (e) => {
  const cur = e.target.closest('[role="radio"]');
  if (!cur || e.metaKey || e.ctrlKey || e.altKey) return;
  const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
  if (!step) return;
  e.preventDefault();
  const group = [...cur.closest('[role="radiogroup"]').querySelectorAll('[role="radio"]')];
  const next = group[(group.indexOf(cur) + step + group.length) % group.length];
  next.focus();
  next.click();
});

// ---- actions --------------------------------------------------------------

function select(uid) {
  sheet.select(uid || 0);
  view = JSON.parse(sheet.view());
  renderDrawing();
  renderInspector();
}

function insertIndex() {
  const i = selIndex();
  return i < 0 ? view.segments.length : i + 1;
}

function addKind(kind, index) {
  sheet.add(kind, index);
  refresh();
  announceEdit();
}

function removeSel(uid = view.selected) {
  if (!uid) return;
  if (sheet.remove(uid)) {
    refresh();
    announceEdit();
  }
}

function moveBy(uid, delta) {
  const i = idxOf(uid);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= view.segments.length) return;
  if (sheet.move_to(uid, j)) {
    refresh();
    announceEdit();
  }
}

function nudge(uid, mm) {
  if (uid && sheet.nudge_width(uid, mm)) {
    refresh();
    announceEdit();
  }
}

function undo() {
  if (sheet.undo()) {
    refresh();
    say(`Undone. ${fitText()}.`);
  }
}
function redo() {
  if (sheet.redo()) {
    refresh();
    say(`Redone. ${fitText()}.`);
  }
}

// ---- pointer: section -----------------------------------------------------

// The first-run cue goes when the resident dismisses it or has made a change
// to the street (the first revision); it is remembered so it does not return.
var welcomePinned = false; // var: render runs before this line
function dismissWelcome() {
  welcomePinned = false;
  document.documentElement.dataset.welcome = "seen";
  remember("cityloom-welcome", "seen");
}
// "How this works" in the settings menu brings the cue back, and keeps it
// until it is dismissed, however much has been edited.
$("how-btn").addEventListener("click", () => {
  welcomePinned = true;
  delete document.documentElement.dataset.welcome;
  remember("cityloom-welcome", "show");
  $("welcome").scrollIntoView({ block: "nearest" });
  $("welcome-dismiss").focus();
});
$("welcome-dismiss").addEventListener("click", () => {
  dismissWelcome();
  el.wrap.focus();
});

const DRAG_START_PX = 4;

el.svg.addEventListener("pointerdown", (e) => {
  if (e.pointerType === "mouse" && e.button !== 0) return;
  const t = e.target.closest("[data-role]");
  el.wrap.focus({ preventScroll: true });
  if (!t) return;
  const role = t.dataset.role;
  if (role === "handle") {
    sheet.begin_gesture();
    drag = { type: "resize", i: Number(t.dataset.i), startX: e.clientX };
  } else if (role === "edge") {
    sheet.begin_gesture();
    drag = { type: "edge", uid: Number(t.dataset.uid), startX: e.clientX };
  } else if (role === "seg") {
    const uid = Number(t.dataset.uid);
    select(uid);
    announceSelection();
    drag = { type: "move", uid, startX: e.clientX, active: false };
  }
  try {
    el.svg.setPointerCapture(e.pointerId);
  } catch {
    // Not an active pointer (synthetic input); events still reach the svg.
  }
  e.preventDefault();
  if (drag && drag.type !== "move") renderDrawing();
});

el.svg.addEventListener("pointermove", (e) => {
  if (!drag) return;
  const dx = e.clientX - drag.startX;
  if (drag.type === "resize") {
    sheet.resize_boundary(drag.i, Math.round(dx / L.scale));
    refresh();
  } else if (drag.type === "edge") {
    sheet.resize_edge(drag.uid, Math.round(dx / L.scale));
    refresh();
  } else if (drag.type === "move") {
    if (!drag.active && Math.abs(dx) < DRAG_START_PX) return;
    drag.active = true;
    drag.px = e.clientX - el.svg.getBoundingClientRect().left;
    drag.idx = sheet.drop_index(mmAt(e.clientX), drag.uid);
    renderDrawing();
  }
});

function finishPointer(commit) {
  if (!drag) return;
  const d = drag;
  drag = null;
  if (d.type === "resize" || d.type === "edge") {
    if (commit) {
      if (sheet.end_gesture()) {
        refresh();
        announceEdit();
        return;
      }
    } else {
      sheet.cancel_gesture();
    }
    refresh();
  } else if (d.type === "move") {
    if (commit && d.active && sheet.move_to(d.uid, d.idx)) {
      refresh();
      announceEdit();
    } else {
      renderDrawing();
    }
  }
}
el.svg.addEventListener("pointerup", () => finishPointer(true));
el.svg.addEventListener("pointercancel", () => finishPointer(false));

// ---- add menu ---------------------------------------------------------------

const addBtn = $("add-btn");
const addMenu = $("add-menu");
const addItems = () => [...el.palette.querySelectorAll(".add-item")];
function setAddOpen(open, refocus = false) {
  addMenu.hidden = !open;
  addBtn.setAttribute("aria-expanded", String(open));
  if (open) addItems()[0]?.focus();
  else if (refocus) addBtn.focus();
}
addBtn.addEventListener("click", () => setAddOpen(addMenu.hidden));
addBtn.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown" && addMenu.hidden) {
    e.preventDefault();
    setAddOpen(true);
  }
});
addMenu.addEventListener("keydown", (e) => {
  const items = addItems();
  const at = items.indexOf(document.activeElement);
  const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: items.length - 1 }[e.key];
  if (to === undefined) return;
  e.preventDefault();
  items[(to + items.length) % items.length].focus();
});
document.addEventListener("pointerdown", (e) => {
  if (!addMenu.hidden && !addMenu.contains(e.target) && !addBtn.contains(e.target)) setAddOpen(false);
});
addMenu.addEventListener("focusout", (e) => {
  if (e.relatedTarget && !addMenu.contains(e.relatedTarget) && e.relatedTarget !== addBtn) setAddOpen(false);
});
el.palette.addEventListener("click", (e) => {
  const item = e.target.closest(".add-item");
  if (!item) return;
  setAddOpen(false, true);
  addKind(Number(item.dataset.kind), insertIndex());
});

// ---- keyboard -------------------------------------------------------------


el.wrap.addEventListener("keydown", (e) => {
  const uid = view.selected;
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowRight": {
      e.preventDefault();
      const d = e.key === "ArrowLeft" ? -1 : 1;
      if (e.shiftKey) {
        if (uid) moveBy(uid, d);
      } else {
        sheet.select_relative(d);
        view = JSON.parse(sheet.view());
        select(view.selected);
        announceSelection();
      }
      break;
    }
    case "+":
    case "=":
      e.preventDefault();
      nudge(uid, e.shiftKey ? 500 : 100);
      break;
    case "-":
    case "_":
      e.preventDefault();
      nudge(uid, e.shiftKey ? -500 : -100);
      break;
    case "Enter": {
      // Straight to the width field, opening the details if they are hidden.
      if (!uid) break;
      e.preventDefault();
      if (document.documentElement.dataset.inspector === "closed") $("inspector-toggle").click();
      const field = el.inspector.querySelector('[data-ifid="width"]');
      field?.focus();
      field?.select();
      break;
    }
    case "Delete":
    case "Backspace":
      e.preventDefault();
      removeSel();
      break;
    case "Escape":
      if (drag) {
        finishPointer(false);
      } else {
        select(0);
      }
      break;
  }
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !addMenu.hidden) setAddOpen(false, true);
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

// ---- header controls ------------------------------------------------------

el.undo.addEventListener("click", undo);
el.redo.addEventListener("click", redo);
el.reset.addEventListener("click", () => {
  if (sheet.reset()) {
    refresh();
    say("Started over from the street as it is today. Undo brings your changes back.");
  }
});
initUnits((u) => {
  units = u;
  renderPalette();
  render();
});
initRegion({
  regions: MATERIALS.regions,
  apply: (i) => sheet.set_region(i),
  current: () => view.region,
  onChange: refresh,
  say,
});
initTheme(say);
initDrawingStyle(say, renderDrawing);

new ResizeObserver(() => renderDrawing()).observe(el.scroll);

renderPalette();
render();

initAccountMenu();
initPanels({ say });

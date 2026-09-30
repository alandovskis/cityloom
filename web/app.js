// Draws the view the Rust core returns and relays pointer and keyboard input.
// No editing rules live here: widths, snapping, limits, history and checks all
// come from the WebAssembly model.

import init, { Sheet, catalogue, materials } from "./pkg/cityloom_editor.js";
import { CURB_HATCH, HATCH, MATERIAL_HATCH, symbol } from "./symbols.js";

await init();

const KINDS = JSON.parse(catalogue());
const MATERIALS = JSON.parse(materials());
const sheet = new Sheet(0);
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
  grip: `<svg class="grip-ico" viewBox="0 0 10 14" aria-hidden="true"><path d="M2 2h.01M8 2h.01M2 7h.01M8 7h.01M2 12h.01M8 12h.01"/></svg>`,
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
        symbol(k.id, cx, G, pxPerM, w, s.width_mm / 1000, F) +
        `<rect class="obj k-${k.id}" x="${x}" y="${G}" width="${w}" height="${Y.slab}"/>` +
        `<rect class="hatch" x="${x}" y="${G}" width="${w}" height="${Y.slab}" fill="url(#${engineering() ? `m-${s.material}` : `h-${k.id}`})"/>` +
        (engineering() ? "" : surfaceCourse(x, G, w, s.material)) +
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
    parts.push(
      `<g class="handle${active}" ${attrs}>${line}` +
        `<rect class="hit" x="${x - 9}" y="${Y.dim - 8}" width="18" height="${bodyBottom - (Y.dim - 8)}"/>` +
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
      // a bike-friendly curb is a low ramp a wheel can ride up
      const ramp = s.curb === "bikefriendly";
      const cw = Math.min(planted ? Math.max(14, 600 * L.scale) : kassel ? Math.max(9, 250 * L.scale) : ramp ? Math.max(9, 300 * L.scale) : Math.max(5, 150 * L.scale), w / 2);
      const fill = planted ? "m-planted" : `c-${s.curb}`;
      const sides = [
        [v.segments[i - 1], x, false],
        [v.segments[i + 1], x + w - cw, true],
      ];
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
          `<rect class="curb" x="${bx}" y="${G - 12}" width="${cw}" height="12"/>` +
            `<rect class="hatch" x="${bx}" y="${G - 12}" width="${cw}" height="12" fill="url(#${fill})"/>`,
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
  if (drag && drag.idx != null && ((drag.type === "move" && drag.active) || (drag.type === "new" && drag.over))) {
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

// ---- legend --------------------------------------------------

function renderPalette() {
  el.palette.innerHTML = KINDS.map(
    (k, i) =>
      `<li><button type="button" class="chip" data-kind="${i}">${swatch(k.id)}<b>${esc(k.name)}</b><span class="dw">${fmt(k.default_mm)}</span>${ICON.grip}</button></li>`,
  ).join("");
}

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

function renderHead() {
  $("street-name").textContent = view.name;
  $("row-dim").textContent = fmt(view.row_mm);
  $("tb-street").textContent = view.name;
  $("tb-row").textContent = fmt(view.row_mm);
  el.undo.disabled = !view.can_undo;
  el.redo.disabled = !view.can_redo;
  el.reset.disabled = !view.changed;
}

function render() {
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

const option = (fid, checked, swatchHtml, name, data) =>
  `<li><button type="button" class="opt" role="radio" aria-checked="${checked}" tabindex="${checked ? 0 : -1}" data-ifid="${fid}" ${data}>${swatchHtml}<span>${esc(name)}</span>${ICON.tick}</button></li>`;

const stepMm = () => (units === "m" ? 100 : 305);

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
  let curbs = "";
  if (k.has_curb) {
    const rows = MATERIALS.curbs.map((c, ci) => option(`c-${c.id}`, c.id === s.curb, curbSwatch(c.id), c.name, `data-icurb="${ci}"`));
    rows.push(option("c-none", s.curb == null, curbSwatch("none"), "None (flush)", `data-icurb="-1"`));
    curbs = `<section class="insp-sec"><h3 class="note-h" id="i-h-curb">Curb</h3><ul class="opts" role="radiogroup" aria-labelledby="i-h-curb">${rows.join("")}</ul></section>`;
  }
  el.inspector.innerHTML = `
    <div class="insp-head">${swatch(k.id)}<div><h2 class="insp-name">${esc(k.name)}</h2><p class="insp-sub">${fmt(s.width_mm)} wide · ${i + 1} of ${view.segments.length}</p></div></div>
    <section class="insp-sec">
      <h3 class="note-h" id="i-h-width">Width</h3>
      <div class="stepper">
        <button type="button" class="ico" data-istep="-1" data-ifid="minus" aria-label="Narrower by ${fmtN(stepMm())} ${units}">${ICON.minus}</button>
        <span class="wfield"><input type="number" inputmode="decimal" data-ifid="width" step="${units === "m" ? "0.1" : "0.25"}" min="${lo}" max="${hi}" value="${num(s.width_mm).toFixed(2)}" aria-labelledby="i-h-width"><span class="unit-tag" aria-hidden="true">${units}</span></span>
        <button type="button" class="ico" data-istep="1" data-ifid="plus" aria-label="Wider by ${fmtN(stepMm())} ${units}">${ICON.plus}</button>
      </div>
      <p class="insp-range">Allowed ${lo} to ${hi} ${units}</p>
    </section>
    <section class="insp-sec"><h3 class="note-h" id="i-h-surface">Surface</h3><ul class="opts" role="radiogroup" aria-labelledby="i-h-surface">${surfaces}</ul></section>
    ${curbs}`;
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
  if (b.dataset.istep) ok = sheet.nudge_width(uid, Number(b.dataset.istep) * stepMm());
  else if (b.dataset.isurface) ok = sheet.set_material(uid, Number(b.dataset.isurface));
  else if (b.dataset.icurb) ok = sheet.set_curb(uid, Number(b.dataset.icurb));
  if (ok) {
    refresh();
    announceEdit();
  }
});

el.inspector.addEventListener("change", (e) => {
  const input = e.target.closest("input");
  const uid = view.selected;
  if (!input || !uid) return;
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

// ---- pointer: legend ------------------------------------------------------

let chipDrag = null;
let suppressClick = false;

el.palette.addEventListener("pointerdown", (e) => {
  const chip = e.target.closest(".chip");
  if (!chip || (e.pointerType === "mouse" && e.button !== 0)) return;
  chipDrag = { kind: Number(chip.dataset.kind), startX: e.clientX, startY: e.clientY, active: false, ghost: null };
  try {
    chip.setPointerCapture(e.pointerId);
  } catch {
    // Not an active pointer (synthetic input).
  }
});

el.palette.addEventListener("pointermove", (e) => {
  if (!chipDrag) return;
  const c = chipDrag;
  if (!c.active) {
    if (Math.hypot(e.clientX - c.startX, e.clientY - c.startY) < DRAG_START_PX) return;
    c.active = true;
    c.ghost = document.createElement("div");
    c.ghost.className = "drag-chip";
    c.ghost.innerHTML = `${swatch(KINDS[c.kind].id)}<span>${esc(KINDS[c.kind].name)}</span>`;
    c.ghost.style.cssText = "";
    document.body.append(c.ghost);
  }
  c.ghost.style.left = `${e.clientX}px`;
  c.ghost.style.top = `${e.clientY}px`;
  const r = el.scroll.getBoundingClientRect();
  const over = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
  drag = { type: "new", kind: c.kind, over, idx: over ? sheet.drop_index(mmAt(e.clientX), 0) : null };
  renderDrawing();
});

function endChipDrag(commit) {
  const c = chipDrag;
  if (!c) return;
  chipDrag = null;
  if (!c.active) return;
  suppressClick = true;
  c.ghost.remove();
  const d = drag;
  drag = null;
  if (commit && d && d.over) addKind(d.kind, d.idx);
  else renderDrawing();
}
el.palette.addEventListener("pointerup", () => endChipDrag(true));
el.palette.addEventListener("pointercancel", () => endChipDrag(false));

el.palette.addEventListener("click", (e) => {
  const chip = e.target.closest(".chip");
  if (!chip) return;
  if (suppressClick) {
    suppressClick = false;
    return;
  }
  addKind(Number(chip.dataset.kind), insertIndex());
});

// ---- keyboard -------------------------------------------------------------

const typing = (t) => t instanceof Element && t.closest("input, select, textarea");

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
      nudge(uid, 100);
      break;
    case "-":
    case "_":
      e.preventDefault();
      nudge(uid, -100);
      break;
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
  if (e.key === "Escape" && chipDrag?.active) endChipDrag(false);
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
    say("Started over from the street as it is today.");
  }
});
for (const b of document.querySelectorAll(".unit[data-unit]")) {
  b.addEventListener("click", () => {
    units = b.dataset.unit;
    for (const o of document.querySelectorAll(".unit[data-unit]")) o.setAttribute("aria-pressed", String(o === b));
    renderPalette();
    render();
  });
}

// Theme: follow the system until the person picks one; the pick is remembered.
const root = document.documentElement;
const dark = matchMedia("(prefers-color-scheme: dark)");
const theme = () => root.dataset.theme || (dark.matches ? "dark" : "light");
function syncTheme() {
  for (const b of document.querySelectorAll(".theme")) b.setAttribute("aria-pressed", String(b.dataset.themeSet === theme()));
}
for (const b of document.querySelectorAll(".theme")) {
  b.addEventListener("click", () => {
    root.dataset.theme = b.dataset.themeSet;
    try {
      localStorage.setItem("cityloom-theme", b.dataset.themeSet);
    } catch {
      // Storage can be blocked; the choice then lasts for this visit only.
    }
    syncTheme();
    say(`${b.textContent} theme.`);
  });
}
dark.addEventListener("change", syncTheme);
syncTheme();

// Drawing style: the same section as a plain engineering drawing; remembered.
const engineering = () => root.dataset.drawing === "engineering";
function syncDrawing() {
  for (const b of document.querySelectorAll(".drawing-mode")) {
    b.setAttribute("aria-pressed", String((b.dataset.drawingSet === "engineering") === engineering()));
  }
}
for (const b of document.querySelectorAll(".drawing-mode")) {
  b.addEventListener("click", () => {
    if (b.dataset.drawingSet === "engineering") root.dataset.drawing = "engineering";
    else delete root.dataset.drawing;
    try {
      localStorage.setItem("cityloom-drawing", b.dataset.drawingSet);
    } catch {
      // Storage can be blocked; the choice then lasts for this visit only.
    }
    syncDrawing();
    renderDrawing();
    say(`${b.textContent} drawing.`);
  });
}
syncDrawing();

new ResizeObserver(() => renderDrawing()).observe(el.scroll);

renderPalette();
render();

// Account menu: a disclosure from the avatar that holds the unit toggle.
{
  const btn = $("account-btn");
  const menu = $("account-menu");
  const setOpen = (open, refocus = false) => {
    menu.hidden = !open;
    btn.setAttribute("aria-expanded", String(open));
    if (!open && refocus) btn.focus();
  };
  btn.addEventListener("click", () => setOpen(menu.hidden));
  document.addEventListener("pointerdown", (e) => {
    if (!menu.hidden && !menu.contains(e.target) && !btn.contains(e.target)) setOpen(false);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !menu.hidden) {
      e.stopPropagation();
      setOpen(false, true);
    }
  });
  menu.addEventListener("focusout", (e) => {
    if (e.relatedTarget && !menu.contains(e.relatedTarget) && e.relatedTarget !== btn) setOpen(false);
  });
}

// Piece details sidebar: collapsible from a header button or the [ key; remembered.
{
  const btn = $("inspector-toggle");
  const label = $("inspector-label");
  const sync = () => {
    const open = root.dataset.inspector !== "closed";
    btn.setAttribute("aria-expanded", String(open));
    label.textContent = open ? "Hide piece details" : "Show piece details";
  };
  const toggle = () => {
    const open = root.dataset.inspector === "closed";
    if (open) delete root.dataset.inspector;
    else root.dataset.inspector = "closed";
    try {
      localStorage.setItem("cityloom-inspector", open ? "open" : "closed");
    } catch {}
    sync();
    say(open ? "Piece details shown." : "Piece details hidden.");
  };
  btn.addEventListener("click", toggle);
  document.addEventListener("keydown", (e) => {
    if (e.key !== "[" || e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return;
    e.preventDefault();
    toggle();
  });
  sync();
}

// Print: the print stylesheet lays the sheet out; this is only the trigger.
$("print").addEventListener("click", () => window.print());

// Notes tabs: Space, Checks, Changes. Arrow keys move between tabs; the choice is remembered.
{
  const tabs = [...document.querySelectorAll(".notes .tab")];
  const show = (tab, focus) => {
    for (const t of tabs) {
      const on = t === tab;
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
      $(t.getAttribute("aria-controls")).hidden = !on;
    }
    if (focus) tab.focus();
    try {
      localStorage.setItem("cityloom-notes-tab", tab.id);
    } catch {}
  };
  let saved = null;
  try {
    saved = localStorage.getItem("cityloom-notes-tab");
  } catch {}
  show(tabs.find((t) => t.id === saved) || tabs[0], false);
  tabs.forEach((t, i) => {
    t.addEventListener("click", () => show(t, false));
    t.addEventListener("keydown", (e) => {
      const to = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 }[e.key];
      if (to === undefined) return;
      e.preventDefault();
      show(tabs[(to + tabs.length) % tabs.length], true);
    });
  });
}

// Notes sidebar: collapsible from a header button or the ] key; remembered.
{
  const root = document.documentElement;
  const btn = $("notes-toggle");
  const label = $("notes-label");
  const sync = () => {
    const open = root.dataset.notes !== "closed";
    btn.setAttribute("aria-expanded", String(open));
    label.textContent = open ? "Hide notes" : "Show notes";
  };
  const toggle = () => {
    const open = root.dataset.notes === "closed";
    if (open) delete root.dataset.notes;
    else root.dataset.notes = "closed";
    try {
      localStorage.setItem("cityloom-notes", open ? "open" : "closed");
    } catch {}
    sync();
    say(open ? "Notes shown." : "Notes hidden.");
  };
  btn.addEventListener("click", toggle);
  document.addEventListener("keydown", (e) => {
    if (e.key !== "]" || e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return;
    e.preventDefault();
    toggle();
  });
  sync();
}

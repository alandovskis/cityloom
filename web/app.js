// Draws the view the Rust core returns and relays pointer and keyboard input.
// No editing rules live here: widths, snapping, limits, history and checks all
// come from the WebAssembly model.

import init, { Sheet, catalogue, samples } from "./pkg/cityloom_editor.js";
import { HATCH, symbol } from "./symbols.js";

await init();

const KINDS = JSON.parse(catalogue());
const SAMPLES = JSON.parse(samples());
const sheet = new Sheet(0);
let view = JSON.parse(sheet.view());
let units = "m";

const $ = (id) => document.getElementById(id);
const el = {
  svg: $("drawing"),
  wrap: $("wrap"),
  scroll: $("scroll"),
  sched: $("sched-body"),
  schedEmpty: $("sched-empty"),
  palette: $("palette"),
  space: $("space"),
  cap: $("cap"),
  checks: $("checks"),
  revs: $("revs"),
  live: $("live"),
  sample: $("sample"),
  undo: $("undo"),
  redo: $("redo"),
  reset: $("reset"),
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

$("defs").innerHTML = `<defs>${Object.values(HATCH).join("")}</defs>`;

const swatch = (kindId) =>
  `<svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><rect width="44" height="22" fill="url(#h-${kindId})" stroke="none"/></svg>`;

const ICON = {
  left: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M12 7H2M6 3 2 7l4 4"/></svg>`,
  right: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10M8 3l4 4-4 4"/></svg>`,
  remove: `<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 3l8 8M11 3l-8 8"/></svg>`,
  grip: `<svg class="grip-ico" viewBox="0 0 10 14" aria-hidden="true"><path d="M2 2h.01M8 2h.01M2 7h.01M8 7h.01M2 12h.01M8 12h.01"/></svg>`,
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
  if (d === 0) return "The street is exactly full";
  if (d < 0) return `${fmt(-d)} unassigned`;
  return `${fmt(d)} over the right-of-way. Narrow or remove ${fmt(d)}`;
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
  propLabel: 104,
  dim: 156,
  top: 172,
  ground: 312,
  slab: 34,
  mark: 368,
  total: 408,
  total2: 438,
  scale: 484,
  height: 512,
};
const Y = { ...Y0 };
let F = 1;
function setGeometry(f) {
  F = f;
  for (const k in Y0) Y[k] = Math.round(Y0[k] * f);
}
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

function renderDrawing() {
  const v = view;
  const W = Math.max(el.wrap.clientWidth, 680);
  L.width = W;
  setGeometry(Math.min(Math.max(W / 900, 1), 1.3));
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
  parts.push(`<text class="t-label t-soft" x="${L.padL}" y="${Y.exLabel}">Existing</text>`);
  for (const s of v.existing) {
    const k = KINDS[s.kind];
    const x = X(s.x_mm);
    const w = s.width_mm * L.scale;
    parts.push(
      `<rect class="obj" x="${x}" y="${Y.ex}" width="${w}" height="${Y.exH}"/>` +
        `<rect class="hatch" x="${x}" y="${Y.ex}" width="${w}" height="${Y.exH}" fill="url(#h-${k.id})"/>`,
    );
    const label = fmtN(s.width_mm);
    if (w > label.length * 8.5 + 10) {
      parts.push(`<text class="t-dim t-halo" x="${x + w / 2}" y="${Y.ex + Y.exH / 2 + 4.5}" text-anchor="middle">${label}</text>`);
    }
  }

  // proposal heading
  const rev = v.revisions.at(-1);
  parts.push(
    `<text class="t-label" x="${L.padL + 30}" y="${Y.propLabel}">Proposed</text>` +
      (rev ? `<text class="t-label t-blue" x="${L.padL + 116}" y="${Y.propLabel}">Rev ${esc(rev.letter)}</text>` : ""),
  );

  // right-of-way lines
  const xL = X(0);
  const xR = X(v.row_mm);
  for (const x of [xL, xR]) {
    parts.push(`<line class="rw" x1="${x}" x2="${x}" y1="${Y.dim - 34}" y2="${Y.total2 + 6}"/>`);
  }
  parts.push(
    `<text class="t-label t-faint" x="${xL}" y="${Y.dim - 42}" text-anchor="middle">R/W</text>` +
      `<text class="t-label t-faint" x="${xR}" y="${Y.dim - 42}" text-anchor="middle">R/W</text>`,
  );

  // ground
  parts.push(`<line class="ground" x1="${xL - 14}" x2="${Math.min(Math.max(X(v.total_mm), xR) + 14, W - 4)}" y1="${G}" y2="${G}"/>`);

  // unassigned space
  if (v.delta_mm < 0) {
    const x = X(v.total_mm);
    const w = xR - x;
    parts.push(`<rect class="free" x="${x}" y="${Y.top}" width="${w}" height="${bodyBottom - Y.top}"/>`);
    const t = `Unassigned ${fmt(-v.delta_mm)}`;
    if (w >= 88) {
      parts.push(
        `<text class="t-label t-faint" x="${x + w / 2}" y="${G - 8}" text-anchor="middle">Unassigned</text>` +
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
        `<rect class="obj" x="${x}" y="${G}" width="${w}" height="${Y.slab}"/>` +
        `<rect class="hatch" x="${x}" y="${G}" width="${w}" height="${Y.slab}" fill="url(#h-${k.id})"/>` +
        `<text class="t-mark t-halo${sel ? " t-blue" : ""}" x="${cx}" y="${Y.mark}" text-anchor="middle">${k.mark}</text>` +
        `</g>`,
    );
    parts.push(dimLine(x, x + w, Y.dim));
    const label = fmtN(s.width_mm);
    if (w > label.length * 9 + 8) {
      parts.push(`<text class="t-dim t-halo${sel ? " t-blue" : ""}" x="${cx}" y="${Y.dim - 7}" text-anchor="middle">${label}</text>`);
    }
    if (sel) {
      parts.push(`<rect class="sel-box" x="${x}" y="${Y.dim - 20}" width="${w}" height="${Y.mark + 10 - (Y.dim - 20)}"/>`);
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
    const top = Y.dim - 26;
    const h = Y.mark + 14 - top;
    parts.push(`<rect class="over-wash" x="${x}" y="${top}" width="${w}" height="${h}"/>`);
    parts.push(cloud(x + 5, top, Math.max(w - 5, 14), h, 6, fresh));
    parts.push(
      `<text class="t-over" x="${xR + 10}" y="${Y.dim - 62}">${signed(v.delta_mm)} ${units} over${clipped ? " (continues)" : ""}</text>`,
    );
  }

  // overall dimension strings
  parts.push(dimLine(xL, xR, Y.total));
  parts.push(`<text class="t-dim t-halo" x="${(xL + xR) / 2}" y="${Y.total - 7}" text-anchor="middle">R/W ${fmt(v.row_mm)}</text>`);
  if (v.delta_mm !== 0) {
    const cls = over ? "dim-red" : "";
    parts.push(dimLine(xL, X(v.total_mm), Y.total2, cls));
    parts.push(
      `<text class="t-dim t-halo ${over ? "t-red" : "t-soft"}" x="${(xL + X(v.total_mm)) / 2}" y="${Y.total2 - 7}" text-anchor="middle">Proposed ${fmt(v.total_mm)}</text>`,
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
  el.svg.setAttribute("height", Y.height);
  el.svg.setAttribute("viewBox", `0 0 ${W} ${Y.height}`);
  el.svg.setAttribute(
    "aria-label",
    `Cross-section of ${v.name}. ${n} segments, ${fmt(v.total_mm)} of ${fmt(v.row_mm)}. ${fitText()}.`,
  );
  el.svg.innerHTML = parts.join("");
}

// ---- schedule and legend --------------------------------------------------

function renderSchedule() {
  const focusId = document.activeElement?.dataset?.fid;
  const n = view.segments.length;
  el.schedEmpty.hidden = n > 0;
  el.sched.innerHTML = view.segments
    .map((s, i) => {
      const k = kindOf(s);
      const sel = s.uid === view.selected;
      const lo = num(s.min_mm).toFixed(2);
      const hi = num(s.max_mm).toFixed(2);
      return `<tr data-uid="${s.uid}"${sel ? ' class="sel" aria-selected="true"' : ""}>
        <td class="mark">${k.mark}</td>
        <td>${swatch(k.id)}</td>
        <td class="name">${esc(k.name)}</td>
        <td class="num"><span class="wfield"><input type="number" inputmode="decimal" data-fid="w-${s.uid}" step="${units === "m" ? "0.1" : "0.25"}" min="${lo}" max="${hi}" value="${num(s.width_mm).toFixed(2)}" aria-label="Width of ${esc(k.name.toLowerCase())}, ${i + 1} of ${n}, in ${unitWord()}"><span class="unit-tag" aria-hidden="true">${units}</span></span></td>
        <td><div class="acts">
          <button type="button" class="ico" data-act="earlier" data-fid="e-${s.uid}" aria-label="Move ${esc(k.name.toLowerCase())} earlier"${i === 0 ? " disabled" : ""}>${ICON.left}</button>
          <button type="button" class="ico" data-act="later" data-fid="l-${s.uid}" aria-label="Move ${esc(k.name.toLowerCase())} later"${i === n - 1 ? " disabled" : ""}>${ICON.right}</button>
          <button type="button" class="ico danger" data-act="remove" data-fid="r-${s.uid}" aria-label="Remove ${esc(k.name.toLowerCase())}">${ICON.remove}</button>
        </div></td>
      </tr>`;
    })
    .join("");
  restoreFocus(focusId);
}

function restoreFocus(fid) {
  if (!fid) return;
  const t = el.sched.querySelector(`[data-fid="${fid}"]`);
  if (t && !t.disabled) {
    t.focus({ preventScroll: true });
    return;
  }
  // The control went away or is now disabled: fall back to the selection, then the drawing.
  const uid = view.selected;
  const alt = uid && el.sched.querySelector(`[data-fid="w-${uid}"]`);
  (alt || el.wrap).focus({ preventScroll: true });
}

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
    if (c.ok) return view.delta_mm === 0 ? "Exactly full" : `${fmt(c.amount_mm)} unassigned`;
    return `${fmt(c.amount_mm)} over. Narrow or remove ${fmt(c.amount_mm)}.`;
  }
  if (c.id === "access") return c.ok ? `A lane of ${fmt(c.amount_mm)} or more` : `No lane of ${fmt(c.amount_mm)} or more`;
  return c.detail;
}

function renderNotes() {
  const o = view.outcomes;
  el.space.innerHTML =
    `<caption class="sr-only">Width by use, in ${unitWord()}</caption>` +
    head(["Use", "Existing", "Proposed", "Change"]) +
    `<tbody>${o.share
      .map((s, i) => {
        const ex = o.existing_share[i].mm;
        return `<tr><td>${s.label}</td><td>${fmtN(ex)}</td><td>${fmtN(s.mm)}</td>${changeCell(s.mm - ex, signed(s.mm - ex))}</tr>`;
      })
      .join("")}</tbody>`;

  const d = o.capacity_pph - o.existing_capacity_pph;
  const pct = o.existing_capacity_pph ? Math.round((d * 100) / o.existing_capacity_pph) : 0;
  const fmtInt = (n) => n.toLocaleString("en-US");
  el.cap.innerHTML =
    `<caption class="sr-only">People per hour, placeholder rates</caption>` +
    head(["", "Existing", "Proposed", "Change"]) +
    `<tbody><tr><td>People per hour</td><td>${fmtInt(o.existing_capacity_pph)}</td><td>${fmtInt(o.capacity_pph)}</td>${changeCell(d, pct === 0 ? "0" : (pct > 0 ? "+" : "−") + Math.abs(pct) + "%")}</tr></tbody>`;

  el.checks.innerHTML = view.checks
    .map(
      (c) =>
        `<li class="${c.ok ? "ok" : "bad"}">${c.ok ? ICON.ok : ICON.bad}<div><b>${esc(c.label)}<span class="sr-only">: ${c.ok ? "passes" : "fails"}</span></b><span>${esc(checkDetail(c))}</span></div></li>`,
    )
    .join("");

  el.revs.innerHTML =
    head(["Rev", "Description"]) +
    `<tbody><tr class="base${view.revisions.length ? "" : " now"}"><td>—</td><td>Existing street</td></tr>${view.revisions
      .map((r, i) => `<tr${i === view.revisions.length - 1 ? ' class="now"' : ""}><td>${esc(r.letter)}</td><td>${esc(r.label)}</td></tr>`)
      .join("")}</tbody>`;
  el.revs.scrollTop = el.revs.scrollHeight;

  $("tb-date").textContent = new Date().toISOString().slice(0, 10);
  $("tb-rev").textContent = view.revisions.at(-1)?.letter ?? "—";
}

function renderFit() {
  const box = $("fit");
  const d = view.delta_mm;
  box.className = "fit" + (d > 0 ? " bad" : "");
  box.textContent = d === 0 ? "Fits: the street is exactly full." : d < 0 ? `Fits: ${fmt(-d)} unassigned.` : `Over by ${fmt(d)}. Narrow or remove ${fmt(d)}.`;
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

function renderSamples() {
  el.sample.innerHTML = SAMPLES.map(
    (s, i) => `<option value="${i}"${i === view.sample ? " selected" : ""}>${esc(s.name)}, ${fmt(s.row_mm)}</option>`,
  ).join("");
}

function render() {
  renderHead();
  renderFit();
  renderDrawing();
  renderSchedule();
  renderNotes();
}

// ---- actions --------------------------------------------------------------

function select(uid) {
  sheet.select(uid || 0);
  view = JSON.parse(sheet.view());
  renderDrawing();
  for (const tr of el.sched.children) {
    const on = Number(tr.dataset.uid) === view.selected;
    tr.classList.toggle("sel", on);
    on ? tr.setAttribute("aria-selected", "true") : tr.removeAttribute("aria-selected");
  }
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

// ---- schedule input -------------------------------------------------------

el.sched.addEventListener("change", (e) => {
  const input = e.target.closest("input");
  if (!input) return;
  const uid = Number(input.closest("tr").dataset.uid);
  const v = parseFloat(input.value);
  if (Number.isFinite(v)) {
    const before = seg(uid).width_mm;
    sheet.set_width(uid, fromInput(v));
    refresh();
    if (seg(uid)?.width_mm !== before) announceEdit();
    else renderSchedule();
  } else {
    renderSchedule();
  }
});

el.sched.addEventListener("click", (e) => {
  const tr = e.target.closest("tr");
  if (!tr) return;
  const uid = Number(tr.dataset.uid);
  const btn = e.target.closest("button");
  if (btn) {
    select(uid);
    if (btn.dataset.act === "earlier") moveBy(uid, -1);
    else if (btn.dataset.act === "later") moveBy(uid, 1);
    else if (btn.dataset.act === "remove") removeSel(uid);
  } else if (uid !== view.selected) {
    select(uid);
  }
});

el.sched.addEventListener("focusin", (e) => {
  const tr = e.target.closest("tr");
  if (tr && Number(tr.dataset.uid) !== view.selected) select(Number(tr.dataset.uid));
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
    say("Reset to the existing street.");
  }
});
el.sample.addEventListener("change", () => {
  sheet.load_sample(Number(el.sample.value));
  wasOver = false;
  refresh();
  say(`${view.name} loaded.`);
});
for (const b of document.querySelectorAll(".unit")) {
  b.addEventListener("click", () => {
    units = b.dataset.unit;
    for (const o of document.querySelectorAll(".unit")) o.setAttribute("aria-pressed", String(o === b));
    renderSamples();
    renderPalette();
    render();
  });
}

new ResizeObserver(() => renderDrawing()).observe(el.scroll);

renderSamples();
renderPalette();
render();

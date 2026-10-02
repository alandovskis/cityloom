// Line-art elevation symbols and hatch patterns for the section drawing.
// Symbol origin is bottom-centre; y runs up as negative. Stroke width is
// constant on screen (see .sym in style.css), so a symbol can be scaled freely.

const at = (x, s, inner) =>
  `<g transform="translate(${x} 0) scale(${s})">${inner}</g>`;

const person = (x = 0, s = 1, v = 0) =>
  at(
    x,
    s,
    `<circle cx="0" cy="-58" r="6"/>` +
      `<path class="f-coat-${v}" d="M-7,-50 Q-8,-52 -5,-52 L5,-52 Q8,-52 7,-50 L9,-24 L-9,-24 Z"/>` +
      `<path class="o" d="M-4,-24 L-5,0 M4,-24 L5,0"/>`,
  );

const tree = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="o" d="M-3,0 L-3,-44 M3,0 L3,-44"/>` +
      `<path class="f-tree" d="M-22,-58 C-33,-62 -31,-86 -17,-88 C-15,-103 11,-107 17,-93 C33,-93 36,-71 24,-63 C21,-49 -13,-49 -22,-58 Z"/>` +
      `<path class="o" d="M0,-44 L0,-68 M0,-56 L-11,-68 M0,-60 L10,-76"/>`,
  );

const shrub = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="f-tree" d="M-15,0 C-18,-14 -7,-24 0,-19 C6,-26 18,-15 15,0 Z"/>` +
      `<path class="o" d="M0,-19 L0,-4 M-6,-14 L-4,-4 M7,-15 L5,-4"/>`,
  );

const cyclist = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<circle cx="-16" cy="-11" r="11"/><circle cx="16" cy="-11" r="11"/>` +
      `<path class="o" d="M-16,-11 L-4,-29 L11,-29 L16,-11 M-4,-29 L2,-11 L-16,-11 M11,-29 L9,-35 M6,-35 L13,-35"/>` +
      `<path class="o" d="M-4,-31 L4,-47 L11,-34 M1,-33 L4,-20 L2,-11"/>` +
      `<circle cx="6" cy="-53" r="5"/>`,
  );

const cone = (x = 0, s = 1) =>
  at(x, s, `<path class="f-van" d="M-9,0 L-4,-34 L4,-34 L9,0 Z"/><path d="M-6.5,-14 L6.5,-14 M-5.2,-24 L5.2,-24"/><path class="o" d="M-13,0 L13,0"/>`);

const car = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="f-car" d="M-46,-8 L-46,-19 C-46,-23 -42,-24 -38,-25 L-26,-27 L-16,-38 C-14,-40 -12,-40 -8,-40 L14,-40 C18,-40 20,-39 22,-37 L32,-27 L42,-25 C46,-24 48,-22 48,-18 L48,-8 Z"/>` +
      `<path d="M-14,-27 L-8,-36 L4,-36 L4,-27 Z"/><path d="M9,-27 L9,-36 L16,-36 L27,-27 Z"/>` +
      `<circle cx="-28" cy="-8" r="8"/><circle cx="28" cy="-8" r="8"/>`,
  );

const bus = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="f-bus" d="M-72,-7 L-72,-58 Q-72,-66 -64,-66 L64,-66 Q72,-66 72,-58 L72,-7 Z"/>` +
      [-60, -40, -20, 0, 20].map((wx) => `<rect x="${wx}" y="-58" width="16" height="20"/>`).join("") +
      `<path d="M42,-58 L58,-58 L58,-14 L42,-14 Z"/>` +
      `<path class="o" d="M-72,-44 L-64,-44"/>` +
      `<circle cx="-46" cy="-9" r="9"/><circle cx="44" cy="-9" r="9"/>`,
  );

const van = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="f-van" d="M-52,-8 L-52,-58 L10,-58 L10,-8 Z"/>` +
      `<path d="M10,-8 L10,-44 L30,-44 L44,-26 L54,-24 L54,-8 Z"/>` +
      `<path d="M15,-40 L28,-40 L38,-27 L15,-27 Z"/>` +
      `<circle cx="-30" cy="-8" r="8"/><circle cx="34" cy="-8" r="8"/>`,
  );

const postP = (x, s = 1) =>
  at(
    x,
    s,
    `<path class="o" d="M0,0 L0,-56"/><rect x="-7" y="-76" width="14" height="14" rx="1"/>` +
      `<path class="o" d="M-2.5,-64 L-2.5,-74 L1.5,-74 Q4,-74 4,-71.5 Q4,-69 1.5,-69 L-2.5,-69"/>`,
  );

// A bus shelter: posts, a roof in the bus lane's colour, a back panel and a
// bench, with a person waiting under it.
const shelter = (x = 0, s = 1) =>
  at(
    x,
    s,
    `<path class="o" d="M-34,0 L-34,-60 M34,0 L34,-60"/>` +
      `<path class="o" d="M-34,-56 L-34,-18 M34,-56 L34,-18 M-34,-18 L34,-18"/>` +
      `<path class="f-bus" d="M-42,-60 L42,-60 L42,-68 L-42,-68 Z"/>` +
      `<path class="o" d="M-24,-14 L-4,-14 M-20,-14 L-20,0 M-8,-14 L-8,0"/>` +
      person(18, 0.8, 1),
  );

// Each returns SVG for a segment `wm` metres wide drawn `wpx` pixels wide.
const SYMBOLS = {
  sidewalk: (wm, o) => {
    if (o.shelter) return shelter(0, 1);
    const n = wm < 2.2 ? 1 : wm < 4.2 ? 2 : 3;
    const gap = (wm * 55 * 0.3) / 1;
    const xs = n === 1 ? [0] : n === 2 ? [-gap * 0.55, gap * 0.55] : [-gap * 0.85, 0, gap * 0.85];
    const sizes = [1, 0.86, 0.94];
    return xs.map((x, i) => person(x, sizes[i], i % 2)).join("");
  },
  planting: (wm, o) => (o.material === "trees" ? tree(0, 1) : shrub(0, 1)),
  bike: () => cyclist(0, 1),
  travel: () => car(0, 1),
  bus: () => bus(0, 1),
  parking: (wm) => car(-8, 0.92) + postP(wm * 55 * 0.36, 0.9),
  median: (wm) => {
    const w = Math.max(wm * 55 - 10, 20) / 2;
    const kerb = `<rect class="f-kerb" x="${-w}" y="-10" width="${w * 2}" height="10"/>`;
    return kerb + (wm >= 2.4 ? tree(0, 0.9) : shrub(-w * 0.35, 0.8) + shrub(w * 0.35, 0.8)).replace(/translate\((-?[\d.]+) 0\)/g, (m, x) => `translate(${x} -10)`);
  },
  loading: () => van(0, 1),
  shoulder: () => cone(0, 1),
};

// Native widths, in drawing units at 55 px per metre, used to shrink a symbol
// to fit a narrow segment.
const NATIVE_W = {
  sidewalk: (wm, o) => (o.shelter ? 84 : wm < 2.2 ? 24 : wm < 4.2 ? 70 : 110),
  planting: (wm, o) => (o.material === "trees" ? 74 : 34),
  bike: () => 54,
  travel: () => 100,
  bus: () => 148,
  parking: () => 112,
  median: (wm) => (wm >= 2.4 ? 62 : 52),
  loading: () => 110,
  shoulder: () => 40,
};

// `opts` carries what a piece has beyond its kind: { shelter, material }.
export function symbol(kindId, cx, groundY, pxPerM, segPx, wm, f = 1, opts = {}) {
  const base = Math.min(pxPerM / 44, 1.2 * f);
  const fit = (segPx * 0.86) / NATIVE_W[kindId](wm, opts);
  const k = Math.min(base, fit);
  if (k < 0.3) return "";
  return `<g class="sym" transform="translate(${cx} ${groundY}) scale(${k})">${SYMBOLS[kindId](wm, opts)}</g>`;
}

// Hatch patterns. One distinct texture per segment type, so the drawing never
// depends on colour alone.
export const HATCH = {
  sidewalk: `<pattern id="h-sidewalk" width="8" height="8" patternUnits="userSpaceOnUse"><circle cx="2" cy="2" r="0.9"/><circle cx="6" cy="6" r="0.9"/></pattern>`,
  planting: `<pattern id="h-planting" width="9" height="8" patternUnits="userSpaceOnUse"><path d="M1.5,7 L2.5,3 M5,7 L4.5,2.5 M8,7 L8.8,3.5"/></pattern>`,
  bike: `<pattern id="h-bike" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0,0 L0,6"/></pattern>`,
  travel: `<pattern id="h-travel" width="14" height="12" patternUnits="userSpaceOnUse"><path d="M1,3 L6,3 M8,9 L13,9"/></pattern>`,
  bus: `<pattern id="h-bus" width="4" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)"><path d="M0,0 L0,4"/></pattern>`,
  parking: `<pattern id="h-parking" width="6" height="5" patternUnits="userSpaceOnUse"><path d="M0,2.5 L6,2.5"/></pattern>`,
  median: `<pattern id="h-median" width="7" height="7" patternUnits="userSpaceOnUse"><path d="M0,0 L7,0 M0,0 L0,7"/></pattern>`,
  shoulder: `<pattern id="h-shoulder" width="12" height="10" patternUnits="userSpaceOnUse"><path d="M0,5 L4,5 M6,5 L7,5 M9,5 L10,5"/></pattern>`,
  loading: `<pattern id="h-loading" width="10" height="8" patternUnits="userSpaceOnUse"><path d="M0,6 L5,1 L10,6"/></pattern>`,
};

// Surface material hatches, used for the pieces in the engineering view, where
// the hatch names the material and the label names the kind of piece.
export const MATERIAL_HATCH = {
  asphalt: `<pattern id="m-asphalt" width="5" height="5" patternUnits="userSpaceOnUse"><circle cx="1.2" cy="1.2" r="0.7"/><circle cx="3.7" cy="3.7" r="0.7"/></pattern>`,
  concrete: `<pattern id="m-concrete" width="12" height="10" patternUnits="userSpaceOnUse"><path d="M2,8 L4.5,3.5 L7,8 Z"/><circle cx="9.5" cy="3" r="0.8"/></pattern>`,
  permeable: `<pattern id="m-permeable" width="8" height="8" patternUnits="userSpaceOnUse"><rect x="1.5" y="1.5" width="5" height="5"/></pattern>`,
  brick: `<pattern id="m-brick" width="12" height="8" patternUnits="userSpaceOnUse"><path d="M0,0 H12 M0,4 H12 M3,0 V4 M9,4 V8"/></pattern>`,
  grass: `<pattern id="m-grass" width="9" height="8" patternUnits="userSpaceOnUse"><path d="M1.5,7 L2.5,3 M5,7 L4.5,2.5 M8,7 L8.8,3.5"/></pattern>`,
  trees: `<pattern id="m-trees" width="12" height="12" patternUnits="userSpaceOnUse"><circle cx="6" cy="6" r="4"/><circle cx="6" cy="6" r="0.8"/></pattern>`,
  planted: `<pattern id="m-planted" width="12" height="10" patternUnits="userSpaceOnUse"><circle cx="3.5" cy="3.5" r="2"/><circle cx="9" cy="7.5" r="1.4"/></pattern>`,
  gravel: `<pattern id="m-gravel" width="11" height="9" patternUnits="userSpaceOnUse"><circle cx="2" cy="2" r="1.1"/><circle cx="7.5" cy="3" r="0.6"/><circle cx="4.5" cy="7" r="0.9"/><circle cx="9.5" cy="7.5" r="0.5"/></pattern>`,
};

// Curb hatches.
export const CURB_HATCH = {
  granite: `<pattern id="c-granite" width="5" height="5" patternUnits="userSpaceOnUse"><path d="M0,0 L5,5 M5,0 L0,5"/></pattern>`,
  concrete: `<pattern id="c-concrete" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0,0 L0,5"/></pattern>`,
  asphalt: `<pattern id="c-asphalt" width="4" height="4" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="0.7"/><circle cx="3" cy="3" r="0.7"/></pattern>`,
  planted: `<pattern id="c-planted" width="6" height="6" patternUnits="userSpaceOnUse"><circle cx="1.8" cy="1.8" r="1.2"/><circle cx="4.6" cy="4.4" r="0.8"/></pattern>`,
  bikefriendly: `<pattern id="c-bikefriendly" width="5" height="5" patternUnits="userSpaceOnUse"><path d="M0,5 L5,0"/><circle cx="1.2" cy="1.2" r="0.6"/></pattern>`,
  island: `<pattern id="c-island" width="6" height="6" patternUnits="userSpaceOnUse"><path d="M0,3 H6 M3,0 V6"/></pattern>`,
  kassel: `<pattern id="c-kassel" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)"><path d="M0,0 L0,5"/></pattern>`,
};

// Starts the street page. The page itself is drawn and driven by the
// WebAssembly module (`mount_street_page`); this opens the street, keeps what
// it makes in the city, and sets up what every page shares.

import { placeParam } from "./city.js";

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
await start(wasm);

async function start({ hatches, mount_standards, mount_street_page, open_street, prepare_city }) {
  await prepare_city();

  const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());
  // The page edits a street of the city of its project (`?street=7&project=p1`) and writes each change back.
  // Without a street of that city the module sends it to the project's map, and without the project to the list.
  const placeId = placeParam("street");
  const sheet = open_street(placeId);
  if (!sheet) await new Promise(() => {});

  // The hatch patterns the section and the swatches are filled with.
  document.getElementById("defs").innerHTML =
    `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

  // What is waiting to be kept in the city is kept when the page is left.
  addEventListener("pagehide", () => sheet.flush());

  mount_street_page(sheet, placeId);
  mount_standards(sheet);

  sheet.mount_shell();
}

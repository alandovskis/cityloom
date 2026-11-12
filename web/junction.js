// Starts the junction page. The page itself is drawn and driven by the
// WebAssembly module (`mount_page`); this opens the junction, keeps what it
// makes in the city, and sets up what every page shares.

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

async function start({ hatches, enter_junction, mount_bare_shell, mount_page, open_junction, prepare_city }) {
  await prepare_city();

  const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());

  // The page edits a junction of the city of its project (`?junction=3&project=p1`), reading its streets from the
  // city, and writes each change back. Without a junction of that city the module sends it to the project's map,
  // and without the project to the list.
  const placeId = placeParam("junction");
  if (!enter_junction(placeId)) await new Promise(() => {});
  const editor = open_junction(placeId);
  if (!editor) {
    // The streets here have been changed so that the junction cannot be drawn: the module says so.
    mount_bare_shell(placeId);
    await new Promise(() => {});
  }

  // The hatch patterns the key's swatches are filled with.
  document.getElementById("defs").innerHTML =
    `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

  // What is waiting to be kept in the city is kept when the page is left.
  addEventListener("pagehide", () => editor.flush());

  mount_page(editor);

  editor.mount_shell();
}

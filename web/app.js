// Starts the street page. The page itself is drawn and driven by the
// WebAssembly module (`mount_street_page`); this opens the street, keeps what
// it makes in the city, and sets up what every page shares.

import init, { hatches, mount_street_page, open_street, prepare_city } from "./pkg/cityloom_editor.js";
import { placeParam } from "./city.js";

await init();
await prepare_city();

const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());
// The page edits a street of the city (`?street=7`) and writes each change back. It is reached from the
// map: without a street of the city it goes there.
const placeId = placeParam("street");
const sheet = placeId ? open_street(placeId) : undefined;
if (!sheet) {
  location.replace("map.html");
  await new Promise(() => {});
}

// The hatch patterns the section and the swatches are filled with.
document.getElementById("defs").innerHTML =
  `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

// What is waiting to be kept in the city is kept when the page is left.
addEventListener("pagehide", () => sheet.flush());

mount_street_page(sheet, placeId);

sheet.mount_shell();

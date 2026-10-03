// Starts the street page. The page itself is drawn and driven by the
// WebAssembly module (`mount_street_page`); this opens the street, keeps what
// it makes in the city, and sets up what every page shares.

import init, { Sheet, hatches, mount_street_page, open_street, street_ends } from "./pkg/cityloom_editor.js";
import { placeParam } from "./city.js";

await init();

const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());
// Opened from the map (`?street=7`) the page edits that street of the city and
// writes each change back; otherwise it is a sandbox on the sample streets.
const placeId = placeParam("street");
const held = placeId ? open_street(placeId) : undefined;
if (placeId && !held) {
  location.replace("map.html");
  await new Promise(() => {});
}
const sheet = held ?? new Sheet(0);

// The hatch patterns the section and the swatches are filled with.
document.getElementById("defs").innerHTML = `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

// What is waiting to be kept in the city is kept when the page is left.
addEventListener("pagehide", () => sheet.flush());

mount_street_page(sheet, held ? street_ends(placeId) : undefined);

sheet.mount_shell();

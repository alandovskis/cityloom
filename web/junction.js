// Starts the junction page. The page itself is drawn and driven by the
// WebAssembly module (`mount_page`); this opens the junction, keeps what it
// makes in the city, and sets up what every page shares.

import init, {
  hatches,
  junction_exists,
  mount_bare_shell,
  mount_page,
  open_junction,
  prepare_city,
} from "./pkg/cityloom_editor.js";
import { placeParam } from "./city.js";

await init();
await prepare_city();

const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());

// The page edits a junction of the city (`?junction=3`), reading its streets from the city, and writes each
// change back. It is reached from the map: without a junction of the city it goes there.
const placeId = placeParam("junction");
if (!placeId || !junction_exists(placeId)) {
  location.replace("map.html");
  await new Promise(() => {});
}
const plan = open_junction(placeId);
if (!plan) {
  // The streets here have been changed so that the junction cannot be drawn: the module says so.
  mount_bare_shell(placeId);
  await new Promise(() => {});
}

// The hatch patterns the plan and the swatches are filled with.
document.getElementById("defs").innerHTML =
  `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}
  <marker id="mv-head" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="mv-tip" d="M1 1 9 5 1 9"/></marker></defs>`;

// What is waiting to be kept in the city is kept when the page is left.
addEventListener("pagehide", () => plan.flush());

mount_page(plan);

plan.mount_shell();

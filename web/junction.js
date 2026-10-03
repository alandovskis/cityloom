// Starts the junction page. The page itself is drawn and driven by the
// WebAssembly module (`mount_page`); this opens the junction, keeps what it
// makes in the city, and sets up what every page shares.

import init, { Plan, hatches, junction_name, mount_page, open_junction } from "./pkg/cityloom_editor.js";
import { placeParam } from "./city.js";

await init();

const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());

// Opened from the map (`?junction=3`) the page edits that junction of the city,
// reading its streets from the city, and writes each change back; otherwise it
// is a sandbox on the sample junctions.
const placeId = placeParam("junction");
const placeName = placeId ? junction_name(placeId) : "";
const held = placeId ? open_junction(placeId) : undefined;
if (placeId && !placeName) {
  location.replace("map.html");
  await new Promise(() => {});
}
if (placeId && !held) {
  // The streets here have been changed so that the junction cannot be drawn.
  new Plan(0).mount_shell(); // only the settings menu and the sidebars have anything to do here
  document.getElementById("street-name").textContent = placeName;
  document.querySelector(".tools").hidden = true;
  document.querySelector(".sheet-body").innerHTML = `<div class="stuck"><h2 class="note-h">This junction cannot be drawn</h2><p>The streets that meet here have been changed so that they no longer make a junction. Give the streets their room back, or start the city over from the map.</p><p><a class="back" href="map.html">City map</a></p></div>`;
  await new Promise(() => {});
}
const plan = held ?? new Plan(0);

// The hatch patterns the plan and the swatches are filled with.
document.getElementById("defs").innerHTML = `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}
  <marker id="mv-head" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="mv-tip" d="M1 1 9 5 1 9"/></marker></defs>`;

// What is waiting to be kept in the city is kept when the page is left.
addEventListener("pagehide", () => plan.flush());

mount_page(plan);

plan.mount_shell();

// Starts the street page. The page itself is drawn and driven by the
// WebAssembly module (`mount_street_page`); this opens the street, keeps what
// it makes in the city, and sets up what every page shares.

import init, { Sheet, hatches, materials, mount_street_page, say } from "./pkg/cityloom_editor.js";
import { NOT_KEPT, keeper, openCity, placeParam, regionIndex, writeCity } from "./city.js";
import { initAccountMenu, initPanels, initRegion, initTheme, initUnits } from "./shell.js";

await init();

const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(hatches());
const MATERIALS = JSON.parse(materials());
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

// The hatch patterns the section and the swatches are filled with.
document.getElementById("defs").innerHTML = `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;

if (held) {
  const keepSoon = keeper(
    () => writeCity((c) => c.keep_street(placeId, sheet)),
    () => say(NOT_KEPT),
  );
  sheet.on_change(keepSoon);
}

mount_street_page(sheet, held ? city.street_ends(placeId) : undefined);

initUnits((u) => sheet.set_units(u));
initRegion({
  regions: MATERIALS.regions,
  apply: (i) => sheet.set_region(i),
  current: () => JSON.parse(sheet.view()).region,
  onChange: () => {},
  say,
});
initTheme(say);
initAccountMenu();
initPanels({ say });

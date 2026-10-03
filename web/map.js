// Starts the city map page. The page itself is drawn and driven by the
// WebAssembly module (`mount_map`); this sets up what every page shares.

import init, { hatches, materials, mount_map, say } from "./pkg/cityloom_editor.js";
import { initAccountMenu, initPanels, initRegion, initTheme, initUnits } from "./shell.js";

await init();

const { HATCH } = JSON.parse(hatches());
const MATERIALS = JSON.parse(materials());

// The hatch patterns the key's swatches are filled with.
document.getElementById("defs").innerHTML = `<defs>${Object.values(HATCH).join("")}</defs>`;

const page = mount_map();

initUnits((u) => page.set_units(u));
initRegion({
  regions: MATERIALS.regions,
  apply: (i) => page.set_region(i),
  current: () => page.region_id(),
  onChange: () => {},
  say,
});
initTheme(say);
initAccountMenu();
initPanels({ say, detailsWord: "places" });

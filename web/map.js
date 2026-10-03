// Starts the city map page. The page itself is drawn and driven by the
// WebAssembly module (`mount_map`); this sets up what every page shares, which the module binds (`mount_shell`).

import init, { hatches, mount_map, prepare_city } from "./pkg/cityloom_editor.js";
import "./city.js";

await init();
// The roads of the area being worked in, kept before the city is opened; if they cannot be
// got, the page says so and works on the sample city.
await prepare_city();

const { HATCH } = JSON.parse(hatches());

// The hatch patterns the key's swatches are filled with.
document.getElementById("defs").innerHTML = `<defs>${Object.values(HATCH).join("")}</defs>`;

mount_map().mount_shell();

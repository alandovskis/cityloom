// Starts the city map page. The page is drawn and driven by the WebAssembly module (`mount_map`); the map itself
// is MapLibre, made by basemap.js; this sets up what every page shares, which the module binds (`mount_shell`).

import init, { mount_map, prepare_city } from "./pkg/cityloom_editor.js";
import { createBasemap } from "./basemap.js";
import "./city.js";

await init();
// The roads of the area being worked in, kept before the city is opened; if they cannot be got, the page says
// so and has no map to show.
await prepare_city();

mount_map(await createBasemap("basemap")).mount_shell();

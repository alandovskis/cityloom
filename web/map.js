// Starts the city map page. The page is drawn and driven by the WebAssembly module (`mount_map`); the map itself
// is MapLibre, made by mapcanvas.js; this sets up what every page shares, which the module binds (`mount_shell`).

import "./city.js";
import "./report.js";

let wasm, canvas;
try {
  [wasm, canvas] = await Promise.all([import("./pkg/cityloom_editor.js"), import("./mapcanvas.js")]);
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
// The roads of the area being worked in, kept before the city is opened; if they cannot be got, the page says
// so and has no map to show.
await wasm.prepare_city();

const page = wasm.mount_map(await canvas.createMapCanvas("map-canvas"));
page.mount_shell();
// The shell makes the Share menu's Download report item when it mounts, so it is bound after.
page.bind_report();

// Starts the city map page. The page is drawn and driven by the WebAssembly module (`mount_map`); the map itself
// is MapLibre, made by basemap.js; this sets up what every page shares, which the module binds (`mount_shell`).

import "./city.js";

let wasm, basemap;
try {
  [wasm, basemap] = await Promise.all([import("./pkg/cityloom_editor.js"), import("./basemap.js")]);
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
// The roads of the area being worked in, kept before the city is opened; if they cannot be got, the page says
// so and has no map to show.
await wasm.prepare_city();

wasm.mount_map(await basemap.createBasemap("basemap")).mount_shell();

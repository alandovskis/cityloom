// Starts the home page. The page itself is drawn and driven by the WebAssembly
// module (`mount_projects`: the list of projects, and the About section below it);
// this loads it and binds what every page shares.

import "./city.js";

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
// The list and the search do not need the city: they are there at once, and the city behind them is drawn when its
// roads are got.
const page = wasm.mount_projects();
page.mount_shell();
await wasm.prepare_city();
page.mount_city();

// Starts the home page. The page itself is drawn and driven by the WebAssembly
// module (`mount_home`); this loads it and binds what every page shares.

import "./city.js";

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
// The search does not need the city: it is there at once, and the city behind it is drawn when its roads are got.
const page = wasm.mount_home();
page.mount_shell();
await wasm.prepare_city();
page.mount_city();

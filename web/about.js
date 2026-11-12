// Starts the About page: what CityLoom is for, with the city of the project opened last drawn behind it. The page
// is drawn and driven by the WebAssembly module (`mount_about`); this loads it and binds what every page shares.

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
// The words are there at once; the city behind them is drawn when its roads are got.
const page = wasm.mount_about();
page.mount_shell();
await wasm.prepare_city();
page.mount_city();

// Starts the account page. The page is drawn and driven by the WebAssembly module (`mount_account`); this loads it
// and binds what every page shares (`mount_shell`).

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
wasm.mount_account().mount_shell();

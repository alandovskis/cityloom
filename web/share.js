// Starts the share page: a project's share links (`share.html?project=<id>`) or a link being opened
// (`share.html#<token>`). The page is drawn and driven by the WebAssembly module (`mount_share`); this loads it and
// binds what every page shares (`mount_shell`).

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}
wasm.mount_share()?.mount_shell();

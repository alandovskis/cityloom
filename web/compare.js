// Starts the comparison page. The page is drawn and driven by the WebAssembly module (`mount_compare`); this gets the
// city's roads, defines the hatches the drawings are filled with, and binds what every page shares (`mount_shell`).

let wasm;
try {
  wasm = await import("./pkg/cityloom_editor.js");
  await wasm.default();
} catch (error) {
  globalThis.cityloomBootFailed?.();
  throw error;
}

await wasm.prepare_city();
const { HATCH, MATERIAL_HATCH, CURB_HATCH } = JSON.parse(wasm.hatches());
// The hatch patterns the plan and the 2.5D view are filled with.
document.getElementById("defs").innerHTML =
  `<defs>${[HATCH, MATERIAL_HATCH, CURB_HATCH].flatMap((h) => Object.values(h)).join("")}</defs>`;
wasm.mount_compare()?.mount_shell();

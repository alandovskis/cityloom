// Starts the account page. The page is drawn and driven by the WebAssembly module (`mount_account`); this loads it
// and binds what every page shares (`mount_shell`).

import { failed } from "./boot.js";

try {
  const { default: init, mount_account } = await import("./pkg/cityloom_editor.js");
  await init();
  mount_account().mount_shell();
} catch (error) {
  failed();
  throw error;
}

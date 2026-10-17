// Starts the home page. The page itself is drawn and driven by the WebAssembly
// module (`mount_home`); this loads it and binds what every page shares.

import { failed } from "./boot.js";
import "./city.js";

try {
  const { default: init, mount_home, prepare_city } = await import("./pkg/cityloom_editor.js");
  await init();
  await prepare_city();
  mount_home().mount_shell();
} catch (error) {
  failed();
  throw error;
}

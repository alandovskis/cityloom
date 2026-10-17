// Starts the home page. The page itself is drawn and driven by the WebAssembly
// module (`mount_home`); this loads it and binds what every page shares.

import { failed } from "./boot.js";
import "./city.js";

try {
  const { default: init, mount_home, prepare_city } = await import("./pkg/cityloom_editor.js");
  await init();
  // The search does not need the city: it is there at once, and the city behind it is drawn when its roads are got.
  const page = mount_home();
  page.mount_shell();
  await prepare_city();
  page.mount_city();
} catch (error) {
  failed();
  throw error;
}

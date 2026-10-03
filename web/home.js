// Starts the home page. The page itself is drawn and driven by the WebAssembly
// module (`mount_home`); this loads it and binds what every page shares.

import init, { mount_home, prepare_city } from "./pkg/cityloom_editor.js";
import "./city.js";

await init();
await prepare_city();

mount_home().mount_shell();

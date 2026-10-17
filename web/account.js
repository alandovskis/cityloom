// Starts the account page. The page is drawn and driven by the WebAssembly module (`mount_account`); this loads it
// and binds what every page shares (`mount_shell`).

import init, { mount_account } from "./pkg/cityloom_editor.js";

await init();

mount_account().mount_shell();

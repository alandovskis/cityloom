# Account slice (`src/account/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- Account page: `web/account.html`, `web/account.js`, `mount_account` (first `project::migrate::move_legacy_work`, before the view-model, the archive or the shell's sync are built: the page syncs as soon as it knows who is signed in). `vm.rs`: linked sign-ins, link/unlink, storage choice (shown and focused first until made; server choice merges and reports places sent/received, see `sync/`; turning off is confirmed, deletes the server copy, keeps the browser's; lists what is stored and what is encrypted), in-page deletion confirmation. `view.rs`, `text.rs`, `i18n/{en,fr}.ftl` (`account-*`).
- Header Sign in button/menu is the shell's `session` (`shell/session.rs`, `session_view.rs`, `session-*`, slot `#session-slot` on all app pages and Account). Shell "account" (`#account-btn`, `.account`) is the SETTINGS menu.
- `Ports.account` is the `Account` port (`me`, `providers`, `set_storage`, `unlink`, `delete`, `sign_out`; `AccountError` keeps the status; `FakeAccount`, `test_ports_full()`). `Page` has `path()` and `take_fragment()` (callback outcome read once, removed from the address).

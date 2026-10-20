import { defineConfig, devices } from "@playwright/test";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

// The pages are static files; the WebAssembly module they load is built first
// with `./scripts/build.sh` (it writes web/pkg).
// E2E_PORT moves the server off 8137 when another one already holds that port.
const port = process.env.E2E_PORT ?? 8137;

const server = fileURLToPath(new URL("../target/debug/cityloom_server", import.meta.url));
if (!existsSync(server)) {
  throw new Error(
    `${server} is missing: run 'just e2e' (it builds the server first) or 'cargo build -p cityloom_server'`,
  );
}

// `just e2e-archive` (E2E_ARCHIVE=1): the server unpacks the archive that script packed and serves the road data from it.
const fromArchive = !!process.env.E2E_ARCHIVE;
const archiveRoot = fileURLToPath(new URL("../target/e2e-archive/", import.meta.url));

const b64 = (byte: number, length: number) => Buffer.alloc(length, byte).toString("base64");

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : undefined,
  // CI keeps an HTML report (with traces) for the run to upload when a test fails.
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    locale: "en-CA",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } }],
  webServer: {
    command: server,
    env: {
      WEB_ROOT: "../web",
      E2E_PORT: String(port),
      // The keys are fixed test values, not secrets. DATABASE_URL is not set here: the server takes it from the
      // caller's environment, and the account side stays off without it (only the account specs fail).
      BASE_URL: `http://127.0.0.1:${port}`,
      COOKIE_KEY: b64(7, 64),
      DATA_KEYS: `1:${b64(1, 32)}`,
      DATA_KEY_CURRENT: "1",
      LOOKUP_KEY: b64(3, 32),
      FAKE_PROVIDER: "1",
      ...(fromArchive ? { CITYLOOM_AREAS: `${archiveRoot}areas`, DATA_ROOT: `${archiveRoot}data` } : {}),
      // Many parallel specs sign in from one address: the production allowance (10 a minute) would answer 429.
      AUTH_RATE_BURST: "100000",
      AUTH_RATE_PERIOD_SECS: "1",
      // The sync specs poll /api/work from many parallel tests of one address: the default burst would answer 429.
      WORK_RATE_BURST: "100000",
      WORK_RATE_PERIOD_SECS: "1",
    },
    url: `http://127.0.0.1:${port}/map.html`,
    // A server already running without the archive would give false results.
    reuseExistingServer: !process.env.CI && !fromArchive,
  },
});

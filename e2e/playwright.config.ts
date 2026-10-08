import { defineConfig, devices } from "@playwright/test";

// The pages are static files; the WebAssembly module they load is built first
// with `./scripts/build.sh` (it writes web/pkg).
// E2E_PORT moves the server off 8137 when another one already holds that port.
const port = process.env.E2E_PORT ?? 8137;

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
    command: "node serve.mjs",
    url: `http://127.0.0.1:${port}/map.html`,
    reuseExistingServer: !process.env.CI,
  },
});

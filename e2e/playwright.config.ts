import { defineConfig, devices } from "@playwright/test";

// The pages are static files; the WebAssembly module they load is built first
// with `./scripts/build.sh` (it writes web/pkg).
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : undefined,
  // CI keeps an HTML report (with traces) for the run to upload when a test fails.
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: "http://127.0.0.1:8137",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "node serve.mjs",
    url: "http://127.0.0.1:8137/map.html",
    reuseExistingServer: !process.env.CI,
  },
});

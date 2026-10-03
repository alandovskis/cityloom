import { defineConfig, devices } from "@playwright/test";

// The pages are static files; the WebAssembly module they load is built first
// with `./scripts/build.sh` (it writes web/pkg).
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:8137",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "node serve.mjs",
    url: "http://127.0.0.1:8137/map.html",
    reuseExistingServer: true,
  },
});

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test as base, type BrowserContext, type Page } from "@playwright/test";

/** Every test fails if its page throws or logs a console error. Each test gets a
 *  fresh browser context, so the page starts with empty storage.
 *
 *  The pages open the area of the world last chosen, the default one before any is.
 *  Most tests are written against the sample city, which needs no network, so a
 *  test starts on that unless it says `test.use({ area: "world" })`. */
export const test = base.extend<{ errors: string[]; area: "sample" | "world" }>({
  area: ["sample", { option: true }],
  context: async ({ context, area }, use) => {
    if (area === "sample") {
      await context.addInitScript(() => {
        try {
          if (!localStorage.getItem("cityloom-area")) localStorage.setItem("cityloom-area", "sample");
        } catch {
          // storage blocked: the page then has the default area
        }
      });
    }
    await serveBasemap(context);
    await use(context);
  },
  errors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(e.message));
      page.on("console", (m) => {
        if (m.type() === "error") errors.push(m.text());
      });
      await use(errors);
      expect(errors).toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };

/** What a screen reader was last told. */
export const live = (page: Page) => page.locator("#live");

/** The sample city, as the map lists it: the junction and street the tests open. */
export const JUNCTION = 3; // "Junction 1"
export const STREET = 1; // "Sample Avenue 2", between the edge of the map and Junction 4

export const PLATEAU_PBF = fileURLToPath(new URL("../fixtures/plateau.osm.pbf", import.meta.url));
export const PLATEAU_TILES = fileURLToPath(new URL("../fixtures/plateau.pmtiles", import.meta.url));

/** Serves the basemap's tiles as a static host with range support does: PMTiles reads them in pieces. */
export async function serveBasemap(context: BrowserContext, file = PLATEAU_TILES) {
  const body = readFileSync(file);
  await context.route("**/data/basemap/montreal.pmtiles", async (route) => {
    const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()["range"] ?? "");
    if (!range) return route.fulfill({ body, headers: { "accept-ranges": "bytes" } });
    const start = Number(range[1]);
    const end = Math.min(range[2] ? Number(range[2]) : body.length - 1, body.length - 1);
    await route.fulfill({
      status: 206,
      body: body.subarray(start, end + 1),
      headers: { "content-range": `bytes ${start}-${end}/${body.length}`, "accept-ranges": "bytes" },
    });
  });
}

/** What the Plateau needs where there is no network: a metro tile of it, and no Overpass. */
export async function serveWorld(page: Page) {
  await page.route("**/data/metro/index.json", (route) =>
    route.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await page.route("**/data/metro/0_0.osm.pbf", (route) => route.fulfill({ body: readFileSync(PLATEAU_PBF) }));
  await page.route("https://overpass-api.de/**", (route) => route.abort());
}

/** Waits until the places are on the map and its tiles are drawn. */
export async function mapReady(page: Page) {
  await page.waitForFunction(() => {
    const m = (window as any).cityloomMap;
    return !!m && !!m.getSource("places") && m.areTilesLoaded() && m.querySourceFeatures("places").length > 0;
  });
}

/** Waits until the map has stopped moving: a fit, a zoom and a pan each ease for a moment. */
export async function mapStill(page: Page) {
  await page.waitForFunction(() => !(window as any).cityloomMap.isMoving());
}

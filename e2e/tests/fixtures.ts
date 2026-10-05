import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test as base, type BrowserContext, type Page } from "@playwright/test";

/** Every test fails if its page throws or logs a console error. Each test gets a
 *  fresh browser context, so the page starts with empty storage.
 *
 *  The pages open the area of the world last chosen, the default one before any is: the Plateau
 *  Mont-Royal. There is no network in the tests, so its roads come from a metro tile served from
 *  `fixtures/plateau.osm.pbf` and its basemap from `fixtures/plateau.pmtiles`. A test that wants
 *  something else asks for it with its own `page.route`, which wins over these. */
export const test = base.extend<{ errors: string[]; world: void }>({
  context: async ({ context }, use) => {
    await serveBasemap(context);
    await use(context);
  },
  world: [
    async ({ page }, use) => {
      await serveWorld(page);
      await use();
    },
    { auto: true },
  ],
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

export const PLATEAU_PBF = fileURLToPath(new URL("../fixtures/plateau.osm.pbf", import.meta.url));
export const PLATEAU_TILES = fileURLToPath(new URL("../fixtures/plateau.pmtiles", import.meta.url));

/** Serves the basemap's tiles as a static host with range support does: PMTiles reads them in pieces.
 *  `tiles` is a file to read them from, or the bytes themselves. */
export async function serveBasemap(context: BrowserContext, tiles: string | Buffer = PLATEAU_TILES) {
  const body = typeof tiles === "string" ? readFileSync(tiles) : tiles;
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

/** Opens a junction of the city the way a person does: from the map's list of places. Says its address. */
export async function openJunction(page: Page, n = 0) {
  return openPlace(page, "intersection.html", n);
}

/** Opens a street of the city from the map's list of places. Says its address. */
export async function openStreet(page: Page, n = 0) {
  return openPlace(page, "street.html", n);
}

async function openPlace(page: Page, editor: string, n: number) {
  await page.goto("/map.html");
  const row = page.locator(`#places-panel a.place-row[href^='${editor}']`).nth(n);
  const href = (await row.getAttribute("href"))!;
  await row.click();
  await page.waitForURL(`**/${href}`);
  return href;
}

/** Waits until the places are on the map and its tiles are drawn. */
export async function mapReady(page: Page) {
  await page.waitForFunction(() => {
    const m = (window as any).cityloomMap;
    return !!m && !!m.getSource("places") && m.areTilesLoaded() && m.querySourceFeatures("places").length > 0;
  });
}

/** Takes out of `errors` the ones a test expects (a request it refused, say), each of which must match one of
 *  `expected`: any other error is left for the fixture to fail the test on. Each expected error must have
 *  happened, so a test whose failure went away does not pass for nothing. */
export function forgive(errors: string[], ...expected: RegExp[]) {
  for (const p of expected)
    expect(
      errors.some((e) => p.test(e)),
      `an error matching ${p}`,
    ).toBe(true);
  const others = errors.filter((e) => !expected.some((p) => p.test(e)));
  errors.splice(0, errors.length, ...others);
}

/** Waits until the map has stopped moving: a fit, a zoom and a pan each ease for a moment. */
export async function mapStill(page: Page) {
  await page.waitForFunction(() => !(window as any).cityloomMap.isMoving());
}

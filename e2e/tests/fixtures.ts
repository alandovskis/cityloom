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
    // Every spec starts in English whatever the machine's language is; a spec that switches to
    // French and reloads keeps it, because the choice is only stored when there is none.
    await context.addInitScript(() => {
      try {
        if (localStorage.getItem("cityloom-lang") === null) localStorage.setItem("cityloom-lang", "en");
      } catch {
        // storage is blocked: the page falls back to its own default
      }
    });
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

/** The places of the city on the map, as the addresses of their editors: the junctions, then the streets, each
 *  in the order of its number. */
export async function placesOnMap(page: Page) {
  await mapReady(page);
  return page.evaluate(() => {
    const hots = new Set<string>();
    for (const f of (window as any).cityloomMap.querySourceFeatures("places")) hots.add(f.properties.hot);
    const hrefs = (kind: string, page: string, param: string) =>
      [...hots]
        .filter((h) => h.startsWith(`${kind}-`))
        .map((h) => Number(h.slice(2)))
        .sort((a, b) => a - b)
        .map((uid) => `${page}?${param}=${uid}`);
    return { junctions: hrefs("j", "intersection.html", "junction"), streets: hrefs("s", "street.html", "street") };
  });
}

/** Opens a junction of the city, by the address the map gives it. Says its address. */
export async function openJunction(page: Page, n = 0) {
  await page.goto("/map.html");
  const href = (await placesOnMap(page)).junctions[n];
  await page.goto("/" + href);
  return href;
}

/** Opens a street of the city, by the address the map gives it. Says its address. */
export async function openStreet(page: Page, n = 0) {
  await page.goto("/map.html");
  const href = (await placesOnMap(page)).streets[n];
  await page.goto("/" + href);
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

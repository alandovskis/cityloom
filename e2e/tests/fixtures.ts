import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test as base, type Browser, type BrowserContext, type Page } from "@playwright/test";

/** `just e2e-archive`: the server serves the Plateau's tile and basemap from an unpacked `.cityloom` archive, so the
 *  fixtures do not route them (a route would answer before the server and the run would prove nothing). */
const FROM_ARCHIVE = !!process.env.E2E_ARCHIVE;

/** Every test fails if its page throws or logs a console error. Each test gets a
 *  fresh browser context, so the page starts with empty storage.
 *
 *  The pages open the area of the world last chosen, the default one before any is: the Plateau
 *  Mont-Royal. There is no network in the tests, so its roads come from a metro tile served from
 *  `fixtures/plateau.osm.pbf` and its basemap from `fixtures/plateau.pmtiles`. A test that wants
 *  something else asks for it with its own `page.route`, which wins over these. With `E2E_ARCHIVE=1` the default
 *  tile, its index and the default basemap are not routed: the server answers them from the archive. */
export const test = base.extend<{ errors: string[]; world: void }>({
  context: async ({ context }, use) => {
    await prepareContext(context);
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
        if (m.type() !== "error") return;
        // Every page asks who is signed in (and for the providers when nobody is). A signed-out visitor, a server
        // with no account side and an older server answer those with 401, 503 or 404, which the browser logs, and a sync that cannot reach `/api/work` (offline, signed out, storage off) logs the
        // same; `sync.spec.ts` checks what the sync does about it.
        if (/^\/api\/(me|providers|work)$/.test(new URL(m.location().url || "http://x/").pathname)) return;
        // A spec that makes the account side fail on purpose (a 401 for an ended session, a 500) says so with an
        // annotation, and only the changes under /api/me are let through for it.
        const path = new URL(m.location().url || "http://x/").pathname;
        if (path.startsWith("/api/me/") && test.info().annotations.some((n) => n.type === "failing-account-api"))
          return;
        errors.push(m.text());
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
export const TURBO_PBF = fileURLToPath(new URL("../fixtures/turbo.osm.pbf", import.meta.url));
export const PLATEAU_TILES = fileURLToPath(new URL("../fixtures/plateau.pmtiles", import.meta.url));

/** Serves the basemap's tiles as a static host with range support does: PMTiles reads them in pieces.
 *  `tiles` is a file to read them from, or the bytes themselves. */
export async function serveBasemap(context: BrowserContext, tiles: string | Buffer = PLATEAU_TILES) {
  if (FROM_ARCHIVE && tiles === PLATEAU_TILES) return; // the server serves the default basemap itself
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
  if (!FROM_ARCHIVE) {
    await page.route("**/data/metro/index.json", (route) =>
      route.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
    );
    await page.route("**/data/metro/0_0.osm.pbf", (route) => route.fulfill({ body: readFileSync(PLATEAU_PBF) }));
  }
  await page.route("https://overpass-api.de/**", (route) => route.abort());
}

/** What every test's own context gets: English unless a spec switches (a spec that switches to French and reloads
 *  keeps it, because the choice is only stored when there is none), and the basemap's tiles, so there is no network. */
export async function prepareContext(context: BrowserContext) {
  await context.addInitScript(() => {
    try {
      if (localStorage.getItem("cityloom-lang") === null) localStorage.setItem("cityloom-lang", "en");
    } catch {
      // storage is blocked: the page falls back to its own default
    }
  });
  await serveBasemap(context);
}

/** A second browser with its own cookies and storage (another person, or the same one on another device), set up
 *  as each test's own is: `prepareContext`, the options `playwright.config.ts` gives every test, and a page that
 *  serves the Plateau's tile. The `errors` fixture watches only the test's own `page`; check this one's if it matters. */
export async function anotherBrowser(browser: Browser, baseURL: string) {
  const context = await browser.newContext({ baseURL, locale: "en-CA", viewport: { width: 1440, height: 900 } });
  await prepareContext(context);
  const page = await context.newPage();
  await serveWorld(page);
  return { context, page };
}

/** The places of the city on the map, as the addresses of their editors: the junctions, then the streets, each
 *  in the order of its number. */
export async function placesOnMap(page: Page) {
  await mapReady(page);
  // The junctions and the streets are drawn from their own tiles: wait until both are there.
  await page.waitForFunction(
    () => {
      const hots = (window as any).cityloomMap.querySourceFeatures("places").map((f: any) => String(f.properties.hot));
      return hots.some((h: string) => h.startsWith("j-")) && hots.some((h: string) => h.startsWith("s-"));
    },
    undefined,
    { timeout: 15_000 },
  );
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

/** The index of the first piece of the section that is wide enough to be pressed: a boundary's handle covers a
 *  narrower one (an imported curb buffer is 150 mm), and a press on it would land on the handle. A handle's hit area
 *  is 18 px wide, centred on the boundary, so 9 px of each edge of a piece is under one; 40 px leaves a clear middle
 *  of at least 22 px to press. */
export const PRESSABLE_PX = 40;

export async function pressablePiece(page: Page) {
  const segs = page.locator('#drawing [data-role="seg"]');
  // `openStreet` returns before the drawing is mounted, and `evaluateAll` does not wait for it.
  await expect(segs.first()).toBeVisible();
  const boxes = await segs.evaluateAll((els) => els.map((e) => e.getBoundingClientRect().width));
  const i = boxes.findIndex((w) => w >= PRESSABLE_PX);
  expect(i, "a piece wide enough to press").toBeGreaterThanOrEqual(0);
  return i;
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

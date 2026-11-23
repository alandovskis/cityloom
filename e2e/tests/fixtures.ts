import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test as base, type Browser, type BrowserContext, type Locator, type Page } from "@playwright/test";

/** The version the edits document is written as (`IMPORT_STAMP_VERSION` in `city/edits.rs`), which every street record's
 *  import stamp makes the document hold. */
export const EDITS_VERSION = 17;

/** `just e2e-archive`: the server serves the Plateau's tile from an unpacked `.citymodel` archive, so the
 *  fixtures do not route them (a route would answer before the server and the run would prove nothing). */
const FROM_ARCHIVE = !!process.env.E2E_ARCHIVE;

/** Every test fails if its page throws or logs a console error. Each test gets a
 *  fresh browser context, so the page starts with empty storage.
 *
 *  The pages open the area of the world last chosen, the default one before any is: the Plateau
 *  Mont-Royal. There is no network in the tests, so its roads come from a metro tile served from
 *  `fixtures/generated/plateau.json` (`just e2e-fixtures` makes it). A test that wants
 *  something else asks for it with its own `page.route`, which wins over these. With `E2E_ARCHIVE=1` the default
 *  tile and its index are not routed: the server answers them from the archive. */
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

/** The OSM fixtures of `e2e/fixtures`, each imported into one network tile (`just e2e-fixtures`, which the browser
 *  test recipes run first). */
const generated = (name: string) => fileURLToPath(new URL(`../fixtures/generated/${name}.json`, import.meta.url));
export const PLATEAU_TILE = generated("plateau");
export const TURBO_TILE = generated("turbo");
export const OPEN_ENDS_TILE = generated("open-ends");

/** The answer to a request for a metro tile: the network tile in `file`. */
export const tileFrom = (file: string) => ({ contentType: "application/json", body: readFileSync(file) });

/** What the Plateau needs where there is no network: a metro tile of it, with a checksum, which the area is kept
 *  with and the importer's version after it, so an edit made on it is stamped with that version (without one, an
 *  edit is unstamped and so stale). */
export async function serveWorld(page: Page) {
  if (FROM_ARCHIVE) return;
  await page.route("**/data/metro/index.json", (route) =>
    route.fulfill({
      json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"], checksums: { "0_0": "e2e" } },
    }),
  );
  await page.route("**/data/metro/0_0.json", (route) => route.fulfill(tileFrom(PLATEAU_TILE)));
}

/** The project every test works in: the first project, as moving the work from before projects makes it (id
 *  `first`, the default area, created at the epoch). The map and the editors open inside a project; with none named
 *  in the address they open the one opened last, which `prepareContext` makes this one. */
export const FIRST_PROJECT = {
  id: "first",
  name: "First project",
  area: "area=45.5261,-73.5978&name=Plateau%20Mont-Royal%2C%20Montr%C3%A9al",
  created_at: "1970-01-01T00:00:00.000Z",
};

/* A page of the first project keeps its edits and area rules under `cityloom-edits@first` and
 * `cityloom-restrictions@first` (`<key>@<project>`): the specs read and seed those. Its area is the one its record
 * names, which `setFirstArea` changes. */

/** Makes the first project's record name the area `query` (an `Area::to_query`): its pages open that area. Nothing
 *  in the app changes a project's area; only the specs that follow an edit into another area do. */
export async function setFirstArea(page: Page, query: string) {
  await page.evaluate((query) => {
    const doc = JSON.parse(localStorage.getItem("cityloom-projects")!);
    doc.projects.find((p: { id: string }) => p.id === "first").area = query;
    localStorage.setItem("cityloom-projects", JSON.stringify(doc));
  }, query);
}

/** What every test's own context gets: English unless a spec switches (a spec that switches to French and reloads
 *  keeps it, because the choice is only stored when there is none), so there is no network; and the first project,
 *  opened last, unless the browser already has a list of projects. */
export async function prepareContext(context: BrowserContext) {
  await context.addInitScript((first) => {
    try {
      if (localStorage.getItem("cityloom-lang") === null) localStorage.setItem("cityloom-lang", "en");
      if (localStorage.getItem("cityloom-projects") === null) {
        localStorage.setItem("cityloom-projects", JSON.stringify({ version: 1, projects: [first] }));
        localStorage.setItem("cityloom-project", first.id);
      }
    } catch {
      // storage is blocked: the page falls back to its own default
    }
  }, FIRST_PROJECT);
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

/** The item named exactly `name` in the open Add a piece menu, in the topic shown or in the search's results. */
export function menuItem(page: Page, name: string): Locator {
  const exact = new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`);
  return page.locator("#add-menu .add-item", { has: page.locator("b", { hasText: exact }) });
}

/** Opens the street page's Add a piece menu, searches for `name` and chooses the item named exactly that. */
export async function addFromMenu(page: Page, name: string): Promise<void> {
  await page.locator("#add-btn").click();
  await page.locator("#add-search").fill(name);
  await menuItem(page, name).click();
}

/** The length of the street in metres, from the plan's drawing. */
export async function streetLength(page: Page) {
  const mm = await page.locator("#plan svg").getAttribute("data-length-mm");
  return Number(mm) / 1000;
}

/** Shows the Street level of the street page's details panel (the speed limit, the standards): the first step of the
 *  panel's path, unless the street is already the level shown. */
export async function showStreetLevel(page: Page) {
  const step = page.locator('.panel-path .path-step[data-level="street"]');
  if ((await step.count()) > 0) await step.click();
  await expect(page.locator("#p-street")).toBeVisible();
}

/** Presses `key` `times` times. */
async function press(page: Page, key: string, times: number) {
  for (let k = 0; k < times; k++) await page.keyboard.press(key);
}

/** Moves the plan's point to `atM` metres from its left end, from the keyboard (whole metres). */
async function pointTo(page: Page, atM: number) {
  await page.locator("#plan-point").focus();
  await page.keyboard.press("Home");
  await press(page, "PageDown", Math.floor(atM / 10));
  await press(page, "ArrowRight", atM % 10);
  await expect(page.locator("#plan-point")).toHaveAttribute("aria-valuenow", String(atM));
}

/** Picks the stretch from `fromM` to `toM` metres from the plan's left end with the keyboard: the point to `fromM`,
 *  then Shift and the arrows to `toM`. */
export async function pickStretch(page: Page, fromM: number, toM: number) {
  await pointTo(page, fromM);
  const by = toM - fromM;
  await press(page, "Shift+PageDown", Math.floor(by / 10));
  await press(page, "Shift+ArrowRight", by % 10);
}

/** Picks the stretch from `fromM` metres to the plan's right end with the keyboard (Shift+End). */
export async function pickToEnd(page: Page, fromM: number) {
  await pointTo(page, fromM);
  await page.keyboard.press("Shift+End");
}

/** Picks the run at `atM` metres from the plan's left end with the keyboard (Enter at the point). */
export async function pickRunAt(page: Page, atM: number) {
  await pointTo(page, atM);
  await page.keyboard.press("Enter");
}

/** Picks the whole street with the keyboard (Escape on the plan's point). */
export async function pickWhole(page: Page) {
  await page.locator("#plan-point").focus();
  await page.keyboard.press("Escape");
  await expect(page.locator("#plan-picked")).toHaveText(/^The whole street/);
}

/** The boundaries between the runs of the street, as the plan draws them. */
export const planBoundaries = (page: Page) => page.locator("#plan .plan-boundary");

/** Makes `count` stretches of whole metres, evenly, by picking each and giving it a speed limit its neighbours do not
 *  have (20 and 30 km/h in turn), so they stay apart. The street needs room for them (5 m a stretch). The whole street
 *  is picked again at the end. */
export async function makeStretches(page: Page, count: number) {
  const length = await streetLength(page);
  const step = Math.floor(length / count);
  expect(step, `a street of ${length} m is too short for ${count} stretches`).toBeGreaterThanOrEqual(5);
  for (let k = 0; k < count; k++) {
    if (k < count - 1) await pickStretch(page, k * step, (k + 1) * step);
    else await pickToEnd(page, k * step);
    await showStreetLevel(page);
    await page.locator("#speed-limit").selectOption(k % 2 === 0 ? "20" : "30");
  }
  await expect(planBoundaries(page)).toHaveCount(count - 1);
  await pickWhole(page);
}

/** Chooses a cell of a row along the street (curbside, stops, raised elements): the rows are folded under the plan
 *  until the keyboard is in one, so its listbox takes the focus first, then the cell is pressed. */
export async function chooseCell(page: Page, cell: Locator) {
  await cell.locator("xpath=ancestor::*[@role='listbox'][1]").focus();
  await cell.click();
}

/** The thing a row's cell `id` stands for, as the plan draws it. */
export const planThing = (page: Page, id: string) => page.locator(`#plan [data-thing="${id}"]`);

/** Opens the first street of the city with a real end to measure from (it has a raised-tables strip, which needs an
 *  end to measure from) that is at least `minM` metres long. */
export async function openAnchoredStreet(page: Page, minM: number) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#plan svg")).toBeVisible();
    if ((await page.locator("#calming-strip").count()) === 0) continue;
    if ((await streetLength(page)) >= minM) return href;
  }
  throw new Error(`no street of ${minM} m or more with a real end`);
}

/** Opens the first street of the city and makes it pedestrian with the Transform dialog: it then has no lane a bus can
 *  stop in, so no Stops row, and the Add a piece menu greys its stops. Every street of the Plateau has such a lane. */
export async function openPedestrianStreet(page: Page) {
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  await transform(page, ".ped-item", "ped");
  await expect(live(page)).toHaveText(/^Made pedestrian\. /);
  await expect(page.locator("#stop-strip-left, #stop-strip-right")).toHaveCount(0);
}

/** Opens the first street of the city that is at least `minM` metres long (the first is about 40 m, too short for
 *  many stretches of 5 m). */
export async function openLongStreet(page: Page, minM: number) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#plan svg")).toBeVisible();
    if ((await streetLength(page)) >= minM) return href;
  }
  throw new Error(`no street of ${minM} m or more`);
}

/** Opens the Transform dialog, chooses the option matching `option` (its tab first, when the option is not in the
 *  tab the dialog opens on) and applies it. The dialog closes and the focus goes to the drawing. */
export async function transform(page: Page, option: string, tab?: "ped" | "transit" | "templates"): Promise<void> {
  await page.locator("#transform-btn").click();
  if (tab) await page.locator(`#transform-tab-${tab}`).click();
  await page.locator(`#transform-dialog ${option}`).first().click();
  await page.locator("#transform-apply").click();
}

/** Opens the first street of the map that is too narrow for some street template, with the Transform dialog open on its
 *  Street templates tab. Fails if the Plateau tile has none. */
export async function openStreetWithUnfitTemplate(page: Page): Promise<void> {
  await page.goto("/map.html");
  for (const href of (await placesOnMap(page)).streets) {
    await page.goto("/" + href);
    await expect(page.locator("#transform-btn")).toBeVisible();
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-templates").click();
    if ((await page.locator('#transform-dialog [role=option][aria-disabled="true"]').count()) > 0) return;
    await page.keyboard.press("Escape");
  }
  throw new Error("no street of the Plateau tile is too narrow for some template");
}

/** Opens every closed fold of a junction's signal plan level (CLM-49), as a person does to reach what is inside. */
export async function openSignalFolds(page: Page) {
  await expect(page.locator("#signal-plan-state")).toBeVisible();
  // the closed ones, first to last: each one opened leaves the list, so the first closed is always the next
  // only the options' own folds (`jn-fold-…`): the lists of calls inside them are folds too, and stay as they are
  const closed = page.locator(`#inspector details.jn-fold[id^="jn-fold-"]:not([open])`);
  for (let n = await closed.count(); n > 0; n--) {
    await closed.first().locator(":scope > summary").click();
    await expect(closed).toHaveCount(n - 1);
  }
}

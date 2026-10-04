import { readFileSync } from "node:fs";

import type { Page } from "@playwright/test";

import { expect, forgive, live, mapReady, mapStill, PLATEAU_TILES, serveBasemap, serveWorld, test } from "./fixtures";

// The map page works on a real area: the default one, the Plateau Mont-Royal, from a metro tile.
test.use({ area: "world" });

const zoom = (page: Page) => page.evaluate(() => (window as any).cityloomMap.getZoom());
const centre = (page: Page) =>
  page.evaluate(() => {
    const c = (window as any).cityloomMap.getCenter();
    return { lng: c.lng, lat: c.lat };
  });

/** A junction the pointer can reach, in page pixels: one the floating panels and buttons do not cover, and
 *  the only one drawn where it is. */
const junctionSpot = (page: Page) =>
  page.evaluate(() => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    for (const f of map.querySourceFeatures("places")) {
      if (f.geometry.type !== "Point") continue;
      const p = map.project(f.geometry.coordinates);
      const x = r.left + p.x;
      const y = r.top + p.y;
      if (document.elementFromPoint(x, y) !== map.getCanvas()) continue;
      const box = [
        [p.x - 8, p.y - 8],
        [p.x + 8, p.y + 8],
      ];
      const there = map.queryRenderedFeatures(box, { layers: ["places-junction"] });
      if (there.length === 1 && there[0].properties.hot === f.properties.hot) return { x, y, hot: f.properties.hot };
    }
    throw new Error("no junction on the map the pointer can reach");
  });

test.describe("the city map", () => {
  test.beforeEach(async ({ page }) => {
    await serveWorld(page);
    await page.goto("/map.html");
    await mapReady(page);
    await mapStill(page); // the city is fitted: the view the tests start from
  });

  test("shows the area over a basemap, with its streets and junctions as places", async ({ page }) => {
    await expect(page.locator("#title-block")).toContainText("Plateau");
    await expect(page.locator("#basemap canvas")).toBeVisible();
    await expect(page.locator("#basemap")).toHaveAttribute("aria-label", "Map of the city, north up");
    await expect(page.locator(".basemap-note")).toBeHidden();
    await expect(page.locator("#fit")).toHaveText(/ places\. Every check passes\.$/);
    expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
    expect(await page.locator("a.place-row[href^='intersection.html']").count()).toBeGreaterThan(10);
    const kinds = await page.evaluate(() => [
      ...new Set((window as any).cityloomMap.querySourceFeatures("places").map((f: any) => f.geometry.type)),
    ]);
    expect(kinds.sort()).toEqual(["LineString", "Point"]);
  });

  test("the streets lie on the basemap's roads", async ({ page }) => {
    // At a street's middle, the basemap draws a road within a few pixels: the overlay is where the earth is.
    // `idle` comes once the new view's tiles are loaded and drawn, which the old view's being loaded does not say.
    await page.evaluate(
      () =>
        new Promise((done) => {
          const map = (window as any).cityloomMap;
          map.once("idle", done);
          map.jumpTo({ zoom: 16 });
        }),
    );
    await mapStill(page);
    await mapReady(page);
    const result = await page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const lines = map
        .querySourceFeatures("places")
        .filter((f: any) => f.geometry.type === "LineString" && f.properties.width_m >= 6);
      // the basemap style's road layers (scripts/make_basemap_style.mjs)
      const roads = ["road", "road-casing"];
      let on = 0;
      let seen = 0;
      for (const f of lines.slice(0, 40)) {
        const c = f.geometry.coordinates;
        const mid = c[Math.floor(c.length / 2)];
        const p = map.project(mid);
        if (p.x < 0 || p.y < 0 || p.x > map.getCanvas().clientWidth || p.y > map.getCanvas().clientHeight) continue;
        seen++;
        if (
          map.queryRenderedFeatures(
            [
              [p.x - 8, p.y - 8],
              [p.x + 8, p.y + 8],
            ],
            { layers: roads },
          ).length > 0
        )
          on++;
      }
      return { on, seen };
    });
    expect(result.seen).toBeGreaterThanOrEqual(10);
    expect(result.on / result.seen).toBeGreaterThan(0.8);
  });

  test("a place in the list opens its editor", async ({ page }) => {
    const href = await page.locator("a.place-row[href^='street.html']").first().getAttribute("href");
    await page.locator(`a.place-row[href="${href}"]`).click();
    await expect(page).toHaveURL(new RegExp(href!.replace("?", "\\?") + "$"));
  });

  test("every place is a link the keyboard reaches, and Enter on one opens its editor", async ({ page }) => {
    // The map's canvas holds no place to focus: the Places list is how a keyboard gets to each of them.
    const rows = page.locator("#places-panel .places a.place-row");
    const counts = (await page.locator("#places-panel .insp-sub").textContent())!;
    const [, junctions, streets] = /^(\d+) junctions?, (\d+) streets?$/.exec(counts)!;
    expect(await rows.count()).toBe(Number(junctions) + Number(streets));
    const reachable = await rows.evaluateAll(
      (els) => els.filter((a) => (a as HTMLAnchorElement).href && (a as HTMLAnchorElement).tabIndex >= 0).length,
    );
    expect(reachable).toBe(Number(junctions) + Number(streets));

    // Tab goes from one place to the next, and the place with the focus is the one shown.
    await rows.first().focus();
    await page.keyboard.press("Tab");
    const second = rows.nth(1);
    await expect(second).toBeFocused();
    await expect(second).toHaveClass(/\bon\b/);
    const href = (await second.getAttribute("href"))!;
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(new RegExp(href.replace(/[?.]/g, "\\$&") + "$"));
  });

  test("pressing a place on the map opens it", async ({ page }) => {
    const spot = await junctionSpot(page);
    await page.mouse.move(spot.x, spot.y);
    await page.mouse.click(spot.x, spot.y);
    await expect(page).toHaveURL(new RegExp(`intersection\\.html\\?junction=${spot.hot.slice(2)}$`));
  });

  test("the pointer over a place on the map highlights it in the list too", async ({ page }) => {
    const spot = await junctionSpot(page);
    await page.mouse.move(spot.x, spot.y);
    await expect(page.locator("a.place-row.on")).toHaveCount(1);
    await expect(page.locator("a.place-row.on")).toHaveAttribute(
      "href",
      `intersection.html?junction=${spot.hot.slice(2)}`,
    );
    await page.mouse.move(2, 400);
    await expect(page.locator("a.place-row.on")).toHaveCount(0);
  });

  test("zooming in and out moves the view and the whole city button brings it back", async ({ page }) => {
    const fitted = await zoom(page);
    await page.locator("#zoom-in").click();
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    await page.locator("#zoom-out").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(async () => Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
  });

  test("the keys move the map when it has focus: zoom and the four directions", async ({ page }) => {
    const fitted = await zoom(page);
    const start = await centre(page);
    await page.locator("#basemap").focus();
    await page.keyboard.press("+");
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    await page.keyboard.press("0");
    await expect.poll(async () => Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
    // Each pan eases for a moment, and a key pressed before it has moved starts from where the map still is:
    // the test lets each one finish, as a person's presses do.
    const pan = async (key: string) => {
      await page.keyboard.press(key);
      await mapStill(page);
    };
    await pan("ArrowRight");
    const east = await centre(page);
    expect(east.lng).toBeGreaterThan(start.lng);
    await pan("ArrowUp");
    expect((await centre(page)).lat).toBeGreaterThan(start.lat);
    await pan("ArrowLeft");
    expect((await centre(page)).lng).toBeLessThan(east.lng);
    await pan("ArrowDown");
    await pan("ArrowDown");
    expect((await centre(page)).lat).toBeLessThan(start.lat);
  });

  test("the units change the scale", async ({ page }) => {
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/\d\s*m$/);
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/(ft|mi)$/);
  });

  test("a theme changes the basemap and keeps the places and the highlight", async ({ page }) => {
    // While a new style is put in, the map has no layers: the colour is then none.
    const land = () =>
      page.evaluate(() => {
        const map = (window as any).cityloomMap;
        return map.getLayer("background") ? map.getPaintProperty("background", "background-color") : null;
      });
    const light = await land();
    expect(light).not.toBeNull();
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await expect.poll(async () => [null, light].includes(await land())).toBe(false);
    await mapReady(page); // the places came back with the new style
    const spot = await junctionSpot(page);
    await page.mouse.move(spot.x, spot.y);
    await expect(page.locator("a.place-row.on")).toHaveCount(1);
  });

  test("a change made in a street's editor shows on the map and in start over", async ({ page }) => {
    await expect(page.locator("#reset")).toBeDisabled();
    const href = (await page.locator("a.place-row[href^='street.html']").first().getAttribute("href"))!;
    await page.goto("/" + href);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/map.html");
    await mapReady(page);
    // the street, and on a real area perhaps a junction it meets
    await expect(page.locator("#fit")).toHaveText(/^\d+ places? needs? attention: /);
    await expect(page.locator("#fit .tick")).toHaveCount(0);
    await expect(page.locator("#reset")).toBeEnabled();
    const marked = await page.evaluate(() =>
      (window as any).cityloomMap.querySourceFeatures("places").some((f: any) => f.properties.status !== "ok"),
    );
    expect(marked).toBe(true);
    await page.locator("#t-changes").click();
    await expect(page.locator("#changes")).not.toBeEmpty();
  });

  test("start over asks to be pressed twice, and then puts the city back as first laid out", async ({ page }) => {
    const href = (await page.locator("a.place-row[href^='street.html']").first().getAttribute("href"))!;
    await page.goto("/" + href);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/map.html");
    await page.locator("#reset").click();
    await expect(page.locator("#reset-label")).toHaveText("Press again to start over");
    await expect(live(page)).toHaveText(
      "This puts every street and junction back as first laid out. Press again to confirm.",
    );
    await page.locator("#reset").click();
    await expect(page.locator("#reset")).toBeDisabled();
    await expect(page.locator("#fit")).toHaveText(/ places\. Every check passes\.$/);
    await expect(page.locator("#fit .tick.fresh")).toBeVisible();
    await page.goto("/" + href);
    await expect(page.locator("#revs tbody tr")).toHaveCount(1);
  });
});

test.describe("when there is no basemap", () => {
  test("tiles that cannot be had are said, and the page and its places still work", async ({
    page,
    context,
    errors,
  }) => {
    await serveWorld(page);
    await context.unroute("**/data/basemap/montreal.pmtiles");
    await page.route("**/data/basemap/montreal.pmtiles", (route) => route.fulfill({ status: 404, body: "" }));
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("The basemap could not be loaded");
    await expect(page.locator(".basemap-note")).toContainText("just basemap-tiles");
    await expect(page.locator("a.place-row[href^='street.html']")).not.toHaveCount(0);
    expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
    // The failed request, and MapLibre's report of its source failing for it, are the point of the test;
    // anything else still fails it.
    forgive(
      errors,
      /^Failed to load resource: the server responded with a status of 404 \(Not Found\)$/,
      /^Error: Bad response code: 404\n/,
    );
  });

  test("an area whose roads could not be got says so", async ({ page, errors }) => {
    await page.route("**/data/metro/index.json", (route) =>
      route.fulfill({ json: { lon0: 0, lat0: 0, dlon: 1, dlat: 1, tiles: [] } }),
    );
    await page.route("https://overpass-api.de/**", (route) => route.abort());
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("The roads of this place could not be loaded");
    // the refused request is the point of the test; anything else still fails it
    forgive(errors, /^Failed to load resource: net::ERR_FAILED$/);
  });

  test("a place beyond the basemap's coverage is said, and the page and its places still work", async ({
    page,
    context,
  }) => {
    // The fixture's tiles, saying in their header that they cover Berlin: the Plateau is then outside them.
    const tiles = Buffer.from(readFileSync(PLATEAU_TILES));
    expect(tiles.subarray(0, 7).toString("latin1")).toBe("PMTiles");
    expect(tiles[7]).toBe(3); // the header below is version 3's
    // PMTiles v3 header: min lon, min lat, max lon, max lat, little-endian int32 of degrees * 1e7.
    [13.0, 52.3, 13.8, 52.7].forEach((deg, i) => tiles.writeInt32LE(Math.round(deg * 1e7), 102 + 4 * i));
    await serveWorld(page);
    await context.unroute("**/data/basemap/montreal.pmtiles");
    await serveBasemap(context, tiles);
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toHaveText(
      "There is no basemap for this place. The map covers the Montréal area.",
    );
    await expect(page.locator("a.place-row[href^='street.html']")).not.toHaveCount(0);
    expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
    // nothing of the city is drawn on a map of somewhere else
    expect(await page.evaluate(() => !(window as any).cityloomMap.getSource("places"))).toBe(true);
  });
});

test.describe("on the sample city, which has no map", () => {
  test.use({ area: "sample" });
  test("says there is no map, and lists the places", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("could not be loaded");
    await expect(page.locator("a.place-row")).toHaveCount(32);
  });
});

import { readFileSync } from "node:fs";

import type { Page } from "@playwright/test";

import {
  expect,
  forgive,
  live,
  mapReady,
  mapStill,
  openLongStreet,
  openStreet,
  PLATEAU_TILES,
  placesOnMap,
  serveBasemap,
  test,
} from "./fixtures";

// The map page works on a real area: the default one, the Plateau Mont-Royal, from a metro tile.

const zoom = (page: Page) => page.evaluate(() => (window as any).cityloomMap.getZoom());
const centre = (page: Page) =>
  page.evaluate(() => {
    const c = (window as any).cityloomMap.getCenter();
    return { lng: c.lng, lat: c.lat };
  });

/** The colour of the basemap's land: none while a new style is put in, when the map has no layers. */
const land = (page: Page) =>
  page.evaluate(() => {
    const map = (window as any).cityloomMap;
    return map.getLayer("background") ? map.getPaintProperty("background", "background-color") : null;
  });

/** Waits for a new style, whose land is not `before`, to be in and the places to be back on it. */
async function restyled(page: Page, before: unknown) {
  await expect.poll(async () => [null, before].includes(await land(page))).toBe(false);
  await mapReady(page);
}

type Box = { x: number; y: number; width: number; height: number };
const overlaps = (a: Box, b: Box) =>
  a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

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

/** Rendered streets with a spot the pointer can reach, and whether the map says each is editable. At most one spot
 *  per street, and only where the street is the one place under the pointer and no panel or button covers it. */
const streetSpots = (page: Page) =>
  page.evaluate(() => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    const seen = new Map<string, { hot: string; editable: boolean; x: number; y: number }>();
    for (const f of map.queryRenderedFeatures({ layers: ["places-street"] })) {
      const hot = f.properties.hot;
      if (seen.has(hot) || f.geometry.type !== "LineString") continue;
      for (const c of f.geometry.coordinates) {
        const p = map.project(c);
        const x = r.left + p.x;
        const y = r.top + p.y;
        if (p.x < 8 || p.y < 8 || p.x > r.width - 8 || p.y > r.height - 8) continue;
        if (document.elementFromPoint(x, y) !== map.getCanvas()) continue;
        const box = [
          [p.x - 8, p.y - 8],
          [p.x + 8, p.y + 8],
        ];
        const there = new Set(
          map
            .queryRenderedFeatures(box, { layers: ["places-junction", "places-street"] })
            .map((g: any) => g.properties.hot),
        );
        if (there.size !== 1 || !there.has(hot)) continue;
        const state = map.getFeatureState({ source: "places", id: hot });
        seen.set(hot, { hot, editable: state.editable !== false, x, y });
        break;
      }
    }
    return [...seen.values()];
  });

test.describe("the city map", () => {
  test.beforeEach(async ({ page }) => {
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
    const places = await placesOnMap(page);
    expect(places.streets.length).toBeGreaterThan(50);
    expect(places.junctions.length).toBeGreaterThan(10);
    const kinds = await page.evaluate(() => [
      ...new Set((window as any).cityloomMap.querySourceFeatures("places").map((f: any) => f.geometry.type)),
    ]);
    expect(kinds.sort()).toEqual(["LineString", "Point"]);
  });

  test("has no list of places: the area rules float on the left and the map runs to the edge of the window", async ({
    page,
  }) => {
    await expect(page.locator("#places-panel")).toHaveCount(0);
    await expect(page.locator("#reset")).toHaveCount(0);
    await expect(page.locator("#areas-panel")).toBeVisible();
    const map = (await page.locator("#basemap").boundingBox())!;
    expect(map.x).toBe(0);
  });

  test("the button for the Notes is a tab down the right-hand side, outside the header, with vertical text", async ({
    page,
  }) => {
    const tab = page.locator("#notes-toggle");
    await expect(page.locator(".bar #notes-toggle")).toHaveCount(0);
    expect(await tab.evaluate((el) => getComputedStyle(el).writingMode)).toBe("vertical-rl");
    const open = (await tab.boundingBox())!;
    const notes = (await page.locator("#notes").boundingBox())!;
    expect(open.x + open.width).toBeCloseTo(notes.x, 0); // against the Notes
    await tab.click();
    await expect(page.locator("html")).toHaveAttribute("data-notes", "closed");
    const shut = (await tab.boundingBox())!;
    expect(shut.x + shut.width).toBeCloseTo(page.viewportSize()!.width, 0); // at the window's edge
    expect(shut.height).toBeGreaterThan(shut.width);
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

  test("pressing a place on the map opens it", async ({ page }) => {
    const spot = await junctionSpot(page);
    await page.mouse.move(spot.x, spot.y);
    await page.mouse.click(spot.x, spot.y);
    await expect(page).toHaveURL(new RegExp(`intersection\\.html\\?junction=${spot.hot.slice(2)}$`));
  });

  test("zooming in and out moves the view and the whole city button brings it back", async ({ page }) => {
    const fitted = await zoom(page);
    await page.locator("#zoom-in").click();
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    await page.locator("#zoom-out").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(async () => Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
    expect(Math.abs((await zoom(page)) - fitted)).toBeLessThan(0.05);
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

  test("with reduced motion the camera moves at once, without easing", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.reload();
    await mapReady(page);
    const fitted = await zoom(page);
    await page.locator("#basemap").focus();
    await page.keyboard.press("+");
    await expect.poll(() => zoom(page)).toBeGreaterThan(fitted + 0.3);
    expect(await page.evaluate(() => (window as any).cityloomMap.isMoving())).toBe(false);
  });

  test("the map is one stop for the keyboard: its canvas is not a second one", async ({ page }) => {
    await expect(page.locator("#basemap canvas")).toHaveAttribute("tabindex", "-1");
    await page.locator("#basemap").focus();
    await page.keyboard.press("Tab");
    const next = await page.evaluate(() => document.activeElement?.tagName);
    expect(next).not.toBe("CANVAS");
    // Shift+Tab from there comes back to the map itself
    await page.keyboard.press("Shift+Tab");
    await expect(page.locator("#basemap")).toBeFocused();
  });

  test("the units change the scale", async ({ page }) => {
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/\d\s*m$/);
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator(".maplibregl-ctrl-scale")).toContainText(/(ft|mi)$/);
  });

  test("the scale and the attribution stay clear of the panels, the status bar and the zoom buttons", async ({
    page,
  }) => {
    const box = async (selector: string) => (await page.locator(selector).boundingBox())!;
    await expect(page.locator(".maplibregl-ctrl-scale")).toBeVisible();
    // the attribution starts open, as a line of text
    await expect(page.locator(".maplibregl-ctrl-attrib")).toContainText("OpenStreetMap");
    const map = await box("#basemap");
    const scale = await box(".maplibregl-ctrl-scale");
    const left = await box("#areas-panel");
    expect(scale.x - (left.x + left.width)).toBeLessThan(24);
    const attribution = await box(".maplibregl-ctrl-attrib");
    for (const covers of ["#areas-panel", "#notes", ".statusbar", "#map-tools-slot"]) {
      expect(overlaps(scale, await box(covers)), `the scale under ${covers}`).toBe(false);
      expect(overlaps(attribution, await box(covers)), `the attribution under ${covers}`).toBe(false);
    }
  });

  test("on a phone the scale and the attribution stay clear of the status line and the zoom buttons", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    const box = async (selector: string) => (await page.locator(selector).boundingBox())!;
    await expect(page.locator(".maplibregl-ctrl-scale")).toBeVisible();
    await expect(page.locator(".maplibregl-ctrl-attrib")).toContainText("OpenStreetMap");
    const [scale, attribution, map] = [
      await box(".maplibregl-ctrl-scale"),
      await box(".maplibregl-ctrl-attrib"),
      await box("#basemap"),
    ];
    for (const control of [scale, attribution]) {
      expect(control.x).toBeGreaterThanOrEqual(map.x);
      expect(control.y).toBeGreaterThanOrEqual(map.y);
      expect(control.x + control.width).toBeLessThanOrEqual(map.x + map.width);
      expect(overlaps(control, await box(".statusbar"))).toBe(false);
      expect(overlaps(control, await box("#map-tools-slot"))).toBe(false);
    }
    expect(overlaps(scale, attribution)).toBe(false);
  });

  test("a theme changes the basemap and keeps the places, and the pointer on one still lights it", async ({ page }) => {
    const light = await land(page);
    expect(light).not.toBeNull();
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await restyled(page, light);
    // the pointer on a place lights it
    const spot = await junctionSpot(page);
    await page.mouse.move(spot.x, spot.y);
    await expect
      .poll(() =>
        page.evaluate(
          (id) => (window as any).cityloomMap.getFeatureState({ source: "places", id }).hot === true,
          spot.hot,
        ),
      )
      .toBe(true);
  });

  test("a change made in a street's editor shows on the map", async ({ page }) => {
    const href = (await placesOnMap(page)).streets[0];
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
    const marked = await page.evaluate(() =>
      (window as any).cityloomMap.querySourceFeatures("places").some((f: any) => f.properties.status !== "ok"),
    );
    expect(marked).toBe(true);
    await page.locator("#t-changes").click();
    await expect(page.locator("#changes")).not.toBeEmpty();
  });

  test("a place the open view does not wholly hold is muted and opens nothing, and one that it holds opens", async ({
    page,
  }) => {
    // Zoomed in about the middle of the city, so that the edges of the view cut some streets and hold others.
    await page.evaluate(() => (window as any).cityloomMap.jumpTo({ zoom: 16.8 }));
    await mapStill(page);
    await expect
      .poll(async () => {
        const spots = await streetSpots(page);
        return [spots.some((s) => s.editable), spots.some((s) => !s.editable)];
      })
      .toEqual([true, true]);
    const spots = await streetSpots(page);
    const muted = spots.find((s) => !s.editable)!;
    const here = page.url();
    await page.mouse.click(muted.x, muted.y);
    await expect(live(page)).toHaveText(
      "Not all of this place is in view. Move the map so all of it shows, then try again.",
    );
    expect(page.url()).toBe(here);
    const open = spots.find((s) => s.editable)!;
    await page.mouse.click(open.x, open.y);
    await expect(page).toHaveURL(/street\.html\?street=\d+/);
  });

  test("the muted places stay muted when the basemap changes style", async ({ page }) => {
    await page.evaluate(() => (window as any).cityloomMap.jumpTo({ zoom: 16.8 }));
    await mapStill(page);
    await expect.poll(async () => (await streetSpots(page)).some((s) => !s.editable)).toBe(true);
    const before = await land(page);
    await page.emulateMedia({ colorScheme: "dark" });
    await restyled(page, before);
    await expect.poll(async () => (await streetSpots(page)).some((s) => !s.editable)).toBe(true);
  });

  test("on a phone, where the panels stack, the places are not all muted", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 800 });
    await page.goto("/map.html");
    await mapReady(page);
    await mapStill(page);
    await expect.poll(async () => (await streetSpots(page)).some((s) => s.editable)).toBe(true);
  });

  test("the map draws pedestrian streets with a dotted line and says each street's class", async ({ page }) => {
    await page.goto("/map.html");
    await mapReady(page);
    expect(await page.evaluate(() => !!(window as any).cityloomMap.getLayer("places-pedestrian"))).toBe(true);
    const classes: unknown[] = await page.evaluate(() =>
      (window as any).cityloomMap
        .querySourceFeatures("places")
        .filter((f: any) => f.geometry.type === "LineString")
        .map((f: any) => f.properties.class),
    );
    expect(classes.length).toBeGreaterThan(0);
    expect(classes.every((c) => typeof c === "string")).toBe(true);
  });

  test("a street made pedestrian is a pedestrian street on the map", async ({ page }) => {
    const href = await openStreet(page);
    const hot = `s-${href.split("=")[1]}`;
    await page.locator("#transform-btn").click();
    await page.locator("#transform-menu .ped-item").first().click();
    await page.goto("/map.html"); // leaving the page keeps what was waiting to be kept
    await mapReady(page);
    await page.waitForFunction(
      (id) =>
        (window as any).cityloomMap
          .querySourceFeatures("places")
          .some((f: any) => f.properties.hot === id && f.properties.class === "pedestrian"),
      hot,
    );
  });
});

test.describe("when there is no basemap", () => {
  test("tiles that cannot be had are said, and the page and its notes still work", async ({
    page,
    context,
    errors,
  }) => {
    await context.unroute("**/data/basemap/montreal.pmtiles");
    await page.route("**/data/basemap/montreal.pmtiles", (route) => route.fulfill({ status: 404, body: "" }));
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toContainText("The basemap could not be loaded");
    await expect(page.locator(".basemap-note")).toContainText("just prepare");
    // the city is in the Notes, though there is no map to draw it on
    expect(Number(await page.locator("#tb-places").innerText())).toBeGreaterThan(50);
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
    // no places stand in for the area, and the page names the area it could not load
    await expect(page.locator("#fit")).toHaveText("No roads to show.");
    await expect(page.locator("#title-block")).toContainText("Plateau Mont-Royal");
    await expect(page.locator("#title-block")).not.toContainText("Sample city");
    // the refused request is the point of the test; anything else still fails it
    forgive(errors, /^Failed to load resource: net::ERR_FAILED$/);
  });

  test("a place beyond the basemap's coverage is said, and the page and its notes still work", async ({
    page,
    context,
  }) => {
    // The fixture's tiles, saying in their header that they cover Berlin: the Plateau is then outside them.
    const tiles = Buffer.from(readFileSync(PLATEAU_TILES));
    expect(tiles.subarray(0, 7).toString("latin1")).toBe("PMTiles");
    expect(tiles[7]).toBe(3); // the header below is version 3's
    // PMTiles v3 header: min lon, min lat, max lon, max lat, little-endian int32 of degrees * 1e7.
    [13.0, 52.3, 13.8, 52.7].forEach((deg, i) => tiles.writeInt32LE(Math.round(deg * 1e7), 102 + 4 * i));
    await context.unroute("**/data/basemap/montreal.pmtiles");
    await serveBasemap(context, tiles);
    await page.goto("/map.html");
    await expect(page.locator(".basemap-note")).toHaveText(
      "There is no basemap for this place. The map covers the Montréal area.",
    );
    // the city is in the Notes, though there is no map to draw it on
    expect(Number(await page.locator("#tb-places").innerText())).toBeGreaterThan(50);
    // nothing of the city is drawn on a map of somewhere else
    expect(await page.evaluate(() => !(window as any).cityloomMap.getSource("places"))).toBe(true);
  });
});

// ---- the boundaries of a street's zones ---------------------------------------------------------------------

/** The street's zone boundaries as ticks, as the map holds them. `querySourceFeatures` may give a feature once for
 *  each tile it is cut into, so equal ticks are counted once. */
const ticks = (page: Page) =>
  page.evaluate(() => {
    const seen = new Set<string>();
    return (window as any).cityloomMap
      .querySourceFeatures("zone-ticks")
      .map((f: any) => ({
        type: f.geometry.type,
        coordinates: f.geometry.coordinates,
        band: f.properties.band === true || f.properties.band === "true",
      }))
      .filter((t: any) => {
        const key = JSON.stringify(t.coordinates.map((c: number[]) => c.map((v) => Math.round(v * 1e6))));
        return !seen.has(key) && !!seen.add(key);
      });
  });

/** Splits a long street of the city in two on its page and goes back to the map. Says the street's name. */
async function splitOneStreet(page: Page) {
  await openLongStreet(page, 20);
  const name = ((await page.locator("#street-name").textContent()) ?? "").trim();
  await page.locator("#zone-split").click();
  await expect(page.locator('#zone-strip [role="option"]')).toHaveCount(2);
  await page.goto("/map.html");
  await mapReady(page);
  return name;
}

test.describe("the zone boundaries on the map", () => {
  test("a street without zones adds no ticks", async ({ page }) => {
    await page.goto("/map.html");
    await mapReady(page);
    await expect.poll(() => page.evaluate(() => !!(window as any).cityloomMap.getLayer("zone-ticks"))).toBe(true);
    expect(await ticks(page)).toHaveLength(0);
  });

  test("a zone boundary is a tick across its street", async ({ page }) => {
    await splitOneStreet(page);
    await expect
      .poll(() =>
        page.evaluate(() => {
          const m = (window as any).cityloomMap;
          return !!m.getLayer("zone-ticks") && !!m.getLayer("zone-ticks-casing");
        }),
      )
      .toBe(true);
    await expect.poll(async () => (await ticks(page)).length).toBe(1);
    const [tick] = await ticks(page);
    expect(tick.type).toBe("LineString");
    expect(tick.coordinates).toHaveLength(2);
    // the two points lie on opposite sides of the street's centreline
    const side = await page.evaluate(
      ([a, b]) => {
        const hot = (window as any).cityloomMap
          .querySourceFeatures("places")
          .filter((f: any) => String(f.properties.hot).startsWith("s-") && f.geometry.type === "LineString");
        const sideOf = (line: number[][], p: number[]) => {
          let best = Infinity;
          let sign = 0;
          for (let i = 0; i + 1 < line.length; i++) {
            const [x1, y1] = line[i];
            const [x2, y2] = line[i + 1];
            const t = Math.max(
              0,
              Math.min(1, ((p[0] - x1) * (x2 - x1) + (p[1] - y1) * (y2 - y1)) / ((x2 - x1) ** 2 + (y2 - y1) ** 2)),
            );
            const d = Math.hypot(p[0] - (x1 + t * (x2 - x1)), p[1] - (y1 + t * (y2 - y1)));
            if (d < best) {
              best = d;
              sign = Math.sign((x2 - x1) * (p[1] - y1) - (y2 - y1) * (p[0] - x1));
            }
          }
          return { best, sign };
        };
        let found: number[] | null = null;
        let min = Infinity;
        for (const f of hot) {
          const line = f.geometry.coordinates;
          const d = sideOf(line, [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]).best;
          if (d < min) {
            min = d;
            found = line;
          }
        }
        return [sideOf(found!, a).sign, sideOf(found!, b).sign];
      },
      tick.coordinates as [number[], number[]],
    );
    expect(side[0] * side[1], "opposite sides of the centreline").toBe(-1);
  });

  test("a boundary with a transition is a band along the street", async ({ page }) => {
    await openLongStreet(page, 20);
    await page.locator("#zone-split").click();
    await expect(page.locator('#zone-strip [role="option"]')).toHaveCount(2);
    await page.locator("#zone-1").click();
    await page.locator("#add-btn").click();
    await page.locator(".add-item", { hasText: "Driving lane" }).click();
    await page.goto("/map.html"); // leaving the page flushes the debounced write
    await mapReady(page);
    await expect.poll(async () => (await ticks(page)).filter((t: any) => t.band).length).toBe(1);
    expect(await ticks(page), "the band replaces the tick").toHaveLength(1);
  });

  test("the street's label gives its zone count", async ({ page }) => {
    const name = await splitOneStreet(page);
    await page.locator("#search").fill(name);
    const row = page.locator("#search-results .place-row", { hasText: "2 zones" });
    await expect(row.first()).toBeVisible();
    await expect(row.first().locator("small")).toHaveText(/ \u00b7 2 zones$/);
  });

  test("a theme change keeps the ticks", async ({ page }) => {
    await splitOneStreet(page);
    await expect.poll(async () => (await ticks(page)).length).toBe(1);
    const light = await land(page);
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await restyled(page, light);
    await expect
      .poll(() =>
        page.evaluate(() => {
          const m = (window as any).cityloomMap;
          return (
            !!m.getLayer("zone-ticks") &&
            !!m.getLayer("zone-ticks-casing") &&
            m.querySourceFeatures("zone-ticks").length > 0
          );
        }),
      )
      .toBe(true);
    expect(await ticks(page)).toHaveLength(1);
  });
});

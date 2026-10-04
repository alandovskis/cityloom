import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test } from "./fixtures";

// The pages open the area of the world that was chosen: before one is, the Plateau Mont-Royal. There are
// no metro tiles in the tests, so one is served: a tile of one place, holding a real extract of the Plateau.
test.use({ area: "world" });

const PLATEAU = fileURLToPath(new URL("../fixtures/plateau.osm.pbf", import.meta.url));

test.beforeEach(async ({ page }) => {
  await page.route("**/data/metro/index.json", (route) =>
    route.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await page.route("**/data/metro/0_0.osm.pbf", (route) => route.fulfill({ body: readFileSync(PLATEAU) }));
  await page.route("https://overpass-api.de/**", (route) => route.abort());
});

test("the map opens on the default area, made of its real streets", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("Plateau");
  await expect(page.locator("#title-block")).not.toContainText("Sample");
  expect(await page.locator("#map-slot a[href^='street.html']").count()).toBeGreaterThan(50);
  expect(await page.locator("#map-slot a[href^='intersection.html']").count()).toBeGreaterThan(10);
  // a street of the area, by its real name
  await expect(page.locator("#map-slot")).toContainText(/Saint-Laurent|Rue |Avenue |Boulevard /);
});

test("a junction of the area opens in the junction editor and a street in the street editor", async ({ page }) => {
  await page.goto("/map.html");
  const junction = await page.locator("#map-slot a[href^='intersection.html']").first().getAttribute("href");
  await page.goto("/" + junction!);
  await expect(page.locator(".plan, #plan").first()).toBeVisible();
  await page.goto("/map.html");
  const street = await page.locator("#map-slot a[href^='street.html']").first().getAttribute("href");
  await page.goto("/" + street!);
  await expect(page.locator("#drawing")).toBeVisible();
});

test("an edit made in the area is kept for it and not in the sample city", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("Plateau");
  const keys = await page.evaluate(() => Object.keys(localStorage));
  expect(keys.some((k) => k.startsWith("cityloom-network:45.5261,-73.5978"))).toBe(true);
  expect(keys).not.toContain("cityloom-city");
});

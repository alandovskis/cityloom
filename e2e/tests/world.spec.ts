import { expect, test } from "./fixtures";

// The pages open the area of the world that was chosen: before one is, the Plateau Mont-Royal. There are
// no metro tiles in the tests, so one is served: a tile of one place, holding a real extract of the Plateau.

test("the map opens on the default area, made of its real streets", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("Plateau");
  expect(await page.locator("a.place-row[href^='street.html']").count()).toBeGreaterThan(50);
  expect(await page.locator("a.place-row[href^='intersection.html']").count()).toBeGreaterThan(10);
  // a street of the area, by its real name
  await expect(page.locator("#places-panel")).toContainText(/Saint-Laurent|Rue |Avenue |Boulevard /);
});

test("a junction of the area opens in the junction editor and a street in the street editor", async ({ page }) => {
  await page.goto("/map.html");
  const junction = await page.locator("a.place-row[href^='intersection.html']").first().getAttribute("href");
  await page.goto("/" + junction!);
  await expect(page.locator(".plan, #plan").first()).toBeVisible();
  await page.goto("/map.html");
  const street = await page.locator("a.place-row[href^='street.html']").first().getAttribute("href");
  await page.goto("/" + street!);
  await expect(page.locator("#drawing")).toBeVisible();
});

test("what is kept for the area is kept under its own keys", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("Plateau");
  const keys = await page.evaluate(() => Object.keys(localStorage));
  expect(keys.some((k) => k.startsWith("cityloom-network:45.5261,-73.5978"))).toBe(true);
  expect(keys).not.toContain("cityloom-city");
});

test("an area is made again when the checksum of its tile changes, and not otherwise", async ({ page }) => {
  let checksum = "one";
  await page.route("**/data/metro/index.json", (route) =>
    route.fulfill({
      json: {
        lon0: -73.6078,
        lat0: 45.5161,
        dlon: 0.0257,
        dlat: 0.018,
        tiles: ["0_0"],
        checksums: { "0_0": checksum },
      },
    }),
  );
  let tileReads = 0;
  page.on("request", (request) => {
    if (request.url().endsWith("/data/metro/0_0.osm.pbf")) tileReads++;
  });
  const key = "cityloom-checksum:45.5261,-73.5978";
  const stored = () => page.evaluate((k) => localStorage.getItem(k), key);

  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("Plateau");
  expect([tileReads, await stored()]).toEqual([1, "one"]);

  await page.reload();
  await expect(page.locator("#title-block")).toContainText("Plateau");
  expect(tileReads, "the same checksum: the roads kept stand").toBe(1);

  checksum = "two";
  await page.reload();
  await expect(page.locator("#title-block")).toContainText("Plateau");
  expect([tileReads, await stored()]).toEqual([2, "two"]);
});

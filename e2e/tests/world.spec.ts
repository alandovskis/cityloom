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

import { expect, test } from "./fixtures";

// The pages open the area of the world that was chosen: before one is, the one that ships with the app.
test.use({ area: "world" });

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

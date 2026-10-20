import { readFileSync } from "node:fs";

import { expect, PLATEAU_PBF, test } from "./fixtures";

test.describe("the home page's search", () => {
  test("can be used before the roads of the city are got", async ({ page }) => {
    let release!: () => void;
    const held = new Promise<void>((resolve) => (release = resolve));
    // The roads come from a tile; hold it back.
    await page.route("**/data/metro/0_0.osm.pbf", async (route) => {
      await held;
      await route.fulfill({ body: readFileSync(PLATEAU_PBF) });
    });
    await page.goto("/");
    const search = page.locator("#search");
    await expect(search).toBeVisible();
    await expect(search).toBeEnabled();
    await search.fill("Plateau");
    await expect(search).toHaveValue("Plateau");
    // the city behind the card is not there yet
    await expect(page.locator("#hero-facts-slot .hero-facts")).toHaveCount(0);
    release();
    await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
    await expect(page.locator(".hero-map svg#map")).toBeVisible();
  });
});

// The footer: the product's version, on every page of the app.

import type { Page } from "@playwright/test";

import { expect, mapReady, openJunction, openStreet, test } from "./fixtures";

const VERSION = /^Version \d+\.\d+\.\d+$/;

const PAGES = [
  { name: "home", open: (page: Page) => page.goto("/") },
  {
    name: "map",
    open: async (page: Page) => {
      await page.goto("/map.html");
      await mapReady(page);
    },
  },
  { name: "street", open: (page: Page) => openStreet(page) },
  { name: "junction", open: (page: Page) => openJunction(page) },
];

for (const { name, open } of PAGES) {
  test(`the ${name} page ends with a footer that says the version`, async ({ page }) => {
    await open(page);
    await expect(page.locator("footer.site-foot")).toHaveText(VERSION);
  });

  test(`the ${name} page's footer covers no control`, async ({ page }) => {
    await open(page);
    const foot = await page.locator("footer.site-foot").boundingBox();
    expect(foot).not.toBeNull();
    const clash = await page.evaluate(({ x, y, width, height }) => {
      const hits = new Set<string>();
      for (let px = x; px < x + width; px += 4) {
        for (let py = y; py < y + height; py += 4) {
          for (const el of document.elementsFromPoint(px, py)) {
            if (el.closest("footer.site-foot") || el === document.documentElement || el === document.body) continue;
            if (el.matches("a, button, input, select, .panel, .statusbar, .zoom-dock, .maplibregl-ctrl"))
              hits.add(el.className || el.tagName);
          }
        }
      }
      return [...hits];
    }, foot!);
    expect(clash).toEqual([]);
  });
}

test("under 900px the footer ends the page, below the last card", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 800 });
  await openStreet(page);
  const foot = (await page.locator("footer.site-foot").boundingBox())!;
  const last = (await page.locator(".panel.left").boundingBox())!;
  expect(foot.y).toBeGreaterThanOrEqual(last.y + last.height);
});

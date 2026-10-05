import type { Page } from "@playwright/test";

import { expect, openJunction, openStreet, test } from "./fixtures";

const box = async (page: Page, selector: string) => (await page.locator(selector).first().boundingBox())!;
const below = (a: { y: number; height: number }, b: { y: number }) => a.y + a.height <= b.y + 1;

for (const p of [
  { name: "street", open: openStreet, head: ["#street-name", "#add-btn", "#undo"] },
  { name: "junction", open: openJunction, head: ["#street-name", "#undo"] },
]) {
  test.describe(`the ${p.name} editor's layout`, () => {
    test.beforeEach(async ({ page }) => {
      await p.open(page);
      await expect(page.locator("#drawing")).toBeVisible();
    });

    test("the title and the tools sit above the drawing", async ({ page }) => {
      const drawing = await box(page, ".drawing");
      for (const s of p.head) {
        await expect(page.locator(s)).toBeVisible();
        expect(below(await box(page, s), drawing)).toBe(true);
      }
    });

    test("the drawing sits between the two panels and is overlapped by neither", async ({ page }) => {
      const left = await box(page, ".panel.left");
      const right = await box(page, ".panel.right");
      const drawing = await box(page, ".drawing");
      expect(drawing.x).toBeGreaterThanOrEqual(left.x + left.width);
      expect(drawing.x + drawing.width).toBeLessThanOrEqual(right.x);
    });

    test("the status chip is under the drawing, not over it", async ({ page }) => {
      const drawing = await box(page, ".drawing");
      const chip = await box(page, "#fit");
      expect(below(drawing, chip)).toBe(true);
      await expect(page.locator("#fit")).toBeInViewport();
    });

    test("the whole drawing is in view without scrolling the page", async ({ page }) => {
      await expect(page.locator("#drawing")).toBeInViewport({ ratio: 0.99 });
    });
  });
}

test("the street is not cut by a scrollbar at the usual laptop width, with the notes closed", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  const scroll = await page.locator("#scroll").evaluate((el) => ({ scroll: el.scrollWidth, client: el.clientWidth }));
  expect(scroll.scroll).toBeLessThanOrEqual(scroll.client + 1);
});

test.describe("on a screen too narrow for the notes beside the drawing", () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test("they start closed and open when asked, and stay open", async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#notes")).toBeHidden();
    await expect(page.locator("#notes-toggle")).toHaveAttribute("aria-expanded", "false");
    await page.locator("#notes-toggle").click();
    await expect(page.locator("#notes")).toBeVisible();
    await page.reload();
    await expect(page.locator("#notes")).toBeVisible();
  });

  test("on the map they are open as before", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#notes")).toBeVisible();
  });
});

test("on a phone the panels stack under the drawing, and nothing overlaps it", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  const drawing = await box(page, ".drawing");
  const status = await box(page, "#fit");
  const left = await box(page, ".panel.left");
  expect(below(drawing, status)).toBe(true);
  expect(below(drawing, left)).toBe(true);
});

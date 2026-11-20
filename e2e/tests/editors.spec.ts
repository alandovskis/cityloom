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

    test("the whole drawing is in view without scrolling the page", async ({ page }) => {
      await expect(page.locator("#drawing")).toBeInViewport({ ratio: 0.99 });
    });
  });
}

// The junction editor: the details on the left, the notes on the right, the drawing between them.
test.describe("the junction editor's sidebars", () => {
  test.beforeEach(async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
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
});

// The street editor (layout option 1 of the editors' review): one panel on the right for what is chosen, and the
// notes as a tray under the drawing whose handle is the status line and the notes' tabs.
test.describe("the street editor's panel and tray", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("the details panel is right of the drawing and overlaps it nowhere", async ({ page }) => {
    const drawing = await box(page, ".drawing");
    const details = await box(page, "#details-panel");
    expect(drawing.x + drawing.width).toBeLessThanOrEqual(details.x);
    await expect(page.locator(".panel.left")).toHaveCount(0);
  });

  test("the notes are a tray under the drawing, its handle the status line and the tabs, in view", async ({ page }) => {
    const drawing = await box(page, ".drawing");
    const tray = await box(page, "#notes");
    const details = await box(page, "#details-panel");
    expect(below(drawing, tray)).toBe(true);
    expect(tray.x + tray.width).toBeLessThanOrEqual(details.x);
    await expect(page.locator("#notes #fit")).toBeInViewport();
    await expect(page.locator("#notes #t-space")).toBeInViewport();
    await expect(page.locator("#notes #notes-toggle")).toBeInViewport();
  });

  test("the title and the tools stay in view while the plan below the drawing is scrolled", async ({ page }) => {
    await page.locator(".stage-scroll").evaluate((el) => el.scrollTo(0, el.scrollHeight));
    await expect(page.locator("#plan")).toBeInViewport();
    for (const s of ["#street-name", "#add-btn", "#undo"]) await expect(page.locator(s)).toBeInViewport();
    await expect(page.locator("#notes #fit")).toBeInViewport();
  });

  test("closing the tray leaves its handle; a tab opens it on that tab", async ({ page }) => {
    const toggle = page.locator("#notes-toggle");
    if ((await toggle.getAttribute("aria-expanded")) === "true") await toggle.click();
    await expect(page.locator("html")).toHaveAttribute("data-notes", "closed");
    await expect(page.locator("#p-space")).toBeHidden();
    await expect(page.locator("#notes #t-checks")).toBeVisible();
    await page.locator("#t-checks").click();
    await expect(page.locator("html")).toHaveAttribute("data-notes", "open");
    await expect(page.locator("#p-checks")).toBeVisible();
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
  });

  test("the open tray takes no more than a quarter of the column's height", async ({ page }) => {
    await page.locator("#t-space").click();
    await expect(page.locator("html")).toHaveAttribute("data-notes", "open");
    const column = await box(page, ".stage-main");
    const tray = await box(page, "#notes");
    expect(tray.height).toBeLessThanOrEqual(column.height * 0.24 + 1);
    await expect(page.locator("#notes #notes-toggle")).toBeInViewport();
  });

  test("the details' edge tab is at the panel's left edge and goes to the window's edge when it collapses", async ({
    page,
  }) => {
    const width = page.viewportSize()!.width;
    const tab = page.locator("#inspector-toggle");
    expect(await tab.evaluate((el) => getComputedStyle(el).writingMode)).toBe("vertical-rl");
    const open = (await tab.boundingBox())!;
    const panel = await box(page, "#details-panel");
    expect(open.x + open.width).toBeCloseTo(panel.x, 0);
    await tab.click();
    await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
    await expect(page.locator("#details-panel")).toBeHidden();
    const shut = (await tab.boundingBox())!;
    expect(shut.x + shut.width).toBeCloseTo(width, 0);
    // the drawing takes the room the panel gave up
    expect((await box(page, ".drawing")).x + (await box(page, ".drawing")).width).toBeGreaterThan(panel.x);
  });
});

test("the street is not cut by a scrollbar at the usual laptop width, with the notes closed", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  const scroll = await page.locator("#scroll").evaluate((el) => ({ scroll: el.scrollWidth, client: el.clientWidth }));
  expect(scroll.scroll).toBeLessThanOrEqual(scroll.client + 1);
});

test.describe("on a screen too narrow for the notes beside the drawing", () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test("the junction's start closed and open when asked, and stay open", async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#notes")).toBeHidden();
    await expect(page.locator("#notes-toggle")).toHaveAttribute("aria-expanded", "false");
    await page.locator("#notes-toggle").click();
    await expect(page.locator("#notes")).toBeVisible();
    await page.reload();
    await expect(page.locator("#notes")).toBeVisible();
  });

  test("the street's tray starts closed to its handle, opens when asked, and stays open", async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#notes #fit")).toBeVisible();
    await expect(page.locator("#p-space")).toBeHidden();
    await expect(page.locator("#notes-toggle")).toHaveAttribute("aria-expanded", "false");
    await page.locator("#notes-toggle").click();
    await expect(page.locator("#p-space")).toBeVisible();
    await page.reload();
    await expect(page.locator("#p-space")).toBeVisible();
  });

  test("on the map they are open as before", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#notes")).toBeVisible();
  });
});

test("a surface's name is shown whole beside its swatch, at a desktop width and on a phone", async ({ page }) => {
  await openStreet(page);
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 844 });
    await page.locator("#piece-list [role=option]").first().click();
    const name = page.locator("#inspector .opt[role=radio]:has(.swatch) span").first();
    await expect(name).toBeVisible();
    const fit = await name.evaluate((el) => ({ width: el.clientWidth, needs: el.scrollWidth }));
    expect(fit.width).toBeGreaterThan(40);
    expect(fit.needs).toBeLessThanOrEqual(fit.width);
  }
});

test("on a phone the panels stack under the drawing, and nothing overlaps it", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  const drawing = await box(page, ".drawing");
  const status = await box(page, "#fit");
  const details = await box(page, "#details-panel");
  expect(below(drawing, status)).toBe(true);
  expect(below(drawing, details)).toBe(true);
});

test.describe("the junction editor's sidebar tabs", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("each sidebar has an edge tab outside the header, at its panel's edge, that moves when it collapses", async ({
    page,
  }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    const width = page.viewportSize()!.width;
    for (const side of [
      { toggle: "#inspector-toggle", panel: "#details-panel", attr: "data-inspector" },
      { toggle: "#notes-toggle", panel: "#notes", attr: "data-notes" },
    ]) {
      const tab = page.locator(side.toggle);
      const left = side.attr === "data-inspector";
      await expect(page.locator(`.bar ${side.toggle}`)).toHaveCount(0);
      expect(await tab.evaluate((el) => getComputedStyle(el).writingMode)).toBe("vertical-rl");
      const open = (await tab.boundingBox())!;
      const panel = (await page.locator(side.panel).boundingBox())!;
      expect(left ? open.x : open.x + open.width).toBeCloseTo(left ? panel.x + panel.width : panel.x, 0);
      await tab.click();
      await expect(page.locator("html")).toHaveAttribute(side.attr, "closed");
      const shut = (await tab.boundingBox())!;
      expect(left ? shut.x : shut.x + shut.width).toBeCloseTo(left ? 0 : width, 0);
      expect(shut.height).toBeGreaterThan(shut.width);
    }
  });
});

test.describe("on a phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("the 2.5D is scaled to the room it has: the whole street is in view and nothing scrolls sideways", async ({
    page,
  }) => {
    await openStreet(page);
    await expect(page.locator("#drawing.iso")).toBeVisible();
    const drawing = await box(page, "#drawing");
    expect(drawing.x).toBeGreaterThanOrEqual(0);
    expect(drawing.x + drawing.width).toBeLessThanOrEqual(390);
    const scroll = await page.locator("#scroll").evaluate((el) => ({ scroll: el.scrollWidth, client: el.clientWidth }));
    expect(scroll.scroll).toBeLessThanOrEqual(scroll.client + 1);
  });
});

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

// The street editor (layout option 1 of the editors' review): one panel on the right for what is chosen, and the
// notes as a tray under the drawing whose handle is the status line and the notes' tabs. The junction editor has the
// same layout (CLM-32, docs/superpowers/specs/2026-11-22-junction-editor-layout-design.md).
for (const p of [
  { name: "street", open: openStreet },
  { name: "junction", open: openJunction },
]) {
  test.describe(`the ${p.name} editor's panel and tray`, () => {
    test.use({ viewport: { width: 1440, height: 900 } });

    test.beforeEach(async ({ page }) => {
      await p.open(page);
      await expect(page.locator("#drawing")).toBeVisible();
    });

    test("the details panel is right of the drawing and overlaps it nowhere", async ({ page }) => {
      const drawing = await box(page, ".drawing");
      const details = await box(page, "#details-panel");
      expect(drawing.x + drawing.width).toBeLessThanOrEqual(details.x);
      await expect(page.locator(".panel.left")).toHaveCount(0);
    });

    test("the notes are a tray under the drawing, its handle the status line and the tabs, in view", async ({
      page,
    }) => {
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
      test.skip(p.name !== "street", "only the street has a plan and strips under its drawing");
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

  test("the junction's tray starts closed to its handle, opens when asked, and stays open", async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#notes #fit")).toBeVisible();
    await expect(page.locator("#p-space")).toBeHidden();
    await expect(page.locator("#notes-toggle")).toHaveAttribute("aria-expanded", "false");
    await page.locator("#notes-toggle").click();
    await expect(page.locator("#p-space")).toBeVisible();
    await page.reload();
    await expect(page.locator("#p-space")).toBeVisible();
  });

  test("the junction's whole drawing, its head and the tray's handle are in view with the tray open", async ({
    page,
  }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator("#t-space").click();
    await expect(page.locator("#p-space")).toBeVisible();
    await expect(page.locator("#drawing")).toBeInViewport({ ratio: 0.99 });
    for (const s of ["#street-name", "#undo", "#notes #notes-toggle"]) await expect(page.locator(s)).toBeInViewport();
    const sideways = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(sideways).toBeLessThanOrEqual(0);
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

  test("the junction's drawing fills its column, keeps its key and part buttons in view, and the same size after the tray", async ({
    page,
  }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    const column = page.locator(".stage-scroll");
    const height = async () => (await box(page, "#drawing")).height;
    const settled = async () => {
      let last = -1;
      await expect
        .poll(async () => {
          const now = await height();
          const same = now === last;
          last = now;
          return same;
        })
        .toBe(true);
      return last;
    };
    const closed = await settled();
    // with the tray closed the drawing's card takes the column, no more (it does not scroll) and no less
    const fit = await column.evaluate((el) => ({
      overflow: el.scrollHeight - el.clientHeight,
      left: el.clientHeight - (el.querySelector(".plan-drawing") as HTMLElement).offsetHeight,
    }));
    expect(fit.overflow).toBeLessThanOrEqual(1);
    expect(fit.left).toBeLessThanOrEqual(8);
    for (const s of ["#plan-key", "#part-next"]) await expect(page.locator(s)).toBeInViewport();
    await page.locator("#t-space").click();
    await expect(page.locator("#p-space")).toBeVisible();
    await settled();
    for (const s of ["#plan-key", "#part-next", "#drawing"]) await expect(page.locator(s)).toBeInViewport();
    await page.locator("#notes-toggle").click();
    await expect(page.locator("#p-space")).toBeHidden();
    expect(await settled()).toBeCloseTo(closed, 0);
  });

  test("a read-only junction keeps its scenarios in the head and has no history", async ({ page }) => {
    const href = await openJunction(page);
    await page.goto(`/${href}${href.includes("?") ? "&" : "?"}view=1`);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator(".stage-head #scenario-slot")).toHaveCount(1);
    await expect(page.locator("#undo")).toHaveCount(0);
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

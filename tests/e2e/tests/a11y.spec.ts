import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";

import {
  expect,
  openJunction,
  openLongStreet,
  openStreet,
  openPedestrianStreet,
  openStreetWithUnfitTemplate,
  makeStretches,
  streetLength,
  test,
} from "./fixtures";

// Keyboard and small-window behaviour that the audit (WCAG 2.2 AA) found wanting.

for (const [name, open] of [
  ["street", openStreet],
  ["junction", openJunction],
] as const) {
  test(`the ${name} editor's first stop is a skip link that shows when focused and leads to the drawing`, async ({
    page,
  }) => {
    await open(page);
    await expect(page.locator("#wrap")).toBeVisible();
    const skip = page.locator(".skip-link");
    await expect(skip).toHaveText("Skip to the drawing");
    expect((await skip.boundingBox())!.y + (await skip.boundingBox())!.height).toBeLessThanOrEqual(0);
    await page.keyboard.press("Tab");
    await expect(skip).toBeFocused();
    expect((await skip.boundingBox())!.y).toBeGreaterThanOrEqual(0);
    await page.keyboard.press("Enter");
    await expect(page.locator("#wrap")).toBeFocused();
  });
}

test("at 400% zoom the sticky bar does not cover the control the keyboard has reached", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 256 });
  await openStreet(page);
  await expect(page.locator("#wrap")).toBeVisible();
  // The page keeps how far the bar covers the window, and scrolls controls clear of it.
  const covered = await page.evaluate(() => parseFloat(document.documentElement.style.getPropertyValue("--bar-stuck")));
  const bar = (await page.locator(".bar").boundingBox())!;
  expect(covered).toBeGreaterThanOrEqual(bar.y + bar.height);
  for (const id of ["#add-btn", "#plan-point", "#wrap"]) {
    await page.locator(id).focus();
    const box = (await page.locator(id).boundingBox())!;
    const barNow = (await page.locator(".bar").boundingBox())!;
    // Not hidden by the bar (WCAG 2.4.11): a part of it, or all of it when it is small, is clear of the bar.
    const clear = Math.min(box.y + box.height, 256) - Math.max(box.y, barNow.y + barNow.height);
    expect(clear, `${id} is clear of the bar`).toBeGreaterThanOrEqual(Math.min(box.height, 24) - 1);
  }
});

test("under 380 px the end buttons do not run over the logo", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await openStreet(page);
  const logo = (await page.locator(".brand").boundingBox())!;
  for (const id of ["#share-btn", "#help-btn", "#account-btn"]) {
    const b = (await page.locator(id).boundingBox())!;
    expect(b.y, id).toBeGreaterThanOrEqual(logo.y + logo.height - 1);
  }
});

test("at 320 px a street too long for the column at the plan's smallest scale scrolls inside the plan, not the page", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 12 * 5);
  // Twelve stretches of 5 m at least: at the smallest scale each is 24 px wide, more than the column holds.
  await makeStretches(page, 12);
  const length = await streetLength(page);
  const m = await page.evaluate(() => {
    const plan = document.getElementById("plan")!;
    return {
      page: document.documentElement.scrollWidth,
      inner: innerWidth,
      scroll: plan.scrollWidth,
      shown: plan.clientWidth,
      drawn: plan.querySelector("svg")!.getBoundingClientRect().width,
    };
  });
  expect(m.page).toBeLessThanOrEqual(m.inner);
  expect(m.drawn, "no smaller than 24 px for 5 m").toBeGreaterThanOrEqual(length * 4.8 - 1);
  expect(m.scroll).toBeGreaterThan(m.shown);
});

test("at 320 px the plan's point is kept in sight inside the plan as the keys move it", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 12 * 5);
  const inside = () =>
    page.evaluate(() => {
      const plan = document.getElementById("plan")!.getBoundingClientRect();
      const point = document.getElementById("plan-point")!.getBoundingClientRect();
      const middle = point.left + point.width / 2;
      return middle >= plan.left - 1 && middle <= plan.right + 1;
    });
  const scrolled = () => page.evaluate(() => document.getElementById("plan")!.scrollLeft);
  await page.locator("#plan-point").focus();
  await page.keyboard.press("Home");
  await expect.poll(inside).toBe(true);
  await page.keyboard.press("End");
  await expect.poll(inside).toBe(true);
  expect(await scrolled(), "the plan scrolled to show its right end").toBeGreaterThan(0);
  await page.keyboard.press("Home");
  await expect.poll(scrolled).toBe(0);
});

test("at 320 px a boundary dragged on the plan is dropped where the pointer was released", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 60);
  await makeStretches(page, 2);
  const line = page.locator("#plan .plan-boundary-line");
  await page.locator("#plan").scrollIntoViewIfNeeded();
  const before = (await line.boundingBox())!;
  const x = before.x + before.width / 2;
  const y = before.y + before.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 30, y, { steps: 6 });
  await page.mouse.up();
  const after = (await line.boundingBox())!;
  const centre = after.x + after.width / 2;
  expect(Math.abs(centre - (x + 30)), `the boundary ends at ${centre}, the pointer at ${x + 30}`).toBeLessThanOrEqual(
    3,
  );
});

test("the plan's point is one tab stop under the drawing and is named", async ({ page }) => {
  await openStreet(page);
  const point = page.locator("#plan-point");
  await expect(page.locator("#plan")).toBeVisible();
  expect(((await page.locator("#plan").getAttribute("aria-label")) ?? "").trim().length).toBeGreaterThan(0);
  await expect(point).toHaveAttribute("aria-label", "Point along the street");
  // In the page's order: the title card, then the drawing, then the plan under it.
  const order = await page.evaluate(() => {
    const before = (a: Element | null, b: Element | null) =>
      !!a && !!b && !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    const card = document.querySelector(".title-card");
    const plan = document.getElementById("plan");
    const wrap = document.getElementById("wrap");
    return { cardFirst: before(card, plan), drawingFirst: before(wrap, plan) };
  });
  expect(order).toEqual({ cardFirst: true, drawingFirst: true });
  // Tabbing from the skip link reaches the drawing, then the plan's point once, and nothing else of the plan.
  const seen: string[] = [];
  await page.keyboard.press("Tab");
  await expect(page.locator(".skip-link")).toBeFocused();
  for (let i = 0; i < 60; i++) {
    await page.keyboard.press("Tab");
    const at = await page.evaluate(() => {
      const a = document.activeElement!;
      return a.closest("#plan") && a.id !== "plan-point" ? "inside-plan" : a.id || a.className || a.tagName;
    });
    seen.push(String(at));
    if (at === "plan-point") break;
  }
  expect(seen.at(-1), "the point is reached").toBe("plan-point");
  expect(seen.indexOf("wrap"), "the drawing comes first").toBeGreaterThanOrEqual(0);
  expect(seen.filter((id) => id === "plan-point")).toHaveLength(1);
  expect(seen.filter((id) => id === "inside-plan")).toHaveLength(0);
});

test("Alt and the arrows on the plan's point do not leave the page", async ({ page }) => {
  await openStreet(page);
  await makeStretches(page, 2);
  await page.locator("#plan-point").focus();
  const url = page.url();
  await page.keyboard.press("Alt+ArrowLeft");
  await page.keyboard.press("Alt+ArrowRight");
  await page.keyboard.press("Alt+ArrowLeft");
  await page.waitForTimeout(300);
  expect(page.url(), "Alt+Arrow is not the browser's Back or Forward").toBe(url);
  await expect(page.locator("#plan")).toBeVisible();
});

test("the arrow keys still work after the plan's point had focus, and V changes nothing", async ({ page }) => {
  await openStreet(page);
  await makeStretches(page, 2);
  await page.locator("#plan-point").focus();
  await page.keyboard.press("ArrowRight");
  // back to the drawing: the 2.5D is the only view, so V does nothing, and the arrows choose a piece
  await page.locator("#wrap").focus();
  await page.keyboard.press("v");
  await expect(page.locator("#drawing.iso")).toBeAttached();
  const before = await page.locator("#live").textContent();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#live")).not.toHaveText(before ?? "");
  await expect(page.locator("#wrap")).toBeFocused();
});

// The street page's single drawing (#139): what the dragging of the section offered is reachable by keyboard, and the
// controls that stand for a press on a piece are large enough for a finger.
test.describe("the street's 2.5D drawing", () => {
  const cells = (page: Page) => page.locator("#piece-list [role=option]");
  const kinds = (page: Page) => cells(page).locator(".piece-name").allInnerTexts();
  const widths = (page: Page) => cells(page).locator(".piece-width").allInnerTexts();

  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("every piece's cell and the buttons of the page are at least 24 by 24 px (2.5.8), also at 320 px", async ({
    page,
  }) => {
    for (const width of [1440, 320]) {
      await page.setViewportSize({ width, height: 900 });
      const n = await cells(page).count();
      expect(n).toBeGreaterThan(0);
      for (const target of [...(await cells(page).all()), page.locator("#undo"), page.locator("#add-btn")]) {
        const b = (await target.boundingBox())!;
        expect(b.width, `width at ${width}`).toBeGreaterThanOrEqual(24);
        expect(b.height, `height at ${width}`).toBeGreaterThanOrEqual(24);
      }
    }
  });

  test("width, order, removal and addition are all done with the keyboard alone", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    // Width: plus and minus, with Shift for bigger steps, and Enter goes to the field.
    const width = async () => parseFloat((await widths(page))[0].replace(",", "."));
    const w0 = await width();
    await page.keyboard.press("+");
    await expect.poll(width).toBeCloseTo(w0 + 0.1, 1);
    await page.keyboard.press("Shift+=");
    await expect.poll(width).toBeCloseTo(w0 + 0.6, 1);
    await page.keyboard.press("Shift+-");
    await expect.poll(width).toBeCloseTo(w0 + 0.1, 1);
    await page.keyboard.press("Enter");
    await expect(page.locator("#width")).toBeFocused();
    await page.locator("#width").fill(String(w0 + 0.2));
    await page.locator("#width").press("Enter");
    await expect.poll(width).toBeCloseTo(w0 + 0.2, 1);
    await page.locator("#wrap").focus();
    // Order: Shift with an arrow moves the selected piece along.
    const order = await kinds(page);
    await page.keyboard.press("Shift+ArrowRight");
    await expect.poll(() => kinds(page)).not.toEqual(order);
    expect((await kinds(page)).slice().sort()).toEqual(order.slice().sort());
    await page.keyboard.press("Shift+ArrowLeft");
    await expect.poll(() => kinds(page)).toEqual(order);
    // Removal.
    await page.keyboard.press("Delete");
    await expect(cells(page)).toHaveCount(order.length - 1);
    // Addition, from the menu, with the keys.
    await page.locator("#add-btn").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#add-menu")).toBeVisible();
    await expect(page.locator("#add-search")).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("#add-panel .add-item").first()).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(cells(page)).toHaveCount(order.length);
    // And undo takes it back.
    await page.keyboard.press("Control+z");
    await expect(cells(page)).toHaveCount(order.length - 1);
  });

  test("the drawing keeps its screen-reader label and its list is the way to a piece the picture hides", async ({
    page,
  }) => {
    const drawing = page.locator("#drawing");
    await expect(drawing).toHaveAttribute("role", "img");
    await expect(drawing).toHaveAttribute(
      "aria-label",
      /in 2\.5D, along a 40 m stretch\. \d+ segments?\.( Speed limit \d+ km\/h\.)?$/,
    );
    // The press targets are part of the picture, not of what is read: their titles are the pieces' names.
    await expect(page.locator("#wrap")).toHaveAttribute("aria-describedby", "keys");
  });

  test("at 280 px the page does not scroll sideways, the drawing fits and a piece is chosen from the list", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await expect(page.locator("#drawing")).toBeVisible();
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    // The drawing is scaled down inside the window, not cut off at its right edge.
    await expect
      .poll(async () => {
        const b = (await page.locator("#drawing").boundingBox())!;
        return b.x >= -0.5 && b.x + b.width <= 280.5;
      })
      .toBe(true);
    const last = cells(page).last();
    await last.scrollIntoViewIfNeeded();
    await last.click();
    await expect(last).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
  });
});

test("the add menu open on Transit, its stops greyed, passes an axe scan", async ({ page }) => {
  await openPedestrianStreet(page);
  await page.locator("#add-btn").click();
  await page.locator("#add-topic-transit").click();
  await expect(page.locator('#add-panel .add-item[aria-disabled="true"]')).toHaveCount(3);
  await expect(page.locator("#add-reason-stops")).toBeVisible();
  const scan = await new AxeBuilder({ page })
    .include("#add-menu")
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(scan.passes.length, "the scan reached the menu").toBeGreaterThan(0);
  expect(scan.violations).toEqual([]);
});

for (const scheme of ["light", "dark"] as const) {
  test(`the street's Transform dialog, open on a previewed choice, has no axe violations in ${scheme}`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator("#transform-btn").click();
    await expect(page.locator("#transform-dialog")).toBeVisible();
    await page.locator("#transform-dialog .transform-item").first().click();
    await expect(page.locator("#transform-preview")).toBeVisible();
    await expectNoDialogViolations(page);
  });

  test(`the street's Transform dialog, open on a previewed transit measure, has no axe violations in ${scheme}`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-transit").click();
    await expect(page.locator("#transform-dialog .transform-family").first()).toBeVisible();
    await page.locator("#transform-dialog .transit-item").first().click();
    await expect(page.locator("#transform-preview-h")).toContainText(/^[A-F]\d · /);
    await expectNoDialogViolations(page);
  });

  test(`the street's Transform dialog, open on its templates with one refused and School previewed, has no axe violations in ${scheme}`, async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await page.emulateMedia({ colorScheme: scheme });
    // A street of the fixture too narrow (or too wide) for some template: the dialog is left open on the templates.
    await openStreetWithUnfitTemplate(page);
    await expect(
      page.locator('#transform-dialog [role=option][aria-disabled="true"] .template-reason').first(),
    ).toBeVisible();
    await page.locator('#transform-dialog [data-template="school"]').click({ force: true });
    await expect(page.locator("#transform-preview-h")).toHaveText("School street");
    await expectNoDialogViolations(page);
  });
}

/** Runs axe (WCAG 2.2 AA, every rule) on the open Transform dialog alone and expects nothing found. */
async function expectNoDialogViolations(page: Page): Promise<void> {
  const found = await new AxeBuilder({ page })
    .include("#transform-dialog")
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(found.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
}

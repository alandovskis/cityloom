import type { Page } from "@playwright/test";

import { expect, openJunction, openLongStreet, openStreet, splitInto, test } from "./fixtures";

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
  for (const id of ["#add-btn", "#zone-strip", "#wrap"]) {
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

test("at 320 px a strip of many zones scrolls inside itself and the page does not scroll sideways", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openStreet(page);
  await expect(page.locator("#zone-strip")).toBeVisible();
  // Thirty zones' cells (the same markup the strip draws), each with its 24 px minimum.
  await page.evaluate(() => {
    const strip = document.getElementById("zone-strip")!;
    const cell = strip.querySelector(".zone-cell")!;
    for (let i = 0; i < 30; i++) strip.append(cell.cloneNode(true));
  });
  const m = await page.evaluate(() => ({
    page: document.documentElement.scrollWidth,
    inner: innerWidth,
    strip: document.getElementById("zone-strip")!.scrollWidth,
    shown: document.getElementById("zone-strip")!.clientWidth,
  }));
  expect(m.page).toBeLessThanOrEqual(m.inner);
  expect(m.strip).toBeGreaterThan(m.shown);
});

test("at 320 px twelve real zones keep 24 px cells and handles, and the page does not scroll sideways", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 12 * 5);
  await splitInto(page, 12);
  const m = await page.evaluate(() => {
    const strip = document.getElementById("zone-strip")!;
    const w = (sel: string) => [...strip.querySelectorAll(sel)].map((e) => e.getBoundingClientRect().width);
    return {
      page: document.documentElement.scrollWidth,
      inner: innerWidth,
      strip: strip.scrollWidth,
      shown: strip.clientWidth,
      cells: w(".zone-cell"),
      handles: w(".zone-handle"),
    };
  });
  expect(m.page).toBeLessThanOrEqual(m.inner);
  expect(m.strip).toBeGreaterThan(m.shown);
  expect(m.cells).toHaveLength(12);
  expect(m.handles).toHaveLength(11);
  for (const w of [...m.cells, ...m.handles]) expect(w).toBeGreaterThanOrEqual(24);
});

test("at 320 px the zone chosen by the keys is scrolled into view inside the strip", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 12 * 5);
  await splitInto(page, 12);
  const inside = (id: string) =>
    page.evaluate((id) => {
      const strip = document.getElementById("zone-strip")!.getBoundingClientRect();
      const cell = document.getElementById(id)!.getBoundingClientRect();
      return cell.left >= strip.left - 1 && cell.right <= strip.right + 1;
    }, id);
  await page.locator("#zone-strip").focus();
  await page.keyboard.press("Home");
  await expect.poll(() => inside("zone-0")).toBe(true);
  await page.keyboard.press("End");
  await expect.poll(() => inside("zone-11")).toBe(true);
  expect(await inside("zone-0"), "the strip scrolled; the first cell left its box").toBe(false);
  await page.keyboard.press("ArrowLeft");
  await expect.poll(() => inside("zone-10")).toBe(true);
});

test("at 320 px a boundary dragged between a floored cell and the rest is dropped where the pointer was released", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await openLongStreet(page, 60);
  // a 5 m first zone: its share of the strip is under the cell's 24 px minimum, so the cell is floored
  await page.locator("#zone-split-at").fill("5");
  await page.locator("#zone-split").click();
  await expect(page.locator('#zone-strip [role="option"]')).toHaveCount(2);
  const handle = page.locator(".zone-handle");
  // the commands are in the left panel, below the strip on a phone: bring the strip back into the window
  await page.locator("#zone-strip").scrollIntoViewIfNeeded();
  const before = (await handle.boundingBox())!;
  const x = before.x + before.width / 2;
  const y = before.y + before.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 60, y, { steps: 6 });
  await page.mouse.up();
  const after = (await handle.boundingBox())!;
  const centre = after.x + after.width / 2;
  expect(Math.abs(centre - (x + 60)), `the handle ends at ${centre}, the pointer at ${x + 60}`).toBeLessThanOrEqual(3);
});

test("the zone strip is one tab stop under the drawing and is named", async ({ page }) => {
  await openStreet(page);
  await expect(page.locator("#zone-strip")).toBeVisible();
  await expect(page.locator("#zone-strip")).not.toHaveAttribute("aria-label", "");
  expect(((await page.locator("#zone-strip").getAttribute("aria-label")) ?? "").trim().length).toBeGreaterThan(0);
  // In the page's order: the title card, then the drawing, then the strip under it.
  const order = await page.evaluate(() => {
    const before = (a: Element | null, b: Element | null) =>
      !!a && !!b && !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    const card = document.querySelector(".title-card");
    const strip = document.getElementById("zone-strip");
    const wrap = document.getElementById("wrap");
    return { cardFirst: before(card, strip), stripFirst: before(wrap, strip) };
  });
  expect(order).toEqual({ cardFirst: true, stripFirst: true });
  // Tabbing from the skip link reaches the drawing, then the strip once, and no cell of it.
  const seen: string[] = [];
  await page.keyboard.press("Tab");
  await expect(page.locator(".skip-link")).toBeFocused();
  for (let i = 0; i < 60; i++) {
    await page.keyboard.press("Tab");
    const at = await page.evaluate(() => {
      const a = document.activeElement!;
      return a.id || a.className || a.tagName;
    });
    seen.push(String(at));
    if (at === "zone-strip") break;
  }
  expect(seen.at(-1), "the strip is reached").toBe("zone-strip");
  expect(seen.indexOf("wrap"), "the drawing comes first").toBeGreaterThanOrEqual(0);
  expect(seen.filter((id) => id === "zone-strip")).toHaveLength(1);
  expect(seen.filter((id) => /^zone-\d+$/.test(id))).toHaveLength(0);
  expect(seen.indexOf("zone-strip")).toBeGreaterThanOrEqual(0);
});

test("Alt and the arrows on the strip do not leave the page", async ({ page }) => {
  await openStreet(page);
  await page.locator("#zone-split").click();
  await expect(page.locator('#zone-strip [role="option"]')).toHaveCount(2);
  await page.locator("#zone-strip").focus();
  const url = page.url();
  await page.keyboard.press("Alt+ArrowLeft");
  await page.keyboard.press("Alt+ArrowRight");
  await page.keyboard.press("Alt+ArrowLeft");
  await page.waitForTimeout(300);
  expect(page.url(), "Alt+Arrow is not the browser's Back or Forward").toBe(url);
  await expect(page.locator("#zone-strip")).toBeVisible();
});

test("the arrow keys still work after the strip had focus, and V changes nothing", async ({ page }) => {
  await openStreet(page);
  await page.locator("#zone-split").click();
  await expect(page.locator('#zone-strip [role="option"]')).toHaveCount(2);
  await page.locator("#zone-strip").focus();
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
    await page.keyboard.press("ArrowDown");
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

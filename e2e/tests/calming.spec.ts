import type { Page } from "@playwright/test";

import { expect, placesOnMap, streetLengthM, test } from "./fixtures";

// Raised tables and crossings: the strip under the zones, its tools, the checks, the 2.5D and the saved document.

/** Opens the first street of the city that has a raised-tables strip (a street with a real end to measure from) and is
 *  at least `minM` metres long. With `stops`, it must also have a transit stop row and a curbside strip. */
async function openCalmingStreet(page: Page, minM: number, stops = false) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#zone-0")).toBeVisible();
    if ((await page.locator("#calming-strip").count()) === 0) continue;
    if (
      stops &&
      ((await page.locator("#stop-strip-right").count()) === 0 || (await page.locator(".curb-strip").count()) === 0)
    )
      continue;
    if ((await streetLengthM(page)) >= minM) return;
  }
  throw new Error(`no street of ${minM} m or more with a raised-tables strip${stops ? " and a stops row" : ""}`);
}

const cells = (page: Page) => page.locator("#calming-strip").locator('[role="option"]');

/** Chooses the free stretch `cell` and adds an element of `kind` (the Kind select's value) and `metres` there. */
async function addElement(page: Page, cell: number, metres: number, kind = "0") {
  await page.locator("#calming-" + cell).click();
  await page.locator("#calming-kind").selectOption(kind);
  await page.locator("#calming-length").fill(String(metres));
  await page.locator("#calming-add").click();
}

const editsVersion = (page: Page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? "{}").version);

test.describe("raised tables and crossings", () => {
  test("a table is added with the tools, drawn, kept over a reload and undone", async ({ page }) => {
    await openCalmingStreet(page, 20);
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#undo")).toBeDisabled();
    const before = await editsVersion(page);

    await addElement(page, 0, 8);
    await expect(cells(page)).toHaveCount(2);
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Raised table.*8\.0 m long/);
    await expect(page.locator("#undo")).toBeEnabled();

    // The 2.5D draws it, names it and says how many it shows; nothing in it is pressed to select.
    await expect(page.locator("#drawing .iso-table")).toHaveCount(1);
    await expect(page.locator("#drawing .iso-table .iso-ramp").first()).toBeAttached();
    await expect(page.locator("#drawing .iso-table [data-uid]")).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /One raised table is shown\./);

    // The element's checks are listed, and a plain table on a street of any speed fits or says why not.
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").filter({ hasText: "Raised elements long enough" })).toHaveCount(1);

    // Kept: the document takes the raised tables' version, and the table is there after a reload.
    expect(before).not.toBe(12);
    await expect.poll(() => editsVersion(page)).toBe(12);
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Raised table.*8\.0 m long/);

    // Undo takes it away, and the saved document goes back to what it was.
    await page.locator("#undo").click();
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#drawing .iso-table")).toHaveCount(0);
    await page.locator("#redo").click();
    await expect(cells(page)).toHaveCount(2);
  });

  test("a crossing paints a zebra and a table does not", async ({ page }) => {
    await openCalmingStreet(page, 20);
    await addElement(page, 0, 8, "1");
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Raised crossing/);
    await expect(page.locator("#drawing .iso-table .iso-zebra").first()).toBeAttached();
    await page.locator("#calming-0").click();
    await page.locator("#calming-kind").selectOption("0");
    await page.locator("#calming-length").fill("8");
    await page.locator("#calming-length").press("Enter");
    await expect(page.locator("#drawing .iso-zebra")).toHaveCount(0);
  });

  test("a table that is too short or runs past the street is refused, and the refusal says why", async ({ page }) => {
    await openCalmingStreet(page, 20);
    await addElement(page, 0, 4);
    await expect(page.locator("#live")).toContainText("This element must be at least");
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#undo")).toBeDisabled();
    await page.locator("#calming-0").click();
    await page.locator("#calming-length").fill("500");
    await page.locator("#calming-add").click();
    await expect(page.locator("#live")).toContainText("That would run past an end of the street.");
    await expect(cells(page)).toHaveCount(1);
  });

  test("the arrows choose a cell and Alt with them moves the table by a metre; Remove takes it away", async ({
    page,
  }) => {
    await openCalmingStreet(page, 20);
    await addElement(page, 0, 8);
    const strip = page.locator("#calming-strip");
    await cells(page).nth(0).click();
    const name = async () => (await cells(page).nth(0).getAttribute("aria-label")) ?? "";
    const first = await name();
    await strip.press("Alt+ArrowRight");
    await expect.poll(name).not.toBe(first);
    await page.locator("#calming-remove").click();
    await expect(cells(page)).toHaveCount(1);
  });

  test("a table over a transit stop fails its check, and moving it clear passes", async ({ page }) => {
    await openCalmingStreet(page, 40, true);
    // A stop at the start of the right side, then a table at the start of the street.
    await page.locator("#stop-right-0").click();
    await page.locator("#add-btn").click();
    await page.locator(".stop-add-item", { hasText: "Curb extension" }).click();
    await page.locator("#stop-right-0").click();
    await page.locator("#stop-length").fill("12");
    await page.locator("#stop-length").press("Enter");
    await addElement(page, 0, 8);
    await page.locator("#t-checks").click();
    const row = page.locator("#checks li").filter({ hasText: "Raised elements clear of transit stops" });
    await expect(row).toHaveCount(1);
    await expect(row).not.toHaveClass("ok");
    await expect(row).toContainText("Stop concerned");
  });

  test("the strip and the tools follow the language", async ({ page }) => {
    await openCalmingStreet(page, 20);
    await addElement(page, 0, 8);
    await page.evaluate(() => localStorage.setItem("cityloom-lang", "fr"));
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(page.locator("#calming-add")).toHaveText("Ajouter un élément surélevé");
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Plateau surélevé/);
  });
  test("a chicane is added with the tools, drawn as build-outs, saved as version 14 and undone", async ({ page }) => {
    await openCalmingStreet(page, 30);
    await page.locator("#calming-0").click();
    await page.locator("#calming-kind").selectOption("2");
    // Only a chicane has a side to start on.
    await expect(page.locator("#calming-first")).toBeVisible();
    await page.locator("#calming-first").selectOption("left");
    await page.locator("#calming-length").fill("24");
    await page.locator("#calming-add").click();
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Chicane.*24\.0 m long/);

    // The 2.5D draws build-outs (not ramps), names them, and counts the chicane apart from the tables.
    await expect(page.locator("#drawing .iso-chicane").first()).toBeAttached();
    await expect(page.locator("#drawing .iso-chicane [data-uid]")).toHaveCount(0);
    await expect(page.locator("#drawing .iso-table")).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /chicane/);
    await page.screenshot({ path: "test-results/chicane.png", fullPage: true });

    // Its checks include the lane row, and the document takes the chicanes' version only while one is held.
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").filter({ hasText: "Chicanes clear of bus and bike lanes" })).toHaveCount(1);
    await expect.poll(() => editsVersion(page)).toBe(14);
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Chicane/);

    // Undo takes it away, and the saved document goes back to what it was.
    await page.locator("#undo").click();
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#drawing .iso-chicane")).toHaveCount(0);
    await expect.poll(() => editsVersion(page)).not.toBe(14);
  });

  test("a chicane that is too short is refused with its minimum", async ({ page }) => {
    await openCalmingStreet(page, 30);
    await addElement(page, 0, 10, "2");
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#drawing .iso-chicane")).toHaveCount(0);
  });
  test("a modal filter is added with the tools, drawn as one block, saved as version 15 and undone", async ({
    page,
  }) => {
    await openCalmingStreet(page, 30);
    await page.locator("#calming-0").click();
    await page.locator("#calming-kind").selectOption("3");
    // Only a chicane has a side to start on.
    await expect(page.locator("#calming-first")).toHaveCount(0);
    await page.locator("#calming-length").fill("2");
    await page.locator("#calming-add").click();
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Modal filter.*2\.0 m long/);

    // The 2.5D draws one block, names it, and counts it apart from the tables and chicanes.
    await expect(page.locator("#drawing .iso-filter").first()).toBeAttached();
    await expect(page.locator("#drawing .iso-filter [data-uid]")).toHaveCount(0);
    await expect(page.locator("#drawing .iso-table, #drawing .iso-chicane")).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /modal filter/);
    await page.screenshot({ path: "test-results/modal-filter.png", fullPage: true });

    // Its checks include the bus-lane and passage rows, and the document takes version 15 only while one is held.
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").filter({ hasText: "Modal filters clear of bus lanes" })).toHaveCount(1);
    await expect(
      page.locator("#checks li").filter({ hasText: "Modal filters passable on foot and by bike" }),
    ).toHaveCount(1);
    await expect.poll(() => editsVersion(page)).toBe(15);
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(cells(page).nth(0)).toHaveAttribute("aria-label", /Modal filter/);

    await page.locator("#undo").click();
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#drawing .iso-filter")).toHaveCount(0);
    await expect.poll(() => editsVersion(page)).not.toBe(15);
  });

  test("a modal filter shorter than its minimum is refused", async ({ page }) => {
    await openCalmingStreet(page, 30);
    await addElement(page, 0, 0.5, "3");
    await expect(cells(page)).toHaveCount(1);
    await expect(page.locator("#drawing .iso-filter")).toHaveCount(0);
  });
});

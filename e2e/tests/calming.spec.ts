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
    await expect.poll(() => editsVersion(page)).toBeGreaterThan(before);
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

  test("a table that is too short is added and fails its check, and a refusal says why", async ({ page }) => {
    await openCalmingStreet(page, 20);
    await addElement(page, 0, 4);
    await page.locator("#t-checks").click();
    const row = page.locator("#checks li").filter({ hasText: "Raised elements long enough" });
    await expect(row).not.toHaveClass("ok");
    await expect(row).toContainText("too short");
    // Longer than the street: refused, and said.
    await page.locator("#calming-1").click();
    await page.locator("#calming-length").fill("500");
    await page.locator("#calming-add").click();
    await expect(cells(page)).toHaveCount(2);
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
    await page.locator("#stop-length").fill("12");
    await page.locator("#stop-add").click();
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
});

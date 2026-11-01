import type { Page } from "@playwright/test";

import { expect, placesOnMap, streetLengthM, test } from "./fixtures";

// The curbside uses: the strip under the zones, the checks, the 2.5D and the saved document.

const USE = { tree: "2", planting: "3", patio: "5", bikeParking: "6" };

/** Opens the first street of the city with a curbside lane (a strip) that is at least `minM` metres long. */
async function openCurbsideStreet(page: Page, minM: number) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#zone-0")).toBeVisible();
    if ((await page.locator(".curb-strip").count()) > 0 && (await streetLengthM(page)) >= minM) return;
  }
  throw new Error(`no street of ${minM} m or more with a curbside lane`);
}

/** Adds a use of `metres` into the first free stretch of the first strip. */
async function addUse(page: Page, cell: number, use: string, metres: number) {
  const strip = page.locator(".curb-strip").first();
  await strip.locator('[role="option"]').nth(cell).click();
  await page.locator("#curb-use").selectOption(use);
  await page.locator("#curb-length").fill(String(metres));
  await page.locator("#curb-add").click();
}

const editsVersion = (page: Page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? "{}").version);

test.describe("the curbside strip", () => {
  test("the Use choice follows the chosen cell, also after it was changed by hand", async ({ page }) => {
    await openCurbsideStreet(page, 10);
    const strip = page.locator(".curb-strip").first();
    const use = page.locator("#curb-use");
    const cells = strip.locator('[role="option"]');
    await expect(cells).toHaveCount(1);

    // Two uses side by side: a tree (value 2), then bike parking (value 6) in the gap that is left.
    await cells.nth(0).click();
    await use.selectOption("2");
    await page.locator("#curb-length").fill("2");
    await page.locator("#curb-add").click();
    await expect(cells).toHaveCount(2);
    await cells.nth(1).click();
    await use.selectOption("6");
    await page.locator("#curb-length").fill("2");
    await page.locator("#curb-add").click();
    await expect(cells).toHaveCount(3);

    // Change the first use's kind by hand, then look at the other and come back.
    await cells.nth(0).click();
    await expect(use).toHaveValue("2");
    await use.selectOption("3");
    await expect(cells.nth(0)).toHaveAttribute("title", "Planting");
    await cells.nth(1).click();
    await expect(use).toHaveValue("6");
    await cells.nth(0).click();
    await expect(use).toHaveValue("3");
  });

  test("a patio is added, checked, drawn raised, kept over a reload and undone", async ({ page }) => {
    await openCurbsideStreet(page, 10);
    const cells = page.locator(".curb-strip").first().locator('[role="option"]');
    await expect(page.locator("#undo")).toBeDisabled();

    await addUse(page, 0, USE.patio, 6);
    await expect(cells).toHaveCount(2);
    await expect(cells.nth(0)).toHaveAttribute("title", "Patio");
    await expect(cells.nth(0)).toHaveAttribute("aria-label", /^Patio, .*6\.0 m long/);
    await expect(page.locator("#undo")).toBeEnabled();

    // The Checks list the five curbside rows.
    await page.locator("#t-checks").click();
    for (const label of [
      "Curbside uses are long enough",
      "Curbside uses have a lane",
      "Parking clear of crossings",
      "Curbside uses clear of bike lanes",
      "Patios leave room on the sidewalk",
    ]) {
      await expect(page.locator("#checks li").filter({ hasText: label })).toHaveCount(1);
    }

    // The 2.5D says how many uses it shows and draws the patio as a raised strip of its own.
    await expect(page.locator("#drawing.iso")).toBeAttached();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /One curbside use is shown\./);
    await expect(page.locator('#drawing .iso-piece:has(> title:text-is("Patio"))').first()).toBeAttached();

    // Kept: the document is version 8 and the use is still there after a reload.
    await expect.poll(() => editsVersion(page)).toBe(8);
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(page.locator(".curb-strip").first().locator('[role="option"]').nth(0)).toHaveAttribute(
      "title",
      "Patio",
    );

    // Undo takes it away.
    await page.locator("#undo").click();
    await expect(page.locator(".curb-strip").first().locator('[role="option"]')).toHaveCount(1);
  });

  test("the arrows move between cells and Alt with them moves the use by a metre", async ({ page }) => {
    await openCurbsideStreet(page, 10);
    const strip = page.locator(".curb-strip").first();
    const cells = strip.locator('[role="option"]');
    await addUse(page, 0, USE.tree, 2);
    await expect(cells).toHaveCount(2);

    await strip.focus();
    await cells.nth(0).click();
    await strip.focus();
    await expect(strip).toHaveAttribute("aria-activedescendant", /-0$/);
    await page.keyboard.press("ArrowRight");
    await expect(strip).toHaveAttribute("aria-activedescendant", /-1$/);
    await page.keyboard.press("ArrowLeft");
    await expect(strip).toHaveAttribute("aria-activedescendant", /-0$/);

    const before = await cells.nth(0).getAttribute("aria-label");
    await page.keyboard.press("Alt+ArrowRight");
    // The use now has a gap before it: the cells are gap, use, gap.
    await expect(cells).toHaveCount(3);
    const moved = await cells.nth(1).getAttribute("aria-label");
    expect(moved).toMatch(/^Street tree, 1\.0 m to 3\.0 m/);
    expect(before).toMatch(/^Street tree, 0\.0 m to 2\.0 m/);
  });

  test("a use is dragged along the street as one undoable step, and Escape cancels a drag", async ({ page }) => {
    await openCurbsideStreet(page, 20);
    const strip = page.locator(".curb-strip").first();
    const cells = strip.locator('[role="option"]');
    await addUse(page, 0, USE.patio, 6);
    await expect(cells).toHaveCount(2);
    const used = () => strip.locator('[role="option"][title="Patio"]');
    const where = async () => await used().getAttribute("aria-label");
    const before = await where();

    const box = (await used().boundingBox())!;
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];

    // Escape during a drag puts the use back.
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 40, y, { steps: 6 });
    await expect.poll(where).not.toBe(before);
    await page.keyboard.press("Escape");
    await page.mouse.up();
    await expect.poll(where).toBe(before);

    // A drag to the right moves it and is one step.
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 20, y, { steps: 4 });
    await page.mouse.move(x + 60, y, { steps: 6 });
    await page.mouse.up();
    const dragged = await where();
    expect(dragged).not.toBe(before);
    await page.locator("#undo").click();
    expect(await where()).toBe(before);
    await expect(page.locator("#undo")).toBeEnabled(); // the add is still there to undo
    await expect(used()).toHaveCount(1);
  });
});

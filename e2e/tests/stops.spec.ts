import type { Page } from "@playwright/test";

import { expect, placesOnMap, streetLengthM, test } from "./fixtures";

// The transit stops: the Stops rows under the zones, the checks, the 2.5D and the saved document.

const ROWS = [
  "Stops have a lane for the bus",
  "Stops are built as their kind needs",
  "Stops are long enough for a bus",
  "Stops have room to board",
  "Stops clear of bike lanes",
  "Shelters have room",
];

/** Opens the first street of the city that has a Stops row on its right and is at least `minM` metres long. With
 *  `curbside`, it must also have a curbside strip (a parking lane: the default stop there is a curb extension, which
 *  has the room to board; on the Plateau's 1.5 m sidewalks a curb stop fails the width check). */
async function openStopStreet(page: Page, minM: number, curbside = true) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#zone-0")).toBeVisible();
    if ((await page.locator("#stop-strip-right").count()) === 0) continue;
    if (curbside && (await page.locator(".curb-strip").count()) === 0) continue;
    if ((await streetLengthM(page)) >= minM) return;
  }
  throw new Error(`no street of ${minM} m or more with a Stops row${curbside ? " and a curbside strip" : ""}`);
}

/** Chooses cell `cell` of the right Stops row and adds a stop of `metres` there. */
async function addStop(page: Page, cell: number, metres: number, shelter = false) {
  await page.locator("#stop-right-" + cell).click();
  if (shelter) await page.locator("#stop-shelter").check();
  await page.locator("#stop-length").fill(String(metres));
  await page.locator("#stop-add").click();
}

const editsVersion = (page: Page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? "{}").version);

test.describe("the Stops row", () => {
  test("a stop with a shelter is added, checked, drawn, kept over a reload and undone", async ({ page }) => {
    await openStopStreet(page, 20);
    const cells = page.locator("#stop-strip-right").locator('[role="option"]');
    await expect(cells).toHaveCount(1);
    await expect(page.locator("#undo")).toBeDisabled();

    await addStop(page, 0, 12, true);
    await expect(cells).toHaveCount(2);
    await expect(cells.nth(0)).toHaveClass(/stop-cell/);
    await expect(cells.nth(0)).toHaveAttribute("aria-label", /12\.0 m long/);
    await expect(cells.nth(0)).toHaveAttribute("aria-label", /with a shelter/);
    await expect(page.locator("#undo")).toBeEnabled();

    // The Checks list the six stop rows. The curb extension (the parking lane) joins the sidewalk beyond it, which
    // together give the 3.9 m a shelter needs on the Plateau, so all six pass.
    await page.locator("#t-checks").click();
    for (const label of ROWS) {
      const row = page.locator("#checks li").filter({ hasText: label });
      await expect(row).toHaveCount(1);
      await expect(row).toHaveClass("ok");
    }
    await expect(page.locator("#checks li").filter({ hasText: "Shelters have room" })).toContainText("Every shelter has room.");

    // The 2.5D says how many stops it shows, and draws the sign and the shelter.
    await page.locator('#view-mode-slot button[aria-pressed="false"]').click();
    await expect(page.locator("#drawing.iso")).toBeAttached();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /One transit stop is shown\./);
    await expect(page.locator("#drawing .iso-stop-sign").first()).toBeAttached();
    await expect(page.locator("#drawing .iso-solid.s-shelter")).toHaveCount(1);

    // Kept: the document is version 9 and the stop is still there after a reload.
    await expect.poll(() => editsVersion(page)).toBe(9);
    await page.reload();
    await expect(page.locator("#zone-0")).toBeVisible();
    await expect(page.locator("#stop-right-0")).toHaveAttribute("aria-label", /with a shelter.*12\.0 m long/);

    // Undo takes it away.
    await page.locator("#undo").click();
    await expect(page.locator("#stop-strip-right").locator('[role="option"]')).toHaveCount(1);
  });

  test("a stop without a shelter passes all six checks", async ({ page }) => {
    await openStopStreet(page, 20);
    await addStop(page, 0, 12);
    await page.locator("#t-checks").click();
    for (const label of ROWS) {
      await expect(page.locator("#checks li").filter({ hasText: label })).toHaveClass("ok");
    }
  });

  test("a name is kept, and removing the stop takes it away", async ({ page }) => {
    await openStopStreet(page, 20);
    const cells = page.locator("#stop-strip-right").locator('[role="option"]');
    await addStop(page, 0, 12);
    await page.locator("#stop-right-0").click();
    await page.locator("#stop-name").fill("Mont-Royal");
    await page.locator("#stop-name").press("Enter");
    await expect(cells.nth(0)).toHaveAttribute("aria-label", /^Mont-Royal \(curb extension\), /);
    await page.reload();
    await expect(page.locator("#stop-right-0")).toHaveAttribute("aria-label", /^Mont-Royal \(curb extension\), /);
    await page.locator("#stop-right-0").click();
    await page.locator("#stop-remove").click();
    await expect(cells).toHaveCount(1);
  });

  test("the arrows move between cells and Alt with them moves the stop by a metre", async ({ page }) => {
    await openStopStreet(page, 20);
    const strip = page.locator("#stop-strip-right");
    const cells = strip.locator('[role="option"]');
    await addStop(page, 0, 12);
    await expect(cells).toHaveCount(2);

    await cells.nth(0).click();
    await strip.focus();
    await expect(strip).toHaveAttribute("aria-activedescendant", /-0$/);
    await page.keyboard.press("ArrowRight");
    await expect(strip).toHaveAttribute("aria-activedescendant", /-1$/);
    await page.keyboard.press("ArrowLeft");
    await expect(strip).toHaveAttribute("aria-activedescendant", /-0$/);

    const before = await cells.nth(0).getAttribute("aria-label");
    await page.keyboard.press("Alt+ArrowRight");
    // The stop now has a gap before it: the cells are gap, stop, gap.
    await expect(cells).toHaveCount(3);
    const moved = await cells.nth(1).getAttribute("aria-label");
    expect(before).toMatch(/0\.0 m to 12\.0 m/);
    expect(moved).toMatch(/1\.0 m to 13\.0 m/);
  });

  test("a stop is dragged along the street as one undoable step, and Escape cancels a drag", async ({ page }) => {
    await openStopStreet(page, 30);
    const strip = page.locator("#stop-strip-right");
    const cells = strip.locator('[role="option"]');
    await addStop(page, 0, 10);
    await expect(cells).toHaveCount(2);
    const used = () => strip.locator('[role="option"].stop-cell:not(.gap)');
    const where = async () => await used().getAttribute("aria-label");
    const before = await where();

    await used().scrollIntoViewIfNeeded();
    const box = (await used().boundingBox())!;
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];

    // Escape during a drag puts the stop back.
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
    expect(await where()).not.toBe(before);
    await page.locator("#undo").click();
    expect(await where()).toBe(before);
    await expect(page.locator("#undo")).toBeEnabled(); // the add is still there to undo
    await expect(used()).toHaveCount(1);
  });

  test("a street of driving lanes only has a Stops row, and a curb stop on a narrow sidewalk fails to board", async ({
    page,
  }) => {
    await openStopStreet(page, 20, false);
    // The row is there because a lane serves the bus; this street has no curbside strip of its own.
    await expect(page.locator(".curb-strip")).toHaveCount(0);
    await expect(page.locator("#stop-strip-right").locator('[role="option"]')).toHaveCount(1);
    await addStop(page, 0, 12);
    await expect(page.locator("#stop-right-0")).toHaveClass(/failing/);
    await page.locator("#t-checks").click();
    const row = page.locator("#checks li").filter({ hasText: "Stops have room to board" });
    await expect(row).toHaveClass("bad");
    await expect(row).toContainText("The sidewalk beside the stop is 1.5 m wide; 2.4 m or more is needed.");
  });
});

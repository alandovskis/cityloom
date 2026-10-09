import { expect, live, openJunction, openStreet, test } from "./fixtures";

import type { Page } from "@playwright/test";

const turns = (page: Page) => page.locator("#turns button.turn");

// The editor is reached from the map: a junction of the Plateau, the first the map lists.
test.describe("the junction editor on a junction of the city", () => {
  test.beforeEach(async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("is named for the junction, links back to the map, and has no streets to add or pages to go to", async ({
    page,
  }) => {
    const name = (await page.locator("#street-name").innerText()).trim();
    expect(name).not.toBe("");
    await expect(page.locator("a.back")).toHaveAttribute("href", "map.html");
    await expect(page).toHaveTitle(`${name} · CityLoom`);
    await expect(page.locator(".lower")).toHaveCount(0);
    await expect(page.locator('.surface[href="street.html"]')).toHaveCount(0);
  });

  test("starts as the city lays it out and says how many streets meet and whether it works", async ({ page }) => {
    await expect(page.locator("#arm-count")).toHaveText(/^[3-5] streets$/);
    await expect(page.locator("#fit")).toContainText(/^[3-5] streets, /);
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /^Plan of the junction, north up\. [3-5] streets/,
    );
    await expect(page.locator("#undo")).toBeDisabled();
    await expect(page.locator("#redo")).toBeDisabled();
    await expect(page.locator("#reset")).toBeDisabled();
  });

  test("the control is changed from the details, announced, and can be undone and redone", async ({ page }) => {
    const before = await page.locator("#fit").innerText();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
    await expect(live(page)).toContainText("roundabout");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator("#reset")).toBeEnabled();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText("Undone.");
    await expect(page.locator("#fit")).toHaveText(before);
    await expect(page.locator("#redo")).toBeEnabled();
    await page.locator("#redo").click();
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
  });

  test("start over puts the junction back as it is today and leaves nothing to undo", async ({ page }) => {
    const before = await page.locator("#fit").innerText();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.locator("#reset").click();
    await expect(live(page)).toHaveText("Started over from the junction as it is today.");
    await expect(page.locator("#fit")).toHaveText(before);
    await expect(page.locator("#reset")).toBeDisabled();
    await expect(page.locator("#undo")).toBeDisabled();
  });

  test("a turn is banned and allowed again from the table, and Ctrl+Z undoes it", async ({ page }) => {
    const turn = turns(page).first();
    await expect(turn).toHaveClass(/\bon\b/);
    await turn.click();
    await expect(turn).not.toHaveClass(/\bon\b/);
    await expect(live(page)).not.toBeEmpty();
    await page.keyboard.press("Control+z");
    await expect(turn).toHaveClass(/\bon\b/);
  });

  test("the arrow keys select a street, which the details then show", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(live(page)).toHaveText(/^.+ \(.+\), \d+ degrees$/);
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
  });

  test("Shift and an arrow turn the selected street, and Delete cannot take it away", async ({ page }) => {
    const count = await page.locator("#arm-count").innerText();
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    // A street turns only where it keeps clear of its neighbours: one way or the other.
    await page.keyboard.press("Shift+ArrowRight");
    if (!/bearing/.test(await live(page).innerText())) await page.keyboard.press("Shift+ArrowLeft");
    await expect(live(page)).toContainText("bearing");
    await page.keyboard.press("Delete");
    await expect(page.locator("#arm-count")).toHaveText(count);
  });

  test("a street is pressed on the plan to select it, and its handle is dragged to turn it", async ({ page }) => {
    const arm = page.locator("#drawing g.arm").first();
    const uid = await arm.getAttribute("data-uid");
    await arm.click();
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    const grip = page.locator(`#drawing [data-role="grip-arm"][data-uid="${uid}"]`);
    const box = (await grip.boundingBox())!;
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 120, y + 40, { steps: 8 });
    await page.mouse.up();
    await expect(live(page)).toContainText("bearing");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  test("the notes list the checks, the changes made and the transit measures", async ({ page }) => {
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").first()).toBeVisible();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.locator("#t-changes").click();
    await expect(page.locator("#revs tbody tr")).toHaveCount(2);
    await page.locator("#t-measures").click();
    await expect(page.locator("#measures tbody tr").first()).toBeVisible();
  });
});

test.describe("what the junction editor keeps in the city", () => {
  test("says which OpenStreetMap nodes it was made from, and whether it has been changed", async ({ page }) => {
    await openJunction(page);
    const block = page.locator("#title-block");
    await expect(block.locator('a[href^="https://www.openstreetmap.org/node/"]').first()).toHaveText(
      /^node \d+( v\d+)?$/,
    );
    await expect(block.locator("#tb-state")).toHaveText("As imported");
    await turns(page).first().click();
    await expect(block.locator("#tb-state")).toHaveText("Edited");
    await page.reload();
    await expect(block.locator("#tb-state")).toHaveText("Edited");
  });

  test("what is changed is kept across a reload and shown on the map", async ({ page }) => {
    await openJunction(page);
    const turn = turns(page).first();
    await expect(turn).toHaveClass(/\bon\b/);
    await turn.click();
    await expect(turn).not.toHaveClass(/\bon\b/);
    await page.reload();
    await expect(turns(page).first()).not.toHaveClass(/\bon\b/);
    await page.goto("/map.html");
    await expect(page.locator("#reset")).toBeEnabled();
  });

  test("a change is kept even when the page is left at once", async ({ page }) => {
    const href = await openJunction(page);
    await turns(page).first().click();
    await page.goto("/map.html");
    await page.goto(`/${href}`);
    await expect(turns(page).first()).not.toHaveClass(/\bon\b/);
  });

  test("a junction that is not in the city goes back to the map", async ({ page }) => {
    await page.goto("/intersection.html?junction=999");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("the page without a junction goes back to the map", async ({ page }) => {
    await page.goto("/intersection.html");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("a street change made elsewhere shows in the junction at its end", async ({ page }) => {
    await openStreet(page);
    const end = (await page.locator('#street a[href^="intersection.html"]').first().getAttribute("href"))!;
    const street = page.url();
    await page.goto(`/${end}`);
    await expect(page.locator("#drawing")).toBeVisible();
    const before = await page.locator("#drawing").innerHTML();
    await page.goto(street);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("-");
    await expect(live(page)).toContainText("Resize");
    await page.goto(`/${end}`);
    await expect(page.locator("#drawing")).toBeVisible();
    expect(await page.locator("#drawing").innerHTML()).not.toBe(before);
  });
});

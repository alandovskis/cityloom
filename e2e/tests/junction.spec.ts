import { expect, JUNCTION, live, test } from "./fixtures";

import type { Page } from "@playwright/test";

const turns = (page: Page) => page.locator("#turns button.turn");

test.describe("the junction editor, as a sandbox on the sample junctions", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/intersection.html");
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("starts on the first sample and says whether it works", async ({ page }) => {
    await expect(page.locator("#arm-count")).toHaveText("4 streets");
    await expect(page.locator("#fit")).toHaveText("4 streets, traffic signal. Every check passes.");
    await expect(page.locator("#fit .tick")).toBeVisible();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /^Plan of the junction, north up\. 4 streets/);
    await expect(page.locator("#undo")).toBeDisabled();
    await expect(page.locator("#redo")).toBeDisabled();
    await expect(page.locator("#reset")).toBeDisabled();
  });

  test("a sample is started from, says which junction it is, and clears the changes", async ({ page }) => {
    await page.getByRole("button", { name: /Five ways/ }).click();
    await expect(page.locator("#arm-count")).toHaveText("5 streets");
    await expect(page.getByRole("button", { name: /Five ways/ })).toHaveAttribute("aria-pressed", "true");
    await expect(live(page)).toContainText("5 streets");
  });

  test("the control is changed from the details, announced, and can be undone and redone", async ({ page }) => {
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
    await expect(live(page)).toContainText("roundabout");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator("#reset")).toBeEnabled();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText("Undone.");
    await expect(page.locator("#fit")).toHaveText("4 streets, traffic signal. Every check passes.");
    await expect(page.locator("#redo")).toBeEnabled();
    await page.locator("#redo").click();
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
  });

  test("start over puts the junction back as it is today and leaves nothing to undo", async ({ page }) => {
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.locator("#reset").click();
    await expect(live(page)).toHaveText("Started over from the junction as it is today.");
    await expect(page.locator("#fit")).toHaveText("4 streets, traffic signal. Every check passes.");
    await expect(page.locator("#reset")).toBeDisabled();
    await expect(page.locator("#undo")).toBeDisabled();
  });

  test("pressing a street in the palette adds it in the widest gap", async ({ page }) => {
    await page.locator("#palette button.chip").first().click();
    await expect(page.locator("#arm-count")).toHaveText("5 streets");
    await expect(page.locator("#undo")).toBeEnabled();
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
    await expect(live(page)).toHaveText(/^Sample .* \(.*\), \d+ degrees$/);
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
  });

  test("Shift and an arrow turn the selected street, and Delete takes it away", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    await expect(live(page)).toContainText("bearing");
    await page.keyboard.press("Delete");
    await expect(page.locator("#arm-count")).toHaveText("3 streets");
    await page.keyboard.press("Control+z");
    await expect(page.locator("#arm-count")).toHaveText("4 streets");
  });

  test("a street cannot be removed below three", async ({ page }) => {
    await page.getByRole("button", { name: /Street and lane/ }).click();
    await expect(page.locator("#arm-count")).toHaveText("3 streets");
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Delete");
    await expect(live(page)).toHaveText("A junction needs at least three streets.");
    await expect(page.locator("#arm-count")).toHaveText("3 streets");
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

  test("units change the lengths in the details and the palette", async ({ page }) => {
    await expect(page.locator("#palette")).toContainText("m wide");
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator("#palette")).toContainText("ft wide");
  });
});

test.describe("the junction editor on a junction of the city", () => {
  test("is named for the junction, links back to the map, and cannot add streets", async ({ page }) => {
    await page.goto(`/intersection.html?junction=${JUNCTION}`);
    await expect(page.locator("#street-name")).toHaveText("Junction 1");
    await expect(page.locator("a.back")).toHaveAttribute("href", "map.html");
    await expect(page.locator(".lower")).toBeHidden();
    await expect(page.locator('.surface[href="index.html"]')).toBeHidden();
    await expect(page).toHaveTitle("Junction 1 · CityLoom");
  });

  test("what is changed is kept in the city across a reload and shown on the map", async ({ page }) => {
    await page.goto(`/intersection.html?junction=${JUNCTION}`);
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
    await page.goto(`/intersection.html?junction=${JUNCTION}`);
    await turns(page).first().click();
    await page.goto("/map.html");
    await page.goto(`/intersection.html?junction=${JUNCTION}`);
    await expect(turns(page).first()).not.toHaveClass(/\bon\b/);
  });

  test("a junction that is not in the city goes back to the map", async ({ page }) => {
    await page.goto("/intersection.html?junction=999");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("a street change made elsewhere shows in the junction at its end", async ({ page }) => {
    await page.goto("/intersection.html?junction=2"); // Junction 4, where Sample Avenue 2 ends
    const before = await page.locator("#drawing").innerHTML();
    await page.goto("/index.html?street=1");
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("-");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/intersection.html?junction=2");
    await expect(page.locator("#drawing")).toBeVisible();
    expect(await page.locator("#drawing").innerHTML()).not.toBe(before);
  });
});

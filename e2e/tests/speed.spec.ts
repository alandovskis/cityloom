import type { Page } from "@playwright/test";

import { expect, placesOnMap, test } from "./fixtures";

const speed = (page: Page) => page.locator("#speed-limit");
const sign = (page: Page) => page.locator("#drawing .iso-sign");

/** Opens the first street of the city whose Speed limit control shows `value` ("" is Not set): the Plateau's
 *  streets carry the speeds OpenStreetMap gives them, and the test does not know which is which by number. */
async function openStreetWithSpeed(page: Page, value: string) {
  await page.goto("/map.html");
  for (const href of (await placesOnMap(page)).streets) {
    await page.goto("/" + href);
    await expect(speed(page)).toBeVisible();
    if ((await speed(page).inputValue()) === value) return href;
  }
  throw new Error(`no street whose speed limit is ${JSON.stringify(value)}`);
}

// A street's speed limit: imported from OpenStreetMap's maxspeed, changed in the left panel, shown as a sign in the 2.5D.

test.describe("a street's speed limit", () => {
  test("the imported speed is shown; choosing another changes it, shows the sign in 2.5D and Undo restores it", async ({
    page,
  }) => {
    await openStreetWithSpeed(page, "40");
    await expect(speed(page)).toBeVisible();
    await expect(speed(page).locator("option").first()).toHaveText("Not set");

    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await expect(page.locator("#undo")).toBeEnabled();

    await page.keyboard.press("Escape");
    await page.locator("#view-iso").click();
    await expect(sign(page)).toHaveCount(1);
    await expect(sign(page)).toContainText("MAXIMUM");
    await expect(sign(page)).toContainText("30");
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /Speed limit 30 km\/h\./);

    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await expect(sign(page)).toContainText("40");
  });

  test("the control follows the street through Undo, Redo and Start over", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await page.locator("#redo").click();
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await page.locator("#redo").click();
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#reset").click();
    await expect(speed(page)).toHaveValue("40");
  });

  test("a chosen speed is kept across a reload", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await page.reload();
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#view-iso").click();
    await expect(sign(page)).toContainText("30");
  });

  test("Not set removes the sign and is kept across a reload", async ({ page }) => {
    await openStreetWithSpeed(page, "30");
    await page.locator("#view-iso").click();
    await expect(sign(page)).toHaveCount(1);
    await speed(page).selectOption("");
    await expect(speed(page)).toHaveValue("");
    await expect(sign(page)).toHaveCount(0);
    await page.reload();
    await expect(speed(page)).toHaveValue("");
    await page.locator("#view-iso").click();
    await expect(sign(page)).toHaveCount(0);
  });

  test("a street OpenStreetMap gives no speed shows Not set and no sign", async ({ page }) => {
    await openStreetWithSpeed(page, "");
    await expect(speed(page)).toHaveValue("");
    await page.locator("#view-iso").click();
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
    await expect(sign(page)).toHaveCount(0);
  });

  test("the control is in the left panel and keeps the focus after a change", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    const inspector = page.locator("#speed-slot");
    await expect(inspector).toBeVisible();
    await expect(speed(page)).toBeVisible();
    await speed(page).focus();
    await speed(page).selectOption("20");
    await expect(speed(page)).toHaveValue("20");
    await expect(speed(page)).toBeFocused();
  });
});

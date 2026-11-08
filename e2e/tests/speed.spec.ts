import type { Page } from "@playwright/test";

import { expect, openAnchoredStreet, placesOnMap, splitInto, test } from "./fixtures";

const speed = (page: Page) => page.locator("#speed-limit");
/** The speed limit sign the 2.5D used to draw: it is gone, the limit is set on the Street tab. */
const sign = (page: Page) => page.locator("#drawing .iso-sign");

/** Opens the first street of the city whose Speed limit control shows `value` ("" is Not set): the Plateau's
 *  streets carry the speeds OpenStreetMap gives them, and the test does not know which is which by number. The
 *  control is on the left panel's Street tab, which this shows. */
async function openStreetWithSpeed(page: Page, value: string) {
  await page.goto("/map.html");
  for (const href of (await placesOnMap(page)).streets) {
    await page.goto("/" + href);
    await page.locator("#t-street").click();
    await expect(speed(page)).toBeVisible();
    if ((await speed(page).inputValue()) === value) return href;
  }
  throw new Error(`no street whose speed limit is ${JSON.stringify(value)}`);
}

// A street's speed limit: imported from OpenStreetMap's maxspeed, changed on the left panel's Street tab. The 2.5D
// draws no sign for it.

test.describe("a street's speed limit", () => {
  // `openStreetWithSpeed` opens the city's streets one by one until one has the speed it wants: about 29 s on CI's
  // runner, against the default 30 s.
  test.describe.configure({ timeout: 120_000 });

  test("the imported speed is shown; choosing another changes it and Undo restores it, with no sign in 2.5D", async ({
    page,
  }) => {
    await openStreetWithSpeed(page, "40");
    await expect(speed(page)).toBeVisible();
    await expect(speed(page).locator("option").first()).toHaveText("Not set");

    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(sign(page)).toHaveCount(0);
    await expect(page.locator("#drawing")).not.toHaveAttribute("aria-label", /Speed limit/);

    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await expect(sign(page)).toHaveCount(0);
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
    // The Street tab was the one showing, and it is remembered.
    await expect(speed(page)).toBeVisible();
    await expect(speed(page)).toHaveValue("30");
  });

  test("Not set is kept across a reload", async ({ page }) => {
    await openStreetWithSpeed(page, "30");
    await speed(page).selectOption("");
    await expect(speed(page)).toHaveValue("");
    await page.reload();
    await expect(speed(page)).toHaveValue("");
  });

  test("a street OpenStreetMap gives no speed shows Not set", async ({ page }) => {
    await openStreetWithSpeed(page, "");
    await expect(speed(page)).toHaveValue("");
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
  });

  test("the control keeps the focus after a change", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await expect(speed(page)).toBeVisible();
    await speed(page).focus();
    await speed(page).selectOption("20");
    await expect(speed(page)).toHaveValue("20");
    await expect(speed(page)).toBeFocused();
  });
});

test.describe("the left panel's tabs", () => {
  test("Piece is the tab that opens; Street holds the speed limit and the choice is remembered", async ({ page }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    const piece = page.locator("#t-piece");
    const street = page.locator("#t-street");
    await expect(piece).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#p-piece")).toBeVisible();
    await expect(page.locator("#p-street")).toBeHidden();
    await expect(speed(page)).toBeHidden();

    await street.click();
    await expect(street).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#p-street")).toBeVisible();
    await expect(page.locator("#p-piece")).toBeHidden();
    await expect(speed(page)).toBeVisible();

    await page.reload();
    await expect(street).toHaveAttribute("aria-selected", "true");
    await expect(speed(page)).toBeVisible();
  });

  test("the arrows, Home and End move between the tabs, with one tab stop", async ({ page }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    const piece = page.locator("#t-piece");
    const street = page.locator("#t-street");
    await expect(street).toHaveAttribute("tabindex", "-1");
    await piece.focus();
    await page.keyboard.press("ArrowRight");
    await expect(street).toBeFocused();
    await expect(street).toHaveAttribute("aria-selected", "true");
    await expect(piece).toHaveAttribute("tabindex", "-1");
    await page.keyboard.press("Home");
    await expect(piece).toBeFocused();
    await expect(piece).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("End");
    await expect(street).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(piece).toBeFocused();
  });

  test("pressing a piece of the drawing shows its details", async ({ page }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    await page.locator("#t-street").click();
    await expect(page.locator("#p-street")).toBeVisible();
    await page.locator("#piece-list [role=option]").first().click();
    await expect(page.locator("#t-piece")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#p-piece")).toBeVisible();
  });
});

// A zone has a speed limit of its own (#148): the select on the Street tab sets the selected zone's, and a zone may
// follow the street's again.
test.describe("a zone's speed limit", () => {
  test.describe.configure({ timeout: 120_000 });

  test("is set for the selected zone only, the others follow the street, and Undo takes it back", async ({ page }) => {
    await openAnchoredStreet(page, 30);
    await splitInto(page, 2);
    await page.locator("#t-street").click();
    await expect(speed(page).locator("option").first()).toHaveText(/^Same as the street/);
    await expect(speed(page)).toHaveValue("street");
    const own = await speed(page).locator("option").first().textContent();
    expect(own).toMatch(/^Same as the street: (\d+ km\/h|not set)$/);

    // The new zone is selected; give it a speed the street does not have.
    const chosen = own?.includes("20 km/h") ? "30" : "20";
    await speed(page).selectOption(chosen);
    await expect(speed(page)).toHaveValue(chosen);
    await page.locator("#zone-0").click();
    await expect(speed(page)).toHaveValue("street");
    await page.locator("#zone-1").click();
    await expect(speed(page)).toHaveValue(chosen);

    await speed(page).selectOption("street");
    await expect(speed(page)).toHaveValue("street");
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue(chosen);
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("street");
  });
});

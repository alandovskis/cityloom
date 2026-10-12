import type { Page } from "@playwright/test";

import { expect, live, openStreet, test } from "./fixtures";

const iso = (page: Page) => page.locator("#view-iso");
const section = (page: Page) => page.locator("#view-section");

// The street page's drawing shows the section, which is edited, or the street in 2.5D, which follows it.

test.describe("the street in 2.5D", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("the toggle shows the street in 2.5D without handles, and V brings the section back", async ({ page }) => {
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    const n = await page.locator('#drawing [data-role="seg"]').count();
    await iso(page).click();
    await expect(iso(page)).toHaveAttribute("aria-pressed", "true");
    await expect(section(page)).toHaveAttribute("aria-pressed", "false");
    await expect(live(page)).toHaveText("Showing the street in 2.5D");
    await expect(page.locator("#drawing .iso-top")).toHaveCount(n);
    await expect(page.locator('#drawing [data-role="handle"]')).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /in 2\.5D, along a 40 m stretch\. \d+ segments?\.$/,
    );
    await page.keyboard.press("v");
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    await expect(live(page)).toHaveText("Showing the section");
    await expect(page.locator('#drawing [data-role="handle"]').first()).toBeVisible();
  });

  test("the 2.5D follows an edit made with the keys and outlines the selected piece", async ({ page }) => {
    const second = await page.locator('#drawing [data-role="seg"]').nth(1).getAttribute("data-uid");
    await iso(page).click();
    const top = page.locator(`#drawing .iso-piece[data-uid="${second}"] .iso-top`);
    const before = await top.getAttribute("points");
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    await page.keyboard.press("+");
    await expect(top).not.toHaveAttribute("points", before ?? "");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator('#drawing [data-role="handle"]')).toHaveCount(0);
  });

  test("the view chosen is kept across a reload", async ({ page }) => {
    await iso(page).click();
    await page.reload();
    await expect(iso(page)).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
  });

  test("V typed in the width field is a letter, not the view", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Enter");
    const width = page.locator("#width");
    await expect(width).toBeFocused();
    await width.press("v");
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#drawing .iso-top")).toHaveCount(0);
  });

  test("in French the toggle reads Coupe and 2,5D and the change is said in French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await page.keyboard.press("Escape");
    await expect(section(page)).toHaveText("Coupe");
    await expect(iso(page)).toHaveText("2,5D");
    await iso(page).click();
    await expect(live(page)).toHaveText("Rue affichée en 2,5D");
  });
});

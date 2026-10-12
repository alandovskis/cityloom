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

test("at 400 px the Notes edge tab leaves the toggle clear, wherever the page is scrolled", async ({ page }) => {
  // A short window puts the tab, half-way down it, level with the toggle when the page is at the top.
  await page.setViewportSize({ width: 400, height: 700 });
  await openStreet(page);
  await expect(iso(page)).toBeVisible();
  const tab = (await page.locator(".edge-tab.right").boundingBox())!;
  for (const id of ["#view-section", "#view-iso"]) {
    const b = (await page.locator(id).boundingBox())!;
    // The tab is fixed and the toggle scrolls past it, so only across can they meet.
    expect(b.x + b.width, id).toBeLessThanOrEqual(tab.x);
  }
});

test("in forced colours the view shown is still marked as pressed", async ({ page }) => {
  // Forced colours drop the pressed button's background and shadow; an outline in Highlight stays.
  await page.emulateMedia({ forcedColors: "active" });
  await openStreet(page);
  const outline = (id: string) => page.locator(id).evaluate((b) => getComputedStyle(b).outlineStyle);
  expect(await outline("#view-section")).toBe("solid");
  expect(await outline("#view-iso")).toBe("none");
  await iso(page).click();
  await page.mouse.click(1, 1);
  expect(await outline("#view-iso")).toBe("solid");
  expect(await outline("#view-section")).toBe("none");
});

test("the toggle is not printed", async ({ page }) => {
  await openStreet(page);
  await expect(page.locator("#view-mode")).toBeVisible();
  await page.emulateMedia({ media: "print" });
  await expect(page.locator("#view-mode")).toBeHidden();
});

test("the toggle sits on the street name's line, so the page's head is no taller for it", async ({ page }) => {
  // A row of its own, under the tools, pushed the drawing out of the window wherever the text is wider (CI's fonts).
  await openStreet(page);
  await expect(iso(page)).toBeVisible();
  const toggle = (await page.locator("#view-mode").boundingBox())!;
  const name = (await page.locator("#street-name").boundingBox())!;
  const sub = (await page.locator("#street-sub").boundingBox())!;
  expect(toggle.y + toggle.height).toBeLessThanOrEqual(sub.y + 1);
  expect(toggle.x).toBeGreaterThanOrEqual(name.x + name.width);
  expect(toggle.height).toBeLessThanOrEqual(name.height + 6);
});

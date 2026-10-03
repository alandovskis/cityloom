// What every page shares: the settings menu, the sidebars and the notes tabs.
// The same checks run on each page.

import { expect, live, test } from "./fixtures";

const PAGES = [
  { name: "map", url: "/map.html", firstTab: "t-checks", secondTab: "t-changes", details: "Places hidden." },
  { name: "street", url: "/index.html", firstTab: "t-space", secondTab: "t-checks", details: "Piece details hidden." },
  {
    name: "junction",
    url: "/intersection.html",
    firstTab: "t-space",
    secondTab: "t-checks",
    details: "Details hidden.",
  },
];

for (const p of PAGES) {
  test.describe(`the shell on the ${p.name} page`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(p.url);
      await expect(page.locator("#region option")).toHaveCount(6);
    });

    test("the settings menu opens from the avatar and closes on Escape or a press outside", async ({ page }) => {
      const menu = page.locator("#account-menu");
      const button = page.locator("#account-btn");
      await expect(menu).toBeHidden();
      await button.click();
      await expect(menu).toBeVisible();
      await expect(button).toHaveAttribute("aria-expanded", "true");
      await page.keyboard.press("Escape");
      await expect(menu).toBeHidden();
      await expect(button).toHaveAttribute("aria-expanded", "false");
      await button.click();
      await page.mouse.click(4, 4);
      await expect(menu).toBeHidden();
    });

    test("the units toggle says which is in use", async ({ page }) => {
      await page.locator("#account-btn").click();
      await expect(page.locator('[data-unit="m"]')).toHaveAttribute("aria-pressed", "true");
      await page.locator('[data-unit="ft"]').click();
      await expect(page.locator('[data-unit="ft"]')).toHaveAttribute("aria-pressed", "true");
      await expect(page.locator('[data-unit="m"]')).toHaveAttribute("aria-pressed", "false");
    });

    test("a theme is chosen, announced and remembered", async ({ page }) => {
      await page.locator("#account-btn").click();
      await page.locator('[data-theme-set="dark"]').click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
      await expect(page.locator('[data-theme-set="dark"]')).toHaveAttribute("aria-pressed", "true");
      await expect(live(page)).toHaveText("Dark theme.");
      await page.reload();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    });

    test("a region is chosen, announced with its side of the road, and remembered", async ({ page }) => {
      await page.locator("#account-btn").click();
      await page.locator("#region").selectOption("united-kingdom");
      await expect(live(page)).toHaveText("United Kingdom: traffic keeps left.");
      await page.reload();
      await expect(page.locator("#region")).toHaveValue("united-kingdom");
    });

    test("the left sidebar is hidden and shown from its button or the [ key, and remembered", async ({ page }) => {
      const toggle = page.locator("#inspector-toggle");
      await expect(toggle).toHaveAttribute("aria-expanded", "true");
      await toggle.click();
      await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
      await expect(toggle).toHaveAttribute("aria-expanded", "false");
      await expect(live(page)).toHaveText(p.details);
      await page.reload();
      await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
      await page.keyboard.press("[");
      await expect(page.locator("html")).not.toHaveAttribute("data-inspector", "closed");
    });

    test("the notes are hidden and shown from their button or the ] key", async ({ page }) => {
      await page.locator("#notes-toggle").click();
      await expect(page.locator("html")).toHaveAttribute("data-notes", "closed");
      await expect(live(page)).toHaveText("Notes hidden.");
      await page.keyboard.press("]");
      await expect(page.locator("html")).not.toHaveAttribute("data-notes", "closed");
      await expect(live(page)).toHaveText("Notes shown.");
    });

    test("the keys for the sidebars are left alone while typing in a field", async ({ page }) => {
      await page.locator("#account-btn").click();
      await page.locator("#region").focus();
      await page.keyboard.press("[");
      await expect(page.locator("html")).not.toHaveAttribute("data-inspector", "closed");
    });

    test("the notes tabs show one panel, move with the arrow keys, and remember the choice", async ({ page }) => {
      const first = page.locator(`#${p.firstTab}`);
      const second = page.locator(`#${p.secondTab}`);
      await expect(first).toHaveAttribute("aria-selected", "true");
      await first.focus();
      await page.keyboard.press("ArrowRight");
      await expect(second).toHaveAttribute("aria-selected", "true");
      await expect(second).toBeFocused();
      await expect(page.locator(`#${await first.getAttribute("aria-controls")}`)).toBeHidden();
      await expect(page.locator(`#${await second.getAttribute("aria-controls")}`)).toBeVisible();
      await page.reload();
      await expect(second).toHaveAttribute("aria-selected", "true");
      await second.focus();
      await page.keyboard.press("ArrowLeft");
      await expect(first).toHaveAttribute("aria-selected", "true");
      await page.keyboard.press("End");
      await expect(page.locator(".notes .tab").last()).toHaveAttribute("aria-selected", "true");
    });
  });
}

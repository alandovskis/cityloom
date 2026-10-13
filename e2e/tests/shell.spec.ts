// What every page shares: the settings menu, the sidebars and the notes tabs.
// The same checks run on each page.

import type { Page } from "@playwright/test";

import { expect, live, openJunction, openStreet, test } from "./fixtures";

// The editors are reached from the map, as a person does.
const PAGES = [
  {
    name: "map",
    open: (page: Page) => page.goto("/map.html"),
    firstTab: "t-checks",
    secondTab: "t-changes",
    details: "Area rules hidden.",
  },
  {
    name: "street",
    open: (page: Page) => openStreet(page),
    firstTab: "t-space",
    secondTab: "t-checks",
    details: "Piece details hidden.",
  },
  {
    name: "junction",
    open: (page: Page) => openJunction(page),
    firstTab: "t-space",
    secondTab: "t-checks",
    details: "Details hidden.",
  },
];

for (const p of PAGES) {
  test.describe(`the shell on the ${p.name} page`, () => {
    test.beforeEach(async ({ page }) => {
      await p.open(page);
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

    test("the keyboard shortcuts are under Help, not in the settings menu, and only one of the two is open", async ({
      page,
    }) => {
      const help = page.locator("#help-menu");
      const button = page.locator("#help-btn");
      const settings = page.locator("#account-menu");
      await expect(help).toBeHidden();
      await expect(settings.locator("#keys")).toHaveCount(0);
      await button.click();
      await expect(help).toBeVisible();
      await expect(help.locator("#keys")).toBeVisible();
      await expect(button).toHaveAttribute("aria-expanded", "true");
      await page.locator("#account-btn").click();
      await expect(settings).toBeVisible();
      await expect(help).toBeHidden();
      await button.click();
      await expect(settings).toBeHidden();
      await page.keyboard.press("Escape");
      await expect(help).toBeHidden();
      await expect(button).toBeFocused();
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

    test("the notes are hidden and shown from their button or the ] key", async ({ page }) => {
      await page.locator("#notes-toggle").click();
      await expect(page.locator("html")).toHaveAttribute("data-notes", "closed");
      await expect(live(page)).toHaveText("Notes hidden.");
      await page.keyboard.press("]");
      await expect(page.locator("html")).not.toHaveAttribute("data-notes", "closed");
      await expect(live(page)).toHaveText("Notes shown.");
    });

    if (p.details) {
      test("the left sidebar is hidden and shown from its button or the [ key, and remembered", async ({ page }) => {
        const toggle = page.locator("#inspector-toggle");
        await expect(toggle).toHaveAttribute("aria-expanded", "true");
        await toggle.click();
        await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
        await expect(toggle).toHaveAttribute("aria-expanded", "false");
        await expect(live(page)).toHaveText(p.details);
        await page.reload();
        await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
        // A head script sets that attribute at once; the key is bound only when the shell is mounted, which
        // is when the region list is filled.
        await expect(page.locator("#region option")).toHaveCount(6);
        await page.keyboard.press("[");
        await expect(page.locator("html")).not.toHaveAttribute("data-inspector", "closed");
      });

      test("the keys for the sidebars are left alone while typing in a field", async ({ page }) => {
        await page.locator("#account-btn").click();
        await page.locator("#region").focus();
        await page.keyboard.press("[");
        await expect(page.locator("html")).not.toHaveAttribute("data-inspector", "closed");
      });
    }

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

// Print is under Share on the two editors; the map page has neither.
for (const p of PAGES.filter((p) => p.name !== "map")) {
  test(`Print is under Share, not in the settings menu, on the ${p.name} page`, async ({ page }) => {
    await p.open(page);
    const share = page.locator("#share-menu");
    const button = page.locator("#share-btn");
    await expect(share).toBeHidden();
    await expect(page.locator("#account-menu #print")).toHaveCount(0);
    await button.click();
    await expect(share).toBeVisible();
    await expect(button).toHaveAttribute("aria-expanded", "true");
    await expect(share.locator("#print")).toBeVisible();
    // Only one of the menus is open: pressing the settings button closes Share.
    await page.locator("#account-btn").click();
    await expect(share).toBeHidden();
    await expect(page.locator("#account-menu")).toBeVisible();
    await page.locator("#account-btn").click();
    await button.click();
    await page.keyboard.press("Escape");
    await expect(share).toBeHidden();
  });
}

// A keyboard user opens one menu, then another, with no pointer press and no focus leaving a menu.
for (const p of PAGES.filter((p) => p.name !== "map")) {
  test(`only one of Settings, Help and Share is open when they are opened by keyboard on the ${p.name} page`, async ({
    page,
  }) => {
    await p.open(page);
    const menus = { "#account-btn": "#account-menu", "#help-btn": "#help-menu", "#share-btn": "#share-menu" };
    for (const [from, fromMenu] of Object.entries(menus)) {
      for (const [to, toMenu] of Object.entries(menus)) {
        if (from === to) continue;
        await page.locator(from).focus();
        await page.keyboard.press("Enter");
        await expect(page.locator(fromMenu)).toBeVisible();
        await page.locator(to).focus();
        await page.keyboard.press("Enter");
        await expect(page.locator(toMenu)).toBeVisible();
        await expect(page.locator(fromMenu)).toBeHidden();
        await page.keyboard.press("Escape");
        await expect(page.locator(toMenu)).toBeHidden();
      }
    }
  });
}

// Phone width: the header's buttons show only icons, and a menu must stay inside the window.
for (const p of PAGES.filter((p) => p.name !== "map")) {
  test(`at phone width the header menus have names and stay inside the window on the ${p.name} page`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await p.open(page);
    for (const [name, menu] of [
      ["Share", "#share-menu"],
      ["Help", "#help-menu"],
      ["Settings", "#account-menu"],
    ]) {
      const button = page.getByRole("button", { name, exact: true });
      await button.click();
      const box = await page.locator(menu).boundingBox();
      expect(box, `${name} menu is shown`).not.toBeNull();
      expect(box!.x, `${name} menu starts inside the window`).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width, `${name} menu ends inside the window`).toBeLessThanOrEqual(390);
      await page.keyboard.press("Escape");
    }
  });

  test(`Print keeps focus on the Share button on the ${p.name} page`, async ({ page }) => {
    await p.open(page);
    await page.evaluate(() => {
      window.print = () => {};
    });
    await page.locator("#share-btn").click();
    await page.locator("#print").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#share-menu")).toBeHidden();
    await expect(page.locator("#share-btn")).toBeFocused();
  });
}

// What every page shares: the settings menu, the sidebars and the notes tabs.
// The same checks run on each page.

import type { Page } from "@playwright/test";

import { expect, live, openJunction, openStreet, test } from "./fixtures";

// The editors are reached from the map, as a person does.
const PAGES = [
  {
    name: "map",
    open: (page: Page) => page.goto("/map.html"),
    // no tabs: the details are levels reached from the city
    firstTab: undefined,
    secondTab: undefined,
    // no left sidebar: the area rules are a level of the details
    details: undefined,
  },
  {
    name: "street",
    open: (page: Page) => openStreet(page),
    firstTab: "t-space",
    secondTab: "t-checks",
    details: "Details hidden.",
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
      // The street page's notes are a tray that starts closed to its handle; the other pages' start open.
      const html = page.locator("html");
      const open = (await html.getAttribute("data-notes")) !== "closed";
      const said = (shown: boolean) => (shown ? "Notes shown." : "Notes hidden.");
      await page.locator("#notes-toggle").click();
      await expect(html).toHaveAttribute("data-notes", open ? "closed" : "open");
      await expect(live(page)).toHaveText(said(!open));
      await page.keyboard.press("]");
      await expect(html).toHaveAttribute("data-notes", open ? "open" : "closed");
      await expect(live(page)).toHaveText(said(open));
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

    // the map's details are levels, not tabs
    if (p.firstTab) {
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
    }
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
    // The shell binds the menus once the page's module has loaded; a key pressed before that does nothing.
    await expect(page.locator("#region option")).toHaveCount(6);
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

// The Sign in button is the last of the header's buttons and the group is anchored to the right: when the answer to
// /api/me comes, the button must appear in the place it kept, not push Settings, Help and Share to the left.
for (const p of PAGES) {
  test(`the header's buttons do not move when the sign-in button appears on the ${p.name} page`, async ({ page }) => {
    let release!: () => void;
    const held = new Promise<void>((resolve) => (release = resolve));
    await page.route("**/api/me", async (route) => {
      await held;
      await route.fulfill({ status: 503 });
    });
    await p.open(page);
    // The box is mounted with the shell, after the page's own markup: measure once it holds its place.
    await expect(page.locator(".session-box")).toHaveClass(/pending/);
    const buttons = ["#account-btn", "#help-btn"].concat(p.name === "map" ? [] : ["#share-btn"]);
    const xs = async () =>
      Promise.all(
        buttons.map(async (b) => {
          const box = await page.locator(b).boundingBox();
          return box ? box.x : null;
        }),
      );
    const before = await xs();
    expect(before[0]).not.toBeNull();
    release();
    await expect(page.locator(".session-box")).toBeVisible();
    await expect(page.locator(".session-box")).not.toHaveClass(/pending/);
    expect(await xs()).toEqual(before);
  });
}

// A project's street or junction edit saved before the importer merged a block's pieces has no `import` stamp: it is
// stale, discarded when one of the project's pages opens, and the page says so once. The notice floats below the
// fixed bar where the bar floats, so it must be seen, in the viewport, and reachable at every width; at a laptop's
// width the side panels leave it less room, and it must not cover their heads.
const CLEARED =
  "The map was rebuilt with one street per block, so your earlier street and junction edits were cleared.";
const NOTICE_PAGES = [
  { name: "map", open: (page: Page) => page.goto("/map.html"), ready: "#region option" },
  { name: "street", open: (page: Page) => openStreet(page), ready: "#region option" },
  { name: "intersection", open: (page: Page) => openJunction(page), ready: "#region option" },
  { name: "timeline", open: (page: Page) => page.goto("/timeline.html"), ready: "#region option" },
];
const WIDTHS = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "laptop", width: 1280, height: 800 },
  { name: "phone", width: 390, height: 844 },
];

/** Whether two boxes share any area. */
const overlap = (a: { x: number; y: number; width: number; height: number }, b: typeof a) =>
  a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

for (const p of NOTICE_PAGES) {
  for (const w of WIDTHS) {
    test(`a project's cleared edits are announced once on the ${p.name} page at ${w.name} width`, async ({ page }) => {
      await page.setViewportSize({ width: w.width, height: w.height });
      await p.open(page);
      await expect(page.locator(p.ready).first()).toBeAttached();
      await page.evaluate(() => {
        const record = { ways: [1], nodes: [], leaving: {}, street: {} };
        localStorage.setItem(
          "cityloom-edits@first",
          JSON.stringify({ version: 17, streets: { w1: record }, junctions: {} }),
        );
      });
      await page.reload();
      const notice = page.locator("p.shell-notice");
      await expect(notice).toHaveAttribute("role", "status");
      await expect(notice).toContainText(CLEARED);
      await expect(notice).toBeVisible();
      await expect(notice).toBeInViewport({ ratio: 1 });
      const box = (await notice.boundingBox())!;
      for (const panel of await page.locator(".panel:visible").all()) {
        const covered = (await panel.boundingBox())!;
        expect(
          overlap(box, covered),
          `the notice ${JSON.stringify(box)} covers a panel ${JSON.stringify(covered)}`,
        ).toBe(false);
      }
      await notice.getByRole("button", { name: "Dismiss" }).click();
      await expect(notice).toHaveCount(0);
      await page.reload();
      await expect(page.locator(p.ready).first()).toBeAttached();
      await expect(notice).toHaveCount(0);
    });
  }
}

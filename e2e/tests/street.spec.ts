import type { Page } from "@playwright/test";

import { expect, live, openStreet, test } from "./fixtures";

const selectFirstPiece = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
};

const pieceCount = (page: Page) => page.locator("#drawing").getAttribute("aria-label");

/** How many pieces the street has, from what the drawing says of itself. */
const pieces = async (page: Page) => Number(/(\d+) segments/.exec((await pieceCount(page)) ?? "")?.[1]);

// The editor is reached from the map: a street of the Plateau, the first the map lists.

test.describe("the street editor on a street of the city", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("starts as the city has the street and says whether the pieces fit", async ({ page }) => {
    await expect(page.locator("#street-name")).not.toHaveText("");
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /\d+ segments, ([\d.]+) m of \1 m\. Every metre of the street is used\.$/,
    );
    await expect(page.locator("#fit")).toHaveText("Every metre of the street is used.");
    await expect(page.locator("#undo")).toBeDisabled();
    await expect(page.locator("#redo")).toBeDisabled();
    await expect(page.locator("#reset")).toBeDisabled();
  });

  test("choosing French in the settings menu changes the page's language and its markup", async ({ page }) => {
    await page.locator("#account-btn").click();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expect(page.locator('[data-lang="fr-CA"]')).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#account-menu .menu-h").first()).toHaveText("Réglages");
  });

  test("the welcome note is shown at first and put away until asked for again", async ({ page }) => {
    await expect(page.locator("#welcome")).toBeVisible();
    await page.locator("#welcome-dismiss").click();
    await expect(page.locator("#welcome")).toBeHidden();
    await page.reload();
    await expect(page.locator("#welcome")).toBeHidden();
    await page.locator("#account-btn").click();
    await page.locator("#how-btn").click();
    await expect(page.locator("#welcome")).toBeVisible();
  });

  test("a piece is selected with the arrow keys, announced, and shown in the details", async ({ page }) => {
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
    await selectFirstPiece(page);
    await expect(live(page)).toHaveText(/^.+, [\d.]+ m, 1 of \d+$/);
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
  });

  test("a piece is pressed to select it", async ({ page }) => {
    await page.locator('#drawing [data-role="seg"]').nth(1).click();
    await expect(live(page)).toHaveText(/, 2 of \d+$/);
  });

  test("making a piece wider says that the street is too wide, and narrower makes room again", async ({ page }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("+");
    await expect(live(page)).toHaveText(/^Resize .+\. 0\.1 m too wide\. Make a piece narrower or remove one\.$/);
    await expect(page.locator("#fit")).toHaveText("0.1 m too wide. Make a piece narrower or remove one.");
    await page.keyboard.press("-");
    await expect(page.locator("#fit")).toHaveText("Every metre of the street is used.");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  test("a street that fits carries a tick, which goes when it is too wide and draws itself in when it fits again", async ({
    page,
  }) => {
    const tick = page.locator("#fit .tick");
    await expect(tick).toBeVisible();
    await expect(tick).not.toHaveClass(/\bfresh\b/);
    await selectFirstPiece(page);
    await page.keyboard.press("+");
    await expect(tick).toHaveCount(0);
    await page.keyboard.press("-");
    await expect(tick).toBeVisible();
    await expect(tick).toHaveClass(/\bfresh\b/);
    await expect(tick.locator("path")).toHaveCSS("animation-name", "draw-tick");
    await page.keyboard.press("+");
    await expect(tick).toHaveCount(0);
  });

  test("with reduced motion the tick is simply there", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await selectFirstPiece(page);
    await page.keyboard.press("+");
    await page.keyboard.press("-");
    await expect(page.locator("#fit .tick.fresh path")).toHaveCSS("animation-name", "none");
  });

  test("removing a piece frees its width, and Ctrl+Z puts it back", async ({ page }) => {
    const before = await pieces(page);
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await expect(page.locator("#fit")).toHaveText(/[\d.]+ m of the street is still unused\.$/);
    expect(await pieces(page)).toBe(before - 1);
    await page.keyboard.press("Control+z");
    await expect(page.locator("#fit")).toHaveText("Every metre of the street is used.");
    await expect(live(page)).toHaveText(/^Undone\. /);
  });

  test("undo and redo are announced with how the pieces now stand", async ({ page }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText("Undone. Every metre of the street is used.");
    await page.locator("#redo").click();
    await expect(live(page)).toHaveText(/^Redone\. [\d.]+ m left to use\.$/);
  });

  test("Shift and an arrow move the selected piece one place along", async ({ page }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("Shift+ArrowRight");
    await expect(live(page)).toContainText("Move");
    await expect(page.locator("#undo")).toBeEnabled();
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowRight");
    await expect(live(page)).toHaveText(/, 2 of \d+$/);
  });

  test("start over puts the street back as it is today and can itself be undone", async ({ page }) => {
    const before = await pieces(page);
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await page.locator("#reset").click();
    await expect(live(page)).toHaveText("Started over from the street as it is today. Undo brings your changes back.");
    expect(await pieces(page)).toBe(before);
    await page.locator("#undo").click();
    expect(await pieces(page)).toBe(before - 1);
  });

  test("a piece is added from the menu, which opens and closes", async ({ page }) => {
    const before = await pieces(page);
    const menu = page.locator("#add-btn");
    await expect(menu).toHaveAttribute("aria-expanded", "false");
    await menu.click();
    await expect(menu).toHaveAttribute("aria-expanded", "true");
    await page.locator(".add-item", { hasText: "Bike lane" }).click();
    expect(await pieces(page)).toBe(before + 1);
    await expect(live(page)).toContainText("Add bike lane");
    await expect(page.locator("#fit")).toHaveText(/too wide/);
  });

  for (const [width, height] of [
    [1440, 900],
    [1280, 720],
    [1024, 768],
    [390, 844],
  ]) {
    test(`the menu of pieces stays inside a ${width} by ${height} window and scrolls to its last piece`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height });
      await page.locator("#add-btn").click();
      const menu = page.locator("#add-menu");
      await expect(menu).toBeVisible();
      const box = (await menu.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.y).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
      expect(box.y + box.height).toBeLessThanOrEqual(height);
      const last = page.locator(".add-item", { hasText: "Street lamp" });
      await last.scrollIntoViewIfNeeded();
      const item = (await last.boundingBox())!;
      expect(item.y + item.height).toBeLessThanOrEqual(height);
    });
  }

  test("a boundary between two pieces is dragged to share width between them", async ({ page }) => {
    const handle = page.locator('#drawing [data-role="handle"]').first();
    const box = (await handle.boundingBox())!;
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 40, y, { steps: 6 });
    await page.mouse.up();
    await expect(live(page)).toContainText("Resize");
    await expect(page.locator("#fit")).toHaveText("Every metre of the street is used.");
    await expect(page.locator("#undo")).toBeEnabled();
    await page.locator("#t-changes").click();
    await expect(page.locator("#revs tbody tr")).toHaveCount(2);
  });

  test("a piece is dragged along to a new place", async ({ page }) => {
    const first = page.locator('#drawing [data-role="seg"]').first();
    const second = page.locator('#drawing [data-role="seg"]').nth(1);
    const a = (await first.boundingBox())!;
    const b = (await second.boundingBox())!;
    await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
    await page.mouse.down();
    await page.mouse.move(b.x + b.width + 10, b.y + b.height / 2, { steps: 10 });
    await page.mouse.up();
    await expect(live(page)).toContainText("Move");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  test("the details change a piece's width and the street follows", async ({ page }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("Enter");
    const width = page.locator("#inspector input").first();
    await expect(width).toBeFocused();
    const was = Number(await width.inputValue());
    await width.fill(String(Math.round((was - 0.3) * 10) / 10));
    await width.press("Enter");
    await expect(page.locator("#fit")).toHaveText(/0\.3 m of the street is still unused\.$/);
  });

  test("a width typed with a decimal comma is read as a decimal", async ({ page }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("Enter");
    const width = page.locator("#inspector input").first();
    await expect(width).toBeFocused();
    await width.fill("2,5");
    await width.press("Enter");
    await expect(width).toHaveValue("2.50");
  });

  test("the notes list the width shared out, the checks, the changes and the transit measures", async ({ page }) => {
    await expect(page.locator("#space")).not.toBeEmpty();
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").first()).toBeVisible();
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await page.locator("#t-changes").click();
    await expect(page.locator("#revs tbody tr")).toHaveCount(2);
    await page.locator("#t-measures").click();
    await expect(page.locator("#t-measures")).toHaveAttribute("aria-selected", "true");
  });

  test("an Atlas measure is arranged on a street it suits, and the change can be undone", async ({ page }) => {
    await page.locator("#t-measures").click();
    await page.locator(".m-apply:not([disabled])").first().click();
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(live(page)).not.toBeEmpty();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/^Undone\./);
  });

  test("units change the lengths shown", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /ft of /);
  });
});

test.describe("what the street editor keeps in the city", () => {
  test("is named for the street, says where it runs between, and links back to the map", async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#street-name")).not.toHaveText("");
    await expect(page.locator("#street-sub")).not.toHaveText("");
    await expect(page.locator("a.back")).toHaveAttribute("href", "map.html");
    await expect(page.locator('.surface[href="intersection.html"]')).toHaveCount(0);
  });

  test("keeps its own title, named for the street, when the language changes", async ({ page }) => {
    await openStreet(page);
    await expect(page).toHaveTitle(/between .* and .* · CityLoom/);
    const name = (await page.locator("#street-name").textContent()) ?? "";
    expect(name).not.toBe("");
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    const title = await page.title();
    expect(title).toContain(name);
    expect(title).toContain("CityLoom");
    expect(title).not.toBe("CityLoom street editor");
    expect(title).not.toBe("Éditeur de rues CityLoom");
  });

  test("says which OpenStreetMap ways it was made from, and whether it has been changed", async ({ page }) => {
    await openStreet(page);
    const block = page.locator("#title-block");
    await expect(block.locator('a[href^="https://www.openstreetmap.org/way/"]').first()).toHaveText(
      /^way \d+( v\d+)?$/,
    );
    await expect(block.locator("#tb-state")).toHaveText("As imported");
    await selectFirstPiece(page);
    await page.keyboard.press("Enter");
    const width = page.locator("#inspector input").first();
    await width.fill(String(Math.round((Number(await width.inputValue()) - 0.3) * 10) / 10));
    await width.press("Enter");
    await expect(block.locator("#tb-state")).toHaveText("Edited");
    await page.reload();
    await expect(block.locator("#tb-state")).toHaveText("Edited");
  });

  test("what is changed is kept in the city across a reload", async ({ page }) => {
    await openStreet(page);
    const before = await pieces(page);
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    expect(await pieces(page)).toBe(before - 1);
    await page.reload();
    expect(await pieces(page)).toBe(before - 1);
  });

  test("a change is kept even when the page is left at once", async ({ page }) => {
    const href = await openStreet(page);
    const before = await pieces(page);
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await page.goto("/map.html");
    await page.goto(`/${href}`);
    expect(await pieces(page)).toBe(before - 1);
  });

  test("a street that is not in the city goes back to the map", async ({ page }) => {
    await page.goto("/street.html?street=999");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("the page without a street goes back to the map", async ({ page }) => {
    await page.goto("/street.html");
    await expect(page).toHaveURL(/map\.html$/);
  });
});

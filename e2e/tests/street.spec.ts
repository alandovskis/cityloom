import type { Page } from "@playwright/test";

import {
  addFromMenu,
  expect,
  live,
  mapReady,
  menuItem,
  OPEN_ENDS_TILE,
  openStreet,
  makeStretches,
  pickRunAt,
  pickStretch,
  pickToEnd,
  pickWhole,
  planBoundaries,
  streetLength,
  test,
  tileFrom,
  transform,
} from "./fixtures";

const selectFirstPiece = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
};

/** The street's pieces by name, in order. */
const pieceNames = (page: Page) => page.locator("#piece-list .piece-name").allTextContents();

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
      /in 2\.5D, along a 40 m stretch\. \d+ segments\./,
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
    await expect(page.locator("#p-street")).toBeVisible();
    await selectFirstPiece(page);
    await expect(live(page)).toHaveText(/^.+, [\d.]+ m, 1 of \d+$/);
    await expect(page.locator("#p-piece")).toBeVisible();
    // Letting go of the piece shows the street.
    await page.keyboard.press("Escape");
    await expect(page.locator("#p-street")).toBeVisible();
    await expect(page.locator("#p-piece")).toBeHidden();
  });

  test("a buffer as narrow as an imported curb's is selected with the arrow keys and announced", async ({ page }) => {
    await addFromMenu(page, "Buffer");
    await page.locator("#wrap").focus();
    for (let k = 0; k < 6; k++) await page.keyboard.press("-");
    const last = (await pieces(page)) - 1; // a new piece goes at the end
    await page.keyboard.press("Escape");
    await page.locator("#wrap").focus();
    for (let k = 0; k <= last; k++) await page.keyboard.press("ArrowRight");
    await expect(live(page)).toHaveText(new RegExp(`^Buffer, 0\\.15 m, ${last + 1} of ${last + 1}$`));
    await expect(page.locator("#p-piece")).toBeVisible();
  });

  test("a piece is pressed to select it", async ({ page }) => {
    await page.locator("#drawing .iso-piece .iso-top").first().click({ force: true });
    await expect(live(page)).toHaveText(/, \d+ of \d+$/);
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
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
    await page.locator("#add-topic-cycling").click();
    await expect(page.locator("#add-topic-cycling")).toHaveAttribute("aria-selected", "true");
    await menuItem(page, "Bike lane").click();
    await expect(menu).toHaveAttribute("aria-expanded", "false");
    await expect(page.locator("#add-menu")).toBeHidden();
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
      await page.locator("#add-topic-utilities").click();
      const last = menuItem(page, "Street lamp");
      await last.scrollIntoViewIfNeeded();
      const item = (await last.boundingBox())!;
      expect(item.x).toBeGreaterThanOrEqual(0);
      expect(item.y).toBeGreaterThanOrEqual(0);
      expect(item.x + item.width).toBeLessThanOrEqual(width);
      expect(item.y + item.height).toBeLessThanOrEqual(height);
    });
  }

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

  test("Enter in the width field keeps the width and Escape puts it back, each giving the keys to the drawing", async ({
    page,
  }) => {
    await selectFirstPiece(page);
    await page.keyboard.press("Enter");
    const width = page.locator("#inspector input").first();
    await expect(width).toBeFocused();
    const was = await width.inputValue();
    await width.fill("0.9");
    await width.press("Escape");
    await expect(page.locator("#wrap")).toBeFocused();
    await expect(page.locator("#inspector input").first()).toHaveValue(was);
    await page.keyboard.press("Enter");
    await page
      .locator("#inspector input")
      .first()
      .fill(String(Number(was) - 0.1));
    await page.locator("#inspector input").first().press("Enter");
    await expect(page.locator("#wrap")).toBeFocused();
    await expect(page.locator("#fit")).toHaveText(/0\.1 m of the street is still unused\.$/);
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

  test("the piece pane's buttons move the piece along and remove it", async ({ page }) => {
    const cells = page.locator("#piece-list [role=option]");
    const count = await cells.count();
    const first = (await cells.nth(0).getAttribute("id"))!;
    await cells.nth(0).click();
    await expect(page.locator("#piece-move-left")).toBeDisabled();
    await page.locator("#piece-move-right").click();
    await expect(cells.nth(1)).toHaveAttribute("id", first);
    await page.locator("#piece-remove").click();
    await expect(cells).toHaveCount(count - 1);
  });

  test("the Checks tab shows what fails first and folds what passes until asked", async ({ page }) => {
    await page.locator("#t-checks").click();
    const passing = page.locator("#checks li.ok");
    const fold = page.locator("#checks-passing");
    await expect(fold).toHaveAttribute("aria-expanded", "false");
    await expect(passing.first()).toBeHidden();
    await fold.click();
    await expect(fold).toHaveAttribute("aria-expanded", "true");
    await expect(passing.first()).toBeVisible();
    await fold.click();
    await expect(passing.first()).toBeHidden();
  });

  test("the notes list the width shared out, the checks and the changes, over where the street came from", async ({
    page,
  }) => {
    await expect(page.locator("#space")).not.toBeEmpty();
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li")).not.toHaveCount(0);
    await selectFirstPiece(page);
    await page.keyboard.press("Delete");
    await page.locator("#t-changes").click();
    await expect(page.locator("#revs li")).toHaveCount(2);
    // where the street came from is in the title block, under every tab: there is no tab of its own
    await expect(page.locator("#title-block #tb-state")).toBeVisible();
    await expect(page.locator("#t-provenance")).toHaveCount(0);
  });

  test("an Atlas measure is arranged on a street it suits, and the change can be undone", async ({ page }) => {
    await transform(page, ".transit-item", "transit");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(live(page)).not.toBeEmpty();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/^Undone\./);
  });

  test("units change the lengths shown", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /along a \d+ ft stretch/);
  });

  test("the left panel is one scroller holding the inspector and nothing else", async ({ page }) => {
    await page.setViewportSize({ width: 1366, height: 768 });
    await selectFirstPiece(page);
    const panel = page.locator("#details-panel");
    await expect(panel.locator(":scope > .panel-body")).toHaveCount(1);
    const body = panel.locator(":scope > .panel-body");
    await expect(body.locator("#inspector")).toHaveCount(1);
    await expect(page.locator("#transit-priority, #h-transit")).toHaveCount(0);
    await expect(panel.locator(".panel-body")).toHaveCount(1);
    // The inspector takes the height of its own content, not half of the panel.
    const inspector = await page
      .locator("#inspector")
      .evaluate((el) => ({ box: el.getBoundingClientRect().height, content: el.scrollHeight }));
    expect(inspector.box).toBeGreaterThanOrEqual(inspector.content - 1);
    // The width field is reachable: inside the panel, or by scrolling the one scroller.
    await page.locator("#width").scrollIntoViewIfNeeded();
    const [field, area] = await Promise.all([page.locator("#width").boundingBox(), body.boundingBox()]);
    expect(field!.y).toBeGreaterThanOrEqual(area!.y - 1);
    expect(field!.y + field!.height).toBeLessThanOrEqual(area!.y + area!.height + 1);
  });
});

test.describe("the add menu", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("search finds an item in another topic and adds it", async ({ page }) => {
    const before = await pieces(page);
    await page.locator("#add-btn").click();
    await expect(page.locator("#add-search")).toBeFocused();
    await expect(page.locator("#add-topic-walking")).toHaveAttribute("aria-selected", "true");
    await page.locator("#add-search").fill("lamp");
    const results = page.locator("#add-results .add-item");
    await expect(results).toHaveCount(1);
    await expect(results.locator("b")).toHaveText("Street lamp");
    await expect(results.locator(".add-topic-name")).toHaveText("Utilities");
    await expect(page.locator(".add-topics")).toHaveCount(0);
    await expect(live(page)).toHaveText("One match");
    await page.keyboard.press("ArrowDown");
    await expect(results).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#add-menu")).toBeHidden();
    await expect(page.locator("#add-btn")).toBeFocused();
    expect(await pieces(page)).toBe(before + 1);
  });

  test("nothing matches says so and clearing returns to the topic", async ({ page }) => {
    await page.locator("#add-btn").click();
    await page.locator("#add-topic-furniture").click();
    await page.locator("#add-search").fill("zzz");
    await expect(page.locator(".add-none")).toHaveText("Nothing matches “zzz”.");
    await expect(page.locator("#add-menu .add-item")).toHaveCount(0);
    await expect(live(page)).toHaveText("No matches");
    await page.locator("#add-search").fill("");
    await expect(page.locator(".add-none")).toHaveCount(0);
    await expect(page.locator("#add-topic-furniture")).toHaveAttribute("aria-selected", "true");
    await expect(menuItem(page, "Bench")).toBeVisible();
  });

  test("the menu opens on the chosen piece's topic", async ({ page }) => {
    await addFromMenu(page, "Bike lane");
    await page.locator("#piece-list [role=option]").filter({ hasText: "Bike lane" }).first().click();
    await expect(page.locator("#p-piece")).toBeVisible();
    await page.locator("#add-btn").click();
    await expect(page.locator("#add-topic-cycling")).toHaveAttribute("aria-selected", "true");
    await expect(menuItem(page, "Bike lane")).toBeVisible();
  });

  test("at phone width the topics are a row of tabs", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.locator("#add-btn").click();
    const tabs = page.locator(".add-topics");
    await expect(tabs).toHaveAttribute("aria-orientation", "horizontal");
    const row = (await tabs.boundingBox())!;
    const panel = (await page.locator("#add-panel").boundingBox())!;
    expect(row.y + row.height).toBeLessThanOrEqual(panel.y);
    const menu = (await page.locator("#add-menu").boundingBox())!;
    expect(menu.x).toBeGreaterThanOrEqual(0);
    expect(menu.y).toBeGreaterThanOrEqual(0);
    expect(menu.x + menu.width).toBeLessThanOrEqual(390);
    expect(menu.y + menu.height).toBeLessThanOrEqual(844);
  });

  test("at phone width the chosen topic's tab is scrolled into the row", async ({ page }) => {
    await addFromMenu(page, "Street lamp");
    await page.locator("#piece-list [role=option]").filter({ hasText: "Street lamp" }).first().click();
    await page.setViewportSize({ width: 390, height: 844 });
    const row = page.locator(".add-topics");
    const shown = async (id: string) => {
      const [tab, box] = [(await page.locator(`#add-topic-${id}`).boundingBox())!, (await row.boundingBox())!];
      return tab.x >= box.x - 1 && tab.x + tab.width <= box.x + box.width + 1;
    };
    await page.locator("#add-btn").click();
    await expect(page.locator("#add-topic-utilities")).toHaveAttribute("aria-selected", "true");
    await expect.poll(() => shown("utilities")).toBe(true);
    // Choosing a topic at the other end brings it back into the row.
    await page.locator("#add-topic-walking").click();
    await expect.poll(() => shown("walking")).toBe(true);
    await page.locator("#add-topic-utilities").click();
    await expect.poll(() => shown("utilities")).toBe(true);
  });

  test("undoing with an item focused keeps the focus on it as its greying changes", async ({ page }) => {
    // A pedestrian street has no lane a bus can stop in, so its stops are greyed.
    await transform(page, ".ped-item", "ped");
    await page.locator("#add-btn").click();
    await page.locator("#add-topic-transit").click();
    await expect(menuItem(page, "Curb stop")).toHaveAttribute("aria-disabled", "true");
    await menuItem(page, "Curb stop").focus();
    await page.keyboard.press("Control+z");
    await expect(live(page)).toHaveText(/^Undone\./);
    await expect(menuItem(page, "Curb stop")).not.toHaveAttribute("aria-disabled", "true");
    await expect(page.locator("#add-menu")).toBeVisible();
    await expect(menuItem(page, "Curb stop")).toBeFocused();
    // Among the search's matches too.
    await page.locator("#add-search").fill("curb stop");
    await menuItem(page, "Curb stop").focus();
    await page.keyboard.press("Control+y");
    await expect(menuItem(page, "Curb stop")).toHaveAttribute("aria-disabled", "true");
    await expect(menuItem(page, "Curb stop")).toBeFocused();
  });
});

test.describe("what the street editor keeps in the city", () => {
  test("is named for the street and says where it runs between", async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#street-name")).not.toHaveText("");
    await expect(page.locator("#street-sub")).not.toHaveText("");
    await expect(page.locator("a.back")).toHaveCount(0);
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
    await expect(page).toHaveTitle(/ entre .* et .* · CityLoom$/);
    await expect(page).toHaveTitle(new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} entre `));
    await page.locator('[data-lang="en"]').click();
    await expect(page).toHaveTitle(/between .* and .* · CityLoom/);
  });

  test("says which OpenStreetMap ways it was made from, and whether it has been changed", async ({ page }) => {
    await openStreet(page);
    await page.locator("#t-space").click();
    const block = page.locator("#title-block");
    await expect(block.locator('a[href^="https://www.openstreetmap.org/way/"]').first()).toBeVisible();
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
    await page.locator("#t-space").click();
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
    await expect(page).toHaveURL(/map\.html\?project=first$/);
  });

  test("the page without a street goes back to the map", async ({ page }) => {
    await page.goto("/street.html");
    await expect(page).toHaveURL(/map\.html\?project=first$/);
  });
});

test.describe("the Transform dialog", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("turns the motor space into the chosen kind, says so, and judges the street as a pedestrian one", async ({
    page,
  }) => {
    const before = await pieces(page);
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await page.locator("#transform-btn").click();
    await expect(page.locator("#transform-dialog")).toBeVisible();
    await page.locator("#transform-tab-ped").click();
    await page.locator("#transform-dialog .ped-item").first().click();
    await page.locator("#transform-apply").click();
    await expect(live(page)).toHaveText(/^Made pedestrian\. .+ is now a pedestrian street\.$/);
    // Without traffic the Make pedestrian group is gone; the button stays only while a transit measure fits.
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await page.locator("#transform-btn").click();
    await expect(page.locator("#transform-tab-ped")).toHaveCount(0);
    await page.keyboard.press("Escape");
    // The choice hands the focus to the drawing, whose keys edit the street.
    await page.locator("#t-checks").click();
    expect(await pieces(page)).toBe(before);
    await expect(page.locator("#checks")).toContainText("No roadway, so no sidewalk is asked for");
    await expect(page.locator("#checks")).toContainText(/unbroken/);
  });

  test("applying hands the focus to the drawing", async ({ page }) => {
    await transform(page, ".ped-item", "ped");
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await expect(page.locator("#wrap")).toBeFocused();
  });

  test("is one edit that undo takes back, and the street carries traffic again", async ({ page }) => {
    await transform(page, ".ped-item", "ped");
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/ carries traffic again\.$/);
    await expect(page.locator("#transform-btn")).toBeVisible();
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-ped").click();
    await expect(page.locator("#transform-dialog .ped-item")).toHaveCount(3);
  });

  test("the dialog closes with Escape and gives the focus back to its button", async ({ page }) => {
    await page.locator("#transform-btn").click();
    await expect(page.locator("#transform-dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await expect(page.locator("#transform-btn")).toBeFocused();
  });

  test("Cancel closes the dialog without changing the street and gives the focus back to its button", async ({
    page,
  }) => {
    await page.locator("#transform-btn").click();
    await page.locator("#transform-dialog .ped-item").first().click();
    await page.locator("#transform-cancel").click();
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await expect(page.locator("#transform-btn")).toBeFocused();
    await expect(page.locator("#undo")).toBeDisabled();
  });

  test("has a tab for making the street pedestrian, one for the transit measures that fit and one for the street templates", async ({
    page,
  }) => {
    await page.locator("#transform-btn").click();
    const dialog = page.locator("#transform-dialog");
    await expect(dialog).toHaveAttribute("aria-labelledby", "transform-dialog-h");
    await expect(dialog.getByRole("tab")).toHaveCount(3);
    await expect(dialog.getByRole("tab", { name: /Make pedestrian/ })).toBeVisible();
    await expect(dialog.getByRole("tab", { name: /Transit priority/ })).toBeVisible();
    await dialog.getByRole("tab", { name: /Street templates/ }).click();
    await expect(dialog.getByRole("option")).toHaveCount(5);
    await dialog.getByRole("tab", { name: /Make pedestrian/ }).click();
    await expect(dialog.getByRole("option")).toHaveCount(3);
    await dialog.getByRole("tab", { name: /Transit priority/ }).click();
    expect(await dialog.getByRole("option").count()).toBeGreaterThan(0);
    await expect(page.locator("#transform-btn")).toHaveAttribute("aria-haspopup", "dialog");
  });

  test("a transit measure is arranged as one edit, announced, and Undo takes it back", async ({ page }) => {
    const before = await pieces(page);
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-transit").click();
    const item = page.locator("#transform-dialog .transit-item").first();
    const code = (await item.locator(".dirtag").textContent()) ?? "";
    await item.click();
    await page.locator("#transform-apply").click();
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(live(page)).not.toBeEmpty();
    await expect(page.locator("#wrap")).toBeFocused();
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-transit").click();
    await expect(page.locator("#transform-dialog .transit-item", { hasText: code })).toHaveCount(0);
    await page.keyboard.press("Escape");
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/^Undone\./);
    expect(await pieces(page)).toBe(before);
  });

  test("pointing at a choice previews it and nothing changes until Apply", async ({ page }) => {
    const before = await pieceNames(page);
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-transit").click();
    await page.locator("#transform-dialog .transit-item").first().hover();
    const preview = page.locator("#transform-preview");
    await expect(preview.locator("figcaption", { hasText: "Now" })).toBeVisible();
    await expect(preview.locator("figcaption", { hasText: "After" })).toBeVisible();
    await expect(preview.locator(".transform-empty")).toHaveCount(0);
    await expect(page.locator("#undo")).toBeDisabled();
    expect(await pieceNames(page)).toEqual(before);
    await page.locator("#transform-cancel").click();
    await expect(page.locator("#undo")).toBeDisabled();
    expect(await pieceNames(page)).toEqual(before);
    // Chosen and applied, the street changes, in one step.
    await transform(page, ".transit-item", "transit");
    await expect(page.locator("#undo")).toBeEnabled();
    expect(await pieceNames(page)).not.toEqual(before);
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/^Undone\./);
    expect(await pieceNames(page)).toEqual(before);
  });

  for (const width of [1280, 700]) {
    test(`the options stay where they are when pointing at one shows its preview, at ${width} px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.locator("#transform-btn").click();
      await page.locator("#transform-tab-transit").click();
      const items = page.locator("#transform-dialog .transit-item");
      const y = async (i: number) => (await items.nth(i).boundingBox())!.y;
      const before = [await y(0), await y(1)];
      // A jump when the preview appears is the dialog growing, tens of pixels; a pixel is the one-column layout's rounding.
      const stays = async () => {
        const now = [await y(0), await y(1)];
        for (const i of [0, 1]) expect(Math.abs(now[i] - before[i])).toBeLessThanOrEqual(2);
      };
      await items.nth(1).hover();
      await expect(page.locator("#transform-preview .transform-empty")).toHaveCount(0);
      await stays();
      await items.nth(0).hover();
      await expect(page.locator("#transform-preview .transform-empty")).toHaveCount(0);
      await stays();
    });
  }

  test("is operated from the keyboard", async ({ page }) => {
    await page.locator("#transform-btn").focus();
    await page.keyboard.press("Enter");
    const list = page.locator("#transform-choices");
    await expect(list).toBeFocused();
    // The first choice is the list's active one as it takes the focus (so a screen reader names it), not chosen.
    await expect(list).toHaveAttribute("aria-activedescendant", "transform-opt-ped-sidewalk");
    await expect(page.locator("#transform-opt-ped-sidewalk")).toHaveAttribute("aria-selected", "false");
    await expect(page.locator("#transform-preview .transform-empty")).toBeVisible();
    await expect(page.locator("#transform-apply")).toBeDisabled();
    // Space chooses it, the arrows move and choose, and Enter applies the chosen one.
    await page.keyboard.press(" ");
    await expect(page.locator("#transform-opt-ped-sidewalk")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#transform-apply")).toBeEnabled();
    await page.keyboard.press("ArrowDown");
    await expect(list).not.toHaveAttribute("aria-activedescendant", "transform-opt-ped-sidewalk");
    await expect(page.locator("#transform-opt-ped-sidewalk")).toHaveAttribute("aria-selected", "false");
    await expect(page.locator("#transform-apply")).toBeEnabled();
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText(/^Made pedestrian\./);
    await expect(page.locator("#wrap")).toBeFocused();
  });

  test("fits a phone and shows its items in forced colours", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 780 });
    await page.emulateMedia({ forcedColors: "active" });
    await page.locator("#transform-btn").click();
    const box = await page.locator("#transform-dialog").boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(390);
    expect(box!.y + box!.height).toBeLessThanOrEqual(780);
    await expect(page.locator("#transform-dialog .transform-item").first()).toBeVisible();
    await expect(page.locator("#transform-apply")).toBeVisible();
  });

  test("on a phone, the preview of the last transit measure shows without scrolling past the list", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 780 });
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-transit").click();
    await page.locator("#transform-dialog .transit-item").last().click();
    await expect(page.locator("#transform-preview h3")).toBeInViewport();
  });

  test("the dialog fits a phone-width window", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 740 });
    await page.locator("#transform-btn").click();
    const dialog = page.locator("#transform-dialog");
    await expect(dialog).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375);
    expect(await dialog.evaluate((e) => e.scrollWidth <= e.clientWidth)).toBe(true);
    await page.locator("#transform-apply").scrollIntoViewIfNeeded();
    await expect(page.locator("#transform-apply")).toBeInViewport();
  });
});

// ---- changing part of a street: the plan, a stretch picked on it, the runs an edit makes ---------------------------

/** Where `atM` metres from the plan's left end lies on the screen, across the middle of its bands. */
const planAt = async (page: Page, atM: number) => {
  await page.locator("#plan").scrollIntoViewIfNeeded();
  const box = (await page.locator("#plan svg").boundingBox())!;
  const length = await streetLength(page);
  return { x: box.x + (atM / length) * box.width, y: box.y + 24 + 48 };
};

/** The stretch picked, as the line under the plan says it. */
const picked = (page: Page) => page.locator("#plan-picked");

test.describe("changing part of a street", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#plan svg")).toBeVisible();
  });

  test("the whole street is picked at first, and the plan's point is one tab stop under the drawing", async ({
    page,
  }) => {
    await expect(picked(page)).toHaveText(/^The whole street, [\d.]+ m long\.$/);
    await expect(page.locator("#plan")).toHaveAttribute("role", "group");
    await expect(page.locator("#plan-point")).toHaveAttribute("role", "slider");
    await expect(page.locator("#plan-point")).toHaveAttribute("aria-valuetext", /^The whole street/);
    await expect(page.locator("#plan [tabindex]")).toHaveCount(1);
  });

  test("a drag along the plan picks a stretch", async ({ page }) => {
    const a = await planAt(page, 10);
    const b = await planAt(page, 25);
    await page.mouse.move(a.x, a.y);
    await page.mouse.down();
    await page.mouse.move(b.x, b.y, { steps: 8 });
    await page.mouse.up();
    await expect(picked(page)).toHaveText(/^From 1\d\.\d m to 2\d\.\d m · 1\d\.\d m long$/);
    await expect(live(page)).toHaveText(/^From 1\d\.\d m to 2\d\.\d m, 1\d\.\d m long/);
    await expect(page.locator("#plan .plan-pick")).toHaveCount(1);
    await expect(page.locator("#plan .plan-handle")).toHaveCount(2);
    await expect(page.locator("#plan-point")).toBeFocused();
  });

  test("a click picks the run under the pointer", async ({ page }) => {
    await pickStretch(page, 10, 25);
    await expect(picked(page)).toHaveText("From 10.0 m to 25.0 m · 15.0 m long");
    const at = await planAt(page, 30);
    await page.mouse.click(at.x, at.y);
    await expect(picked(page)).toHaveText(/^The whole street/);
    await expect(live(page)).toHaveText(/^The whole street, [\d.]+ m long\./);
  });

  test("a stretch is picked by typing its ends in the details panel, with no drag", async ({ page }) => {
    const from = page.locator("#pick-from");
    const to = page.locator("#pick-to");
    await expect(from).toHaveValue("0.0");
    await from.click();
    await from.fill("6");
    await to.click();
    await to.fill("12");
    await page.keyboard.press("Enter");
    await expect(picked(page)).toHaveText("From 6.0 m to 12.0 m · 6.0 m long");
    await expect(live(page)).toHaveText("From 6.0 m to 12.0 m, 6.0 m long.");
    await expect(from).toHaveValue("6.0");
    // (The fixture street is about 40.4 m long, so every end here stays more than 5 m from its ends.)
    // Moved past its own end, one field at a time: From 30 waits, as typed, for To; nothing is swapped.
    await from.click();
    await from.fill("18");
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("From must come before To.");
    await expect(from).toHaveValue("18");
    await expect(picked(page)).toHaveText("From 6.0 m to 12.0 m · 6.0 m long");
    await to.click();
    await to.fill("25");
    await page.keyboard.press("Enter");
    await expect(picked(page)).toHaveText("From 18.0 m to 25.0 m · 7.0 m long");
    await expect(from).toHaveValue("18.0");
    // A stretch too short is refused, said, and what was typed stays to be put right.
    await to.fill("20");
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("A stretch must be at least 5.0 m long.");
    await expect(to).toHaveValue("20");
    await expect(picked(page)).toHaveText("From 18.0 m to 25.0 m · 7.0 m long");
  });

  test("Shift and the arrows pick a stretch from the keyboard, and the point says where it is on the way", async ({
    page,
  }) => {
    const point = page.locator("#plan-point");
    await point.focus();
    await page.keyboard.press("Home");
    await page.keyboard.press("PageDown");
    await expect(point).toHaveAttribute("aria-valuenow", "10");
    await expect(point).toHaveAttribute("aria-valuetext", "At 10.0 m");
    await page.keyboard.press("Shift+ArrowRight");
    await expect(point).toHaveAttribute("aria-valuetext", "At 11.0 m");
    for (let k = 0; k < 7; k++) await page.keyboard.press("Shift+ArrowRight");
    await expect(picked(page)).toHaveText("From 10.0 m to 18.0 m · 8.0 m long");
    await expect(point).toHaveAttribute("aria-valuetext", "From 10.0 m to 18.0 m, 8.0 m long.");
    await expect(live(page)).toHaveText("From 10.0 m to 18.0 m, 8.0 m long.");
    await page.keyboard.press("Escape");
    await expect(picked(page)).toHaveText(/^The whole street/);
  });

  test("an edit in the stretch makes a boundary the plan shows, and editing it back joins it", async ({ page }) => {
    await expect(planBoundaries(page)).toHaveCount(0);
    await pickStretch(page, 10, 25);
    await selectFirstPiece(page);
    await page.keyboard.press("-");
    await expect(planBoundaries(page)).toHaveCount(2);
    await expect(page.locator("#plan .plan-run")).toHaveCount(3);
    await page.keyboard.press("+");
    await expect(planBoundaries(page)).toHaveCount(0);
    await expect(page.locator("#plan .plan-run")).toHaveCount(1);
  });

  test("the Transform dialog previews and applies its choice to the picked stretch alone", async ({ page }) => {
    await pickStretch(page, 10, 25);
    await page.locator("#transform-btn").click();
    await page.locator("#transform-tab-ped").click();
    await page.locator("#transform-dialog .ped-item").first().hover();
    await expect(page.locator("#transform-preview figcaption", { hasText: "After" })).toBeVisible();
    await expect(planBoundaries(page), "nothing changes until Apply").toHaveCount(0);
    await page.locator("#transform-dialog .ped-item").first().click();
    await page.locator("#transform-apply").click();
    await expect(page.locator("#transform-dialog")).toBeHidden();
    await expect(planBoundaries(page), "the stretch alone changed").toHaveCount(2);
    await expect(page.locator("#plan .plan-run")).toHaveCount(3);
    await expect(picked(page)).toHaveText("From 10.0 m to 25.0 m · 15.0 m long");
    await page.locator("#undo").click();
    await expect(planBoundaries(page)).toHaveCount(0);
  });

  test("a reload keeps the stretches", async ({ page }) => {
    const tops = () =>
      page.locator("#drawing .iso-top").evaluateAll((els) => els.map((e) => e.getAttribute("points")).join("|"));
    await pickStretch(page, 10, 25);
    await selectFirstPiece(page);
    await page.keyboard.press("-");
    await expect(planBoundaries(page)).toHaveCount(2);
    const edited = await tops();
    const plan = await page.locator("#plan svg").innerHTML();
    await page.waitForTimeout(500); // the debounced write
    await page.reload();
    await expect(planBoundaries(page)).toHaveCount(2);
    await pickWhole(page);
    const runs = page.locator("#plan .plan-run");
    await expect(runs).toHaveCount(3);
    expect(await page.locator("#plan .plan-boundary").evaluateAll((b) => b.map((e) => e.outerHTML))).toEqual(
      [...plan.matchAll(/<g class="plan-boundary"[^]*?<\/g>/g)].map((m) => m[0]),
    );
    // The edited run, picked again, is drawn as it was.
    await pickRunAt(page, 15);
    await expect(picked(page)).toHaveText("From 10.0 m to 25.0 m · 15.0 m long");
    await expect.poll(tops).toBe(edited);
  });

  test("drags a boundary with the pointer as one undoable step, keeping the pointer captured", async ({ page }) => {
    await makeStretches(page, 2);
    const line = page.locator("#plan .plan-boundary-line");
    const before = await line.getAttribute("x1");
    const box = (await line.boundingBox())!;
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    // the drawing that holds the pointer must survive the redraws of a drag
    await page.locator("#plan .plan-canvas").evaluate((el) => ((window as any).__canvas = el));
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 20, y, { steps: 4 });
    await page.mouse.move(x + 40, y, { steps: 4 });
    const state = await page.evaluate(() => {
      const el = (window as any).__canvas as HTMLElement;
      return { connected: el.isConnected, captured: el.hasPointerCapture(1) };
    });
    expect(state, "the drawing is kept and holds the pointer").toEqual({ connected: true, captured: true });
    await expect(line).not.toHaveAttribute("x1", before!);
    await page.mouse.up();
    const dragged = await line.getAttribute("x1");
    expect(Number(dragged)).toBeGreaterThan(Number(before));
    await page.locator("#undo").click();
    await expect(line).toHaveAttribute("x1", before!);
    await expect(planBoundaries(page)).toHaveCount(1);
  });

  test("a boundary clicked on the plan is moved by typing where it goes, with no drag", async ({ page }) => {
    await makeStretches(page, 2);
    const line = page.locator("#plan .plan-boundary-line");
    const before = await line.getAttribute("x1");
    const box = (await line.boundingBox())!;
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await expect(live(page)).toHaveText(/^The change at [\d.]+ m; type where it goes\.$/);
    await expect(picked(page)).toHaveText(/^The whole street/);
    const field = page.locator("#boundary-to");
    const at = Number(await field.inputValue());
    await field.click();
    await field.fill(String(at + 3));
    await page.keyboard.press("Enter");
    await expect(line).not.toHaveAttribute("x1", before!);
    await expect(live(page)).toHaveText(new RegExp(`Move a change along the street to ${at + 3}\\.0 m`));
    await expect(field).toHaveValue(`${at + 3}.0`);
    await page.keyboard.press("Escape");
    await expect(field).toHaveCount(0);
    await page.locator("#undo").click();
    await expect(line).toHaveAttribute("x1", before!);
  });

  test("Escape cancels a drag", async ({ page }) => {
    await makeStretches(page, 2);
    const line = page.locator("#plan .plan-boundary-line");
    const before = await line.getAttribute("x1");
    const box = (await line.boundingBox())!;
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 30, y, { steps: 6 });
    await expect(line).not.toHaveAttribute("x1", before!);
    await page.keyboard.press("Escape");
    await page.mouse.up();
    await expect(line).toHaveAttribute("x1", before!);
  });

  test("Alt and the arrows move the boundary at the pick's end", async ({ page }) => {
    const url = page.url();
    await makeStretches(page, 2);
    const line = page.locator("#plan .plan-boundary-line");
    const before = await line.getAttribute("x1");
    const half = Math.floor((await streetLength(page)) / 2);
    await pickToEnd(page, half);
    await page.keyboard.press("Alt+ArrowRight");
    await expect(line).not.toHaveAttribute("x1", before!);
    await expect(picked(page)).toHaveText(new RegExp(`^From ${half + 1}\\.0 m to`));
    expect(page.url(), "Alt+Arrow is not the browser's Back").toBe(url);
    await page.locator("#undo").click();
    await expect(line).toHaveAttribute("x1", before!);
  });

  test("the plan at 375 px wide makes the page scroll no further sideways", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    await expect(page.locator("#plan svg")).toBeVisible();
    const m = await page.evaluate(() => ({
      page: document.documentElement.scrollWidth,
      inner: innerWidth,
      plan: document.getElementById("plan")!.getBoundingClientRect().right,
    }));
    expect(m.page).toBeLessThanOrEqual(m.inner);
    expect(m.plan).toBeLessThanOrEqual(m.inner);
  });

  /** Picks the street's second half and narrows its first piece there: the second half is a run of its own, picked. */
  const isoAcrossBoundary = async (page: Page) => {
    await pickToEnd(page, Math.floor((await streetLength(page)) / 2));
    await selectFirstPiece(page);
    await page.keyboard.press("-");
    await expect(planBoundaries(page)).toHaveCount(1);
    await expect(page.locator("#drawing.iso")).toBeAttached();
  };

  test("the 2.5D shows the run where the pick starts, with its neighbour across a boundary", async ({ page }) => {
    await isoAcrossBoundary(page);
    const drawing = page.locator("#drawing");
    await expect(drawing.locator(".iso-section")).toHaveCount(2);
    await expect(drawing.locator(".iso-across").first()).toBeAttached();
    expect(await drawing.locator(".iso-across [data-uid]").count(), "the neighbour's pieces are not pressable").toBe(0);
    // The shown stretch is the street's second half, from the boundary the label is centred on.
    await expect(drawing).toHaveAttribute(
      "aria-label",
      /centred on ([\d.]+ m), where the section changes abruptly: the stretch from 0\.0 m to \1 is in front and the one from \1 to [\d.]+ m beyond\. The stretch from \1 to [\d.]+ m has /,
    );
    const second = await drawing.innerHTML();
    await pickRunAt(page, 1);
    await expect(drawing).toHaveAttribute(
      "aria-label",
      /where the section changes abruptly: the stretch from 0\.0 m to ([\d.]+ m) is in front .* The stretch from 0\.0 m to \1 has /,
    );
    await expect(drawing.locator(".iso-section")).toHaveCount(2);
    const first = await drawing.innerHTML();
    expect(first, "the across run swaps with the run shown").not.toBe(second);
    await pickRunAt(page, Math.floor((await streetLength(page)) / 2) + 1);
    await expect(drawing).toHaveAttribute(
      "aria-label",
      /centred on ([\d.]+ m), where the section changes abruptly: the stretch from 0\.0 m to \1 is in front and the one from \1 to [\d.]+ m beyond\. The stretch from \1 to [\d.]+ m has /,
    );
    expect(await drawing.innerHTML(), "the second run's own pieces again").not.toBe(first);
  });

  /** Adds a driving lane to the street's second half: the first run hosts the transition, which leads into the second. */
  const addLaneInSecondHalf = async (page: Page) => {
    await pickToEnd(page, Math.floor((await streetLength(page)) / 2));
    await addFromMenu(page, "Driving lane");
    await expect(planBoundaries(page)).toHaveCount(1);
  };

  test("the 2.5D says how long the transition is that leads into the run beyond", async ({ page }) => {
    await addLaneInSecondHalf(page);
    await expect(page.locator("#drawing.iso")).toBeAttached();
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /centred on ([\d.]+ m), .* A transition of [\d.]+ m leads into the stretch from \1 to [\d.]+ m\./,
    );
    await expect(page.locator("#drawing .iso-taper").first()).toBeAttached();
    await expect(page.locator("#plan .plan-taper")).toHaveCount(1);
  });

  test("the checks say a run is too short for its transition", async ({ page }) => {
    await addLaneInSecondHalf(page);
    await page.locator("#t-checks").click();
    const row = page.locator("#checks li", { hasText: "Room for the transition" });
    await expect(row).toBeVisible();
    await expect(row).toContainText(/needs .* but the stretch from 0\.0 m to [\d.]+ m is/);
  });

  test("a 2.5D with no boundary near is not split in two", async ({ page }) => {
    await expect(planBoundaries(page)).toHaveCount(0);
    await expect(page.locator("#drawing.iso")).toBeAttached();
    await expect(page.locator("#drawing .iso-section")).toHaveCount(0);
    await expect(page.locator("#drawing .iso-across")).toHaveCount(0);
  });

  test("the 2.5D label at a boundary is in French when the page is", async ({ page }) => {
    await isoAcrossBoundary(page);
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /centré sur [\d,]+\sm, là où la coupe change brusquement\s: le tronçon de [\d,]+\sm à [\d,]+\sm est au premier plan/,
    );
  });

  test("the plan is in French when the page is", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("#plan")).toHaveAttribute("aria-label", /^La rue de sa première extrémité à sa dernière/);
    await expect(picked(page)).toHaveText(/^Toute la rue, /);
  });
});

test("a street cut at both ends says a stretch of it cannot be picked", async ({ page }) => {
  await page.route("**/data/metro/0_0.json", (route) => route.fulfill(tileFrom(OPEN_ENDS_TILE)));
  // the tile holds one road and no junction, so the map's places are its one street
  await page.goto("/map.html");
  await mapReady(page);
  const hot = await page.evaluate(
    () => (window as any).cityloomMap.querySourceFeatures("places").map((f: any) => String(f.properties.hot))[0],
  );
  expect(hot).toMatch(/^s-\d+$/);
  await page.goto(`/street.html?street=${hot.slice(2)}`);
  await expect(page.locator("#plan")).toBeVisible();
  // Enter at the point picks the run there: refused, the street having no end to measure from.
  await page.locator("#plan-point").focus();
  await page.keyboard.press("Enter");
  await expect(live(page)).toHaveText(
    "Both ends of this street are cut by the edge of the map, so it can only be changed along its whole length.",
  );
  await expect(planBoundaries(page)).toHaveCount(0);
});

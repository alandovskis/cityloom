import type { Page } from "@playwright/test";

import { expect, live, openStreet, placesOnMap, test } from "./fixtures";

const names = (page: Page) => page.locator("#piece-list .piece-name").allTextContents();
const template = (page: Page, id: string) => page.locator(`#transform-menu [data-template="${id}"]`);

// Street templates (#65): ready-made cross-sections in the Transform menu. Applying one lays out ordinary pieces in
// one undoable edit; one that does not suit the street stays listed, disabled, with its reason.

test.describe("street templates", () => {
  test("all five are listed in the Transform menu under their own group", async ({ page }) => {
    await openStreet(page);
    await page.locator("#transform-btn").click();
    const group = page.locator("#transform-menu").getByRole("group", { name: "Street templates" });
    await expect(group.getByRole("menuitem")).toHaveText([
      "Commercial main street",
      "School street",
      "Neighbourhood greenway",
      "Bus corridor",
      "Industrial access street",
    ]);
  });

  test("applying one lays out ordinary pieces as a single edit that Undo and Start over take back", async ({
    page,
  }) => {
    await openStreet(page);
    const before = await names(page);
    await page.locator("#transform-btn").click();
    const school = template(page, "school");
    await expect(school).not.toHaveAttribute("aria-disabled", "true");
    await school.click();
    await expect(page.locator("#wrap")).toBeFocused();
    await expect(live(page)).not.toBeEmpty();
    await expect(page.locator("#undo")).toBeEnabled();
    expect(await names(page)).toEqual(["Sidewalk", "Planting strip", "Driving lane", "Planting strip", "Sidewalk"]);
    // The result is ordinary: a piece can be selected like any other.
    await page.locator("#piece-list [role=option]").nth(1).click();
    await expect(page.locator("#piece-list [role=option]").nth(1)).toHaveAttribute("aria-selected", "true");
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText(/^Undone\./);
    expect(await names(page)).toEqual(before);
    // And Start over restores the street after a template.
    await page.locator("#transform-btn").click();
    await template(page, "greenway").click();
    await page.locator("#reset").click();
    expect(await names(page)).toEqual(before);
  });

  test.describe("on a street too narrow for one", () => {
    test.describe.configure({ timeout: 120_000 });

    test("a template that does not fit is disabled, says why, and announces it instead of applying", async ({
      page,
    }) => {
      await page.goto("/map.html");
      const streets = (await placesOnMap(page)).streets;
      let found = false;
      for (const href of streets) {
        await page.goto("/" + href);
        await expect(page.locator("#transform-btn")).toBeVisible();
        if ((await page.locator('#transform-menu [role=menuitem][aria-disabled="true"]').count()) > 0) {
          found = true;
          break;
        }
      }
      expect(found, "a street of the Plateau tile is too narrow for some template").toBe(true);
      const before = await names(page);
      await page.locator("#transform-btn").click();
      const item = page.locator('#transform-menu [role=menuitem][aria-disabled="true"]').first();
      const reasonId = await item.getAttribute("aria-describedby");
      expect(reasonId).toMatch(/^template-reason-/);
      await expect(page.locator(`#${reasonId}`)).toBeVisible();
      await expect(page.locator(`#${reasonId}`)).toContainText(/needs a street at least .+ wide|wider than/);
      // It is still reached with the arrows.
      await page.keyboard.press("End");
      await expect(page.locator("#transform-menu [role=menuitem]").last()).toBeFocused();
      const reason = (await page.locator(`#${reasonId}`).textContent()) ?? "";
      // Playwright's click refuses an aria-disabled item, as it should a disabled control: choose it by the key.
      await item.focus();
      await page.keyboard.press("Enter");
      await expect(live(page)).toHaveText(reason);
      await expect(page.locator("#undo")).toBeDisabled();
      expect(await names(page)).toEqual(before);
    });
  });
});

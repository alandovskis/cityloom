import type { Page } from "@playwright/test";

import {
  expect,
  live,
  openAnchoredStreet,
  openStreet,
  openStreetWithUnfitTemplate,
  pickRunAt,
  planBoundaries,
  showStreetLevel,
  test,
  transform,
} from "./fixtures";

const names = (page: Page) => page.locator("#piece-list .piece-name").allTextContents();
const template = (page: Page, id: string) => page.locator(`#transform-dialog [data-template="${id}"]`);

/** Gives the street a speed limit above a school zone's, which the Plateau's streets may not have; the Undo it
 *  leaves is the first of the street's history. */
async function fasterThanSchool(page: Page) {
  await showStreetLevel(page);
  await page.locator("#speed-limit").selectOption("50");
}

// Street templates (#65): ready-made cross-sections in the Transform dialog. Applying one lays out ordinary pieces in
// one undoable edit; one that does not suit the street stays listed, disabled, with its reason.

test.describe("street templates", () => {
  test("all five are listed in the Transform dialog under their own tab", async ({ page }) => {
    await openStreet(page);
    await page.locator("#transform-btn").click();
    await page.getByRole("tab", { name: /Street templates/ }).click();
    await expect(page.locator("#transform-dialog .template-item b")).toHaveText([
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
    await page.getByRole("tab", { name: /Street templates/ }).click();
    const school = template(page, "school");
    await expect(school).not.toHaveAttribute("aria-disabled", "true");
    await school.click();
    await page.locator("#transform-apply").click();
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
    await transform(page, '[data-template="greenway"]', "templates");
    await page.locator("#reset").click();
    expect(await names(page)).toEqual(before);
  });

  test("the School street on a street too short for a separate zone makes the whole street one and says so", async ({
    page,
  }) => {
    await openStreet(page);
    await fasterThanSchool(page);
    await transform(page, '[data-template="school"]', "templates");
    await expect(live(page)).toContainText("The speed limit there is 30 km/h.");
    await expect(live(page)).toContainText("No separate school zone was made");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  test.describe("the School street's school zone", () => {
    test.describe.configure({ timeout: 120_000 });

    test("is a zone with a lower speed and a raised table, made in one step that Undo takes back", async ({ page }) => {
      await openAnchoredStreet(page, 80);
      const boundaries = planBoundaries(page);
      const table = page.locator("#calming-strip").locator('[role="option"]');
      await expect(boundaries).toHaveCount(0);
      await fasterThanSchool(page);
      await transform(page, '[data-template="school"]', "templates");

      await expect(boundaries).toHaveCount(2);
      await expect(page.locator("#plan-picked"), "the school stretch is picked").toHaveText(/^From /);
      await expect(table).toHaveCount(3);
      await expect(table.nth(1)).toHaveAttribute("aria-label", /Raised table/);
      await expect(live(page)).toContainText("The school zone runs from");
      await expect(live(page)).toContainText("The speed limit there is 30 km/h.");
      await expect(live(page)).toContainText("A raised table is at");

      // The stretch's speed is the Street level's, for the picked stretch; the rest of the street follows the street.
      await showStreetLevel(page);
      await expect(page.locator("#speed-limit")).toHaveValue("30");
      await pickRunAt(page, 1);
      await expect(page.locator("#speed-limit")).toHaveValue("street");

      // One step: a single Undo takes the zones, the speed and the table back.
      await page.locator("#undo").click();
      await expect(boundaries).toHaveCount(0);
      await expect(table).toHaveCount(1);
      await showStreetLevel(page);
      await expect(page.locator("#speed-limit")).toHaveValue("50");
    });
  });

  test.describe("on a street too narrow for one", () => {
    test.describe.configure({ timeout: 120_000 });

    test("a template that does not fit is disabled, says why, previews its reason and cannot be applied", async ({
      page,
    }) => {
      await openStreetWithUnfitTemplate(page);
      const before = await names(page);
      const item = page.locator('#transform-dialog [role=option][aria-disabled="true"]').first();
      const reason = item.locator(".template-reason");
      await expect(reason).toHaveAttribute("id", /^template-reason-/);
      await expect(reason).toBeVisible();
      await expect(reason).toContainText(/needs a street at least .+ wide|wider than/);
      // The option is named by what it shows: its name and why it does not fit.
      await expect(item).toHaveAccessibleName(/needs a street at least .+ wide|wider than/);
      // Pointing at it previews its reason, and Apply stays disabled even when it is chosen.
      await item.hover();
      await expect(page.locator("#transform-preview .transform-no")).toContainText(
        /needs a street at least .+ wide|wider than/,
      );
      await expect(page.locator("#transform-apply")).toBeDisabled();
      // Playwright refuses to click an aria-disabled option, as it should a disabled control: press it anyway.
      await item.click({ force: true });
      await expect(page.locator("#transform-apply")).toBeDisabled();
      // The arrows still reach the list's last choice.
      await page.locator("#transform-choices").focus();
      await page.keyboard.press("End");
      await expect(page.locator("#transform-choices")).toHaveAttribute("aria-activedescendant", /^transform-opt-/);
      await page.locator("#transform-cancel").click();
      await expect(page.locator("#undo")).toBeDisabled();
      expect(await names(page)).toEqual(before);
    });
  });
});

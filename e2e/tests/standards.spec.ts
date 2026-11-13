import type { Page } from "@playwright/test";

import { expect, openStreet, test } from "./fixtures";

// Project-specific design standards (#79): a profile of local thresholds, kept in the browser, that the street's checks
// read in place of the built-in ones and name when they do.

const section = (page: Page) => page.locator(".standards");
const checkNamed = (page: Page, name: string) => page.locator("#checks li").filter({ hasText: name });

async function openProfiles(page: Page) {
  await openStreet(page);
  await expect(page.locator("#drawing")).toBeVisible();
  await page.locator("#t-street").click();
  await expect(section(page)).toBeVisible();
}

async function createProfile(page: Page, name: string) {
  await page.locator("#standards-name").fill(name);
  await page.locator("#standards-create").click();
  await expect(page.locator("#standards-edit-form")).toBeVisible();
}

test.describe("standards profiles on the street page", () => {
  test.beforeEach(async ({ page }) => {
    await openProfiles(page);
  });

  test("with no profile the checks name no standard and the built-in standards are in force", async ({ page }) => {
    await expect(page.locator("#standards-active")).toHaveText("The checks use the built-in standards.");
    await expect(page.locator("#standards-in-force")).toHaveValue("");
    await page.locator("#t-checks").click();
    await expect(checkNamed(page, "Room for emergency vehicles")).toBeVisible();
    await expect(page.locator("#checks .check-standard")).toHaveCount(0);
  });

  test("a profile is created, edited, chosen, and the access check names it with both figures", async ({ page }) => {
    await createProfile(page, "City of X");
    await page.locator("#standards-access-lane").fill("3.5");
    await page.locator("#standards-save").click();
    await expect(page.locator("#standards-access-lane")).toHaveValue(/^3\.5/);
    await expect(page.locator("#standards-error")).toBeHidden();

    await page.locator("#standards-in-force").selectOption({ label: "City of X" });
    await expect(page.locator("#standards-active")).toHaveText('The checks use the standard "City of X".');
    await page.locator("#t-checks").click();
    const access = checkNamed(page, "Room for emergency vehicles");
    await expect(access.locator(".check-standard")).toHaveText("Standard City of X: 3.5 m (built-in 3.0 m)");
    await expect(access).toContainText("3.5 m or more");

    // The choice is the browser's: it is still in force when the page is opened again.
    await page.reload();
    await page.locator("#t-street").click();
    await expect(page.locator("#standards-in-force")).toHaveValue(/.+/);
    await page.locator("#t-checks").click();
    await expect(checkNamed(page, "Room for emergency vehicles").locator(".check-standard")).toContainText("City of X");
  });

  test("an invalid figure is refused with its range and nothing is kept", async ({ page }) => {
    await createProfile(page, "Bad figures");
    await page.locator("#standards-access-lane").fill("0.5");
    await page.locator("#standards-save").click();
    await expect(page.locator("#standards-error")).toHaveText(
      "Emergency access lane width must be between 2.0 m and 6.0 m.",
    );
    await page.locator("#standards-access-lane").fill("far");
    await page.locator("#standards-save").click();
    await expect(page.locator("#standards-error")).toHaveText("That is not a number.");
    await page.locator("#standards-access-lane").fill("3");
    await page.locator("#standards-rename").fill("   ");
    await page.locator("#standards-save").click();
    await expect(page.locator("#standards-error")).toHaveText("Give the profile a name.");
  });

  test("a looser figure is allowed and shown as a warning", async ({ page }) => {
    await createProfile(page, "Loose");
    await page.locator("#standards-access-lane").fill("2.5");
    await page.locator("#standards-save").click();
    await expect(page.locator("#standards-warnings")).toContainText(
      "Emergency access lane width: 2.5 m is looser than the built-in 3.0 m.",
    );
  });

  test("a target speed adds a check for the street's class that names the standard", async ({ page }) => {
    await createProfile(page, "Calm streets");
    for (const cls of ["motorway", "arterial", "collector", "local", "pedestrian"]) {
      await page.locator(`#standards-target-${cls}`).fill("5");
    }
    await page.locator("#standards-save").click();
    await page.locator("#standards-in-force").selectOption({ label: "Calm streets" });
    await page.locator("#t-checks").click();
    const target = checkNamed(page, "Speed limit within the target for its class");
    await expect(target).toBeVisible();
    await expect(target.locator(".check-standard")).toContainText("Standard Calm streets: target 5 km/h");
  });

  test("a piece's width range names the standard that sets it", async ({ page }) => {
    await createProfile(page, "Widths");
    await page.locator("#standards-width-kind").selectOption("travel");
    await page.locator("#standards-width-min").fill("2.8");
    await page.locator("#standards-width-max").fill("3.4");
    await page.locator("#standards-width-set").click();
    await expect(page.locator("#standards-widths")).toContainText("2.8 m to 3.4 m");
    await page.locator("#standards-in-force").selectOption({ label: "Widths" });
    await page
      .locator("#piece-list .piece-cell")
      .filter({ hasText: /Driving lane/ })
      .first()
      .click();
    await expect(page.locator("#i-width-range")).toContainText("Standard Widths: 2.8 m to 3.4 m");
  });

  test("a profile is exported as a file, imported again, and removed after a question", async ({ page }) => {
    await createProfile(page, "City of X");
    await page.locator("#standards-access-lane").fill("3.5");
    await page.locator("#standards-save").click();
    await page.locator("#standards-close").click();
    await page.locator("[id^=standards-export-]").first().click();
    const file = await page.locator("#standards-exported").inputValue();
    expect(JSON.parse(file)).toMatchObject({ schema: "cityloom-standards", schema_version: 1, name: "City of X" });
    await expect(page.locator("#standards-download")).toHaveAttribute("download", "city-of-x.json");

    // The same name is refused, like creating it again.
    await page.locator("#standards-import-text").fill(file);
    await page.locator("#standards-import").click();
    await expect(page.locator("#standards-error")).toHaveText("Another profile already has that name.");
    await page.locator("#standards-import-text").fill(file.replace("City of X", "City of Y"));
    await page.locator("#standards-import").click();
    await expect(page.locator("#standards-list li")).toHaveCount(2);
    await page.locator("#standards-import-text").fill(file.replace("City of X", "City of Z").replace("3500", "100"));
    await page.locator("#standards-import").click();
    await expect(page.locator("#standards-error")).toContainText("must be between 2.0 m and 6.0 m");

    // Removing asks first; the built-in standards are back when the profile in force goes.
    await page.locator("#standards-in-force").selectOption({ label: "City of Y" });
    await page.locator("[id^=standards-remove-]").filter({ hasText: "Remove" }).last().click();
    await expect(page.locator(".standards-ask")).toContainText('Remove the profile "City of Y"?');
    await page.locator("#standards-remove-no").click();
    await expect(page.locator("#standards-list li")).toHaveCount(2);
    await page.locator("[id^=standards-remove-]").filter({ hasText: "Remove" }).last().click();
    await page.locator("#standards-remove-yes").click();
    await expect(page.locator("#standards-list li")).toHaveCount(1);
    await expect(page.locator("#standards-active")).toHaveText("The checks use the built-in standards.");
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks .check-standard")).toHaveCount(0);
  });
});

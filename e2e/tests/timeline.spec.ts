import type { Page } from "@playwright/test";

import { expect, live, test } from "./fixtures";

// The timeline page: milestones of a project's work, saved by name, compared with the work now, restored after a
// question and deleted. The first project is every test's own (fixtures.ts); its edits are `cityloom-edits@first`.

const EDITS_KEY = "cityloom-edits@first";

/** Keeps streets `[key, name, width]` as the first project's edits, as a page of the project would. */
async function keepStreets(page: Page, streets: [string, string, number][]) {
  await page.evaluate(
    ({ key, streets }) => {
      // Stamped with the importer version, as a saved record is: one without it is stale and discarded.
      const doc = { version: 17, streets: {} as Record<string, unknown> };
      for (const [k, name, width] of streets)
        doc.streets[k] = { ways: [1], import: 11, street: { name, row_mm: width } };
      localStorage.setItem(key, JSON.stringify(doc));
    },
    { key: EDITS_KEY, streets },
  );
}

/** The edits as kept, read as data: a restore writes the document again, so its text may be ordered differently. */
const edits = (page: Page) => page.evaluate((key) => JSON.parse(localStorage.getItem(key) ?? "null"), EDITS_KEY);

const rows = (page: Page) => page.locator("li.timeline-row");

test.describe("the timeline page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/timeline.html?project=first");
    await expect(page.locator("#timeline-heading")).toBeVisible();
  });

  test("names the project, starts empty and refuses a milestone with no name", async ({ page }) => {
    await expect(page).toHaveTitle("CityLoom timeline");
    await expect(page.locator("h1")).toHaveText("Timeline of First project");
    await expect(page.locator(".timeline-empty")).toContainText("No milestones yet");
    await expect(page.getByRole("link", { name: "Back to the map" })).toHaveAttribute("href", "map.html?project=first");
    await page.getByRole("button", { name: "Save milestone" }).click();
    await expect(page.locator("#timeline-create-error")).toHaveText("Give the milestone a name.");
    await expect(page.locator("#timeline-name")).toHaveAttribute("aria-invalid", "true");
    await expect(page.locator("#timeline-name")).toBeFocused();
    await expect(rows(page)).toHaveCount(0);
  });

  test("saves a milestone of the work now, which stays over a reload", async ({ page }) => {
    await keepStreets(page, [
      ["w1", "Rue A", 9000],
      ["w2", "Rue B", 9000],
    ]);
    await page.getByLabel("Milestone name").fill("First design");
    await page.keyboard.press("Enter");
    await expect(rows(page)).toHaveCount(1);
    await expect(live(page)).toHaveText("Milestone First design saved.");
    const row = page.getByRole("listitem", { name: /First design/ });
    await expect(row).toContainText("2 streets, 0 junctions, 0 rules");
    await expect(row).toContainText("CityLoom ");
    await expect(row).toContainText("UTC");
    await page.reload();
    await expect(rows(page)).toHaveCount(1);
  });

  test("compares a milestone with the work now by place and by what changed", async ({ page }) => {
    await keepStreets(page, [
      ["w1", "Rue A", 9000],
      ["w2", "Rue B", 9000],
    ]);
    await page.getByLabel("Milestone name").fill("Base");
    await page.keyboard.press("Enter");
    await expect(rows(page)).toHaveCount(1);
    await page.getByRole("button", { name: "Compare with now: Base" }).click();
    await expect(page.locator(".timeline-compare-summary")).toHaveText("The work is the same as in Base.");
    await page.getByRole("button", { name: "Close the comparison" }).click();
    await expect(page.getByRole("button", { name: "Compare with now: Base" })).toBeFocused();

    await keepStreets(page, [
      ["w1", "Rue A", 12000],
      ["w3", "Rue C", 9000],
    ]);
    await page.getByRole("button", { name: "Compare with now: Base" }).click();
    const compare = page.getByRole("region", { name: "Since Base" });
    await expect(compare).toContainText("3 places differ from Base.");
    await expect(compare.locator(".timeline-diff > li")).toHaveText([
      /Changed\s+Rue A\s*width/,
      /Removed since\s+Rue B/,
      /Added since\s+Rue C/,
    ]);
  });

  test("restores after a question that Escape answers no, and keeps the work it replaces", async ({ page }) => {
    await keepStreets(page, [["w1", "Rue A", 9000]]);
    const original = await edits(page);
    await page.getByLabel("Milestone name").fill("Base");
    await page.keyboard.press("Enter");
    await expect(rows(page)).toHaveCount(1);
    await keepStreets(page, [
      ["w1", "Rue A", 12000],
      ["w2", "Rue B", 9000],
    ]);
    const edited = await edits(page);

    await page.getByRole("button", { name: "Restore Base" }).click();
    const question = page.getByRole("group", { name: /Put the work back as it was in Base/ });
    await expect(question).toBeVisible();
    await expect(page.locator("#timeline-restore-safe")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(question).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Restore Base" })).toBeFocused();
    expect(await edits(page)).toEqual(edited);

    await page.getByRole("button", { name: "Restore Base" }).click();
    await page.getByRole("button", { name: "Restore this milestone" }).click();
    await expect(live(page)).toHaveText("Restored Base. The earlier work is saved as Before restoring Base.");
    expect(await edits(page)).toEqual(original);
    await expect(rows(page)).toHaveCount(2);
    const backup = page.getByRole("listitem", { name: /Before restoring Base/ });
    await expect(backup).toContainText("Saved before a restore");

    // Restoring the backup undoes the restore.
    await page.getByRole("button", { name: "Restore Before restoring Base" }).click();
    await page.getByRole("button", { name: "Restore this milestone" }).click();
    expect(await edits(page)).toEqual(edited);
  });

  test("deletes a milestone after a question and leaves the work alone", async ({ page }) => {
    await keepStreets(page, [["w1", "Rue A", 9000]]);
    const work = await edits(page);
    await page.getByLabel("Milestone name").fill("Base");
    await page.keyboard.press("Enter");
    await page.getByRole("button", { name: "Delete Base" }).click();
    await page.getByRole("button", { name: "Cancel" }).click();
    await expect(rows(page)).toHaveCount(1);
    await page.getByRole("button", { name: "Delete Base" }).click();
    await page.getByRole("button", { name: "Delete the milestone" }).click();
    await expect(rows(page)).toHaveCount(0);
    await expect(live(page)).toHaveText("Milestone Base deleted.");
    await expect(page.locator("#timeline-heading")).toBeFocused();
    expect(await edits(page)).toEqual(work);
  });

  test("is reached from the project's row on the home page and speaks French", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: "Timeline of First project" }).click();
    await expect(page).toHaveURL(/timeline\.html\?project=first$/);
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("h1")).toHaveText("Chronologie de First project");
    await expect(page).toHaveTitle("Chronologie CityLoom");
    await expect(page.getByRole("button", { name: "Enregistrer le jalon" })).toBeVisible();
  });

  test("a project that is not here goes to the list", async ({ page }) => {
    await page.goto("/timeline.html?project=nope");
    await expect(page).toHaveURL(/index\.html\?missing=1$/);
  });
});

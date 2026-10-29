import type { Page } from "@playwright/test";

import { expect, live, placesOnMap, streetLengthM, test } from "./fixtures";

// Street trees and green infrastructure: the tools, the Green row, the checks, the 2.5D and the saved document.

// Walking the streets of the city is slow on CI (about twice as slow as here), so each test has room.
test.describe.configure({ timeout: 120_000 });

const SIGHTLINE = "Planting clear of the corners at junctions";

/** Opens the first street of the city that is at least `minM` metres long and has a sidewalk to put a tree on. */
async function openGreenStreet(page: Page, minM: number) {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  for (const href of streets) {
    await page.goto("/" + href);
    await expect(page.locator("#zone-0")).toBeVisible();
    if ((await streetLengthM(page)) < minM) continue;
    if ((await page.locator("#green-add").count()) === 0) continue;
    const hosts = await page.locator("#green-host option").allTextContents();
    if (hosts.some((h) => /^Sidewalk/.test(h))) return;
  }
  throw new Error(`no street of ${minM} m or more with a sidewalk and the tree tools`);
}

/** Puts a tree `metres` from the start of the street, on the sidewalk, with the form's defaults across. */
async function addTree(page: Page, metres: number) {
  await page.locator("#green-kind").selectOption({ label: "Tree" });
  await page.locator("#green-at").fill(String(metres));
  const sidewalk = page
    .locator("#green-host option")
    .filter({ hasText: /^Sidewalk/ })
    .first();
  await page.locator("#green-host").selectOption((await sidewalk.getAttribute("value"))!);
  await page.locator("#green-add").click();
}

/**
 * Opens the first street of the city whose start is a junction, which is found rather than assumed: a tree 5 m from the
 * start of each street (that is long enough, with a sidewalk) is tried, and the first whose sightline row fails is the
 * one. It is left open with that tree on it, and its row is returned.
 */
async function openStreetWithAJunctionAtItsStart(page: Page) {
  await page.goto("/map.html");
  for (const href of (await placesOnMap(page)).streets) {
    await page.goto("/" + href);
    await expect(page.locator("#zone-0")).toBeVisible();
    if ((await streetLengthM(page)) < 30 || (await page.locator("#green-add").count()) === 0) continue;
    const hosts = await page.locator("#green-host option").allTextContents();
    if (!hosts.some((h) => /^Sidewalk/.test(h))) continue;
    await addTree(page, 5);
    await page.locator("#t-checks").click();
    const row = page.locator("#checks li").filter({ hasText: SIGHTLINE });
    if ((await row.count()) === 1 && !(await row.evaluate((li) => li.classList.contains("ok")))) return row;
    if ((await page.locator("#green-strip").count()) > 0) await page.locator("#undo").click();
  }
  throw new Error("no street of 30 m or more with a sidewalk and a junction at its start");
}

const editsVersion = (page: Page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? "{}").version);

const crowns = (page: Page) => page.locator("#drawing .iso-solid.s-crown");

test.describe("street trees and green infrastructure", () => {
  test("a tree is added on a sidewalk, drawn in 2.5D, kept over a reload and undone", async ({ page }) => {
    await openGreenStreet(page, 30);
    await expect(page.locator("#green-strip")).toHaveCount(0);
    await addTree(page, 15);

    const cells = page.locator("#green-strip").locator('[role="option"]');
    await expect(cells).toHaveCount(1);
    await expect(cells.first()).toHaveAttribute("aria-label", /^Tree at /);

    await page.locator('#view-mode-slot button[aria-pressed="false"]').click();
    await expect(page.locator("#drawing.iso")).toBeAttached();
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /Trees and plantings shown: /);
    await expect(crowns(page).first()).toBeAttached();

    // Version 11 is the edits document that can hold green elements.
    await expect.poll(() => editsVersion(page)).toBe(11);
    await page.reload();
    await expect(page.locator("#green-strip").locator('[role="option"]')).toHaveCount(1);

    await page.locator("#undo").click();
    await expect(page.locator("#green-strip")).toHaveCount(0);
  });

  test("Alt with an arrow moves the tree by a metre", async ({ page }) => {
    await openGreenStreet(page, 30);
    await addTree(page, 15);
    const cell = page.locator("#green-strip").locator('[role="option"]').first();
    await expect(cell).toHaveAttribute("aria-label", /15\.0 m to /);
    await page.locator("#green-strip").focus();
    await page.keyboard.press("Alt+ArrowRight");
    await expect(cell).toHaveAttribute("aria-label", /16\.0 m to /);
  });

  test("the roadway is not offered as a host and the form refuses a place on it", async ({ page }) => {
    await openGreenStreet(page, 30);
    const hosts = await page.locator("#green-host option").allTextContents();
    expect(hosts.length).toBeGreaterThan(0);
    for (const h of hosts) expect(h, "a travel lane is not a host").not.toMatch(/^(Travel|Driving|Transit)/);

    // Four metres (400 cm) to either side of the middle of a sidewalk is on the road or off the section, whichever side the
    // sidewalk is on: one of the two is the roadway, and the add refuses it with its sentence.
    await page.locator("#green-kind").selectOption({ label: "Tree" });
    await page.locator("#green-at").fill("15");
    const sidewalk = page
      .locator("#green-host option")
      .filter({ hasText: /^Sidewalk/ })
      .first();
    await page.locator("#green-host").selectOption((await sidewalk.getAttribute("value"))!);
    const refusals: string[] = [];
    for (const fine of ["-400", "400"]) {
      await page.locator("#green-fine").fill(fine);
      await page.locator("#green-add").click();
      await expect(live(page)).toHaveText(/^That /);
      refusals.push((await live(page).textContent()) ?? "");
    }
    expect(refusals).toContain("That cannot stand on the roadway.");
    await expect(page.locator("#green-strip")).toHaveCount(0);
  });

  test("a tree 5 m from a junction fails the sightline check, naming it", async ({ page }) => {
    const row = await openStreetWithAJunctionAtItsStart(page);
    await expect(row).not.toHaveClass("ok");
    await expect(row).toContainText("Tree at 5.0 m");
    await expect(row).toContainText("from a junction at the end of the street");
  });

  test("the option label is French in French", async ({ page }) => {
    await openGreenStreet(page, 30);
    await addTree(page, 15);
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expect(page.locator("#green-strip").locator('[role="option"]').first()).toHaveAttribute(
      "aria-label",
      /^Arbre à /,
    );
  });

  test("the tools and the row work from the keyboard alone, with one tab stop in the row", async ({ page }) => {
    await openGreenStreet(page, 30);
    await page.locator("#green-at").focus();
    await page.keyboard.type("15");
    await page.locator("#green-host").selectOption({ index: 0 });
    // Enter in a field adds the element (nothing is chosen yet).
    await page.locator("#green-at").press("Enter");
    const strip = page.locator("#green-strip");
    await expect(strip.locator('[role="option"]')).toHaveCount(1);
    // The focus went to the row, which is one tab stop (the listbox), its cells are not.
    await expect(strip).toBeFocused();
    await expect(strip).toHaveAttribute("tabindex", "0");
    await expect(strip.locator('[role="option"]').first()).not.toHaveAttribute("tabindex", /.+/);
    await expect(strip).toHaveAttribute("aria-activedescendant", /^green-0$/);
    await page.keyboard.press("Alt+ArrowRight");
    await expect(strip.locator('[role="option"]').first()).toHaveAttribute("aria-label", /16\.0 m to /);
  });
});

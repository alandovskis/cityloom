import type { Page } from "@playwright/test";

import {
  chooseCell,
  expect,
  openAnchoredStreet,
  pickRunAt,
  pickToEnd,
  placesOnMap,
  planBoundaries,
  showStreetLevel,
  streetLength,
  test,
} from "./fixtures";

const speed = (page: Page) => page.locator("#speed-limit");
/** The speed limit sign the 2.5D used to draw: it is gone, the limit is set at the details panel's Street level. */
const sign = (page: Page) => page.locator("#drawing .iso-sign");

/** Opens the first street of the city whose Speed limit control shows `value` ("" is Not set): the Plateau's
 *  streets carry the speeds OpenStreetMap gives them, and the test does not know which is which by number. The
 *  control is at the details panel's Street level, which this shows. */
async function openStreetWithSpeed(page: Page, value: string) {
  await page.goto("/map.html");
  for (const href of (await placesOnMap(page)).streets) {
    await page.goto("/" + href);
    await showStreetLevel(page);
    await expect(speed(page)).toBeVisible();
    if ((await speed(page).inputValue()) === value) return href;
  }
  throw new Error(`no street whose speed limit is ${JSON.stringify(value)}`);
}

// A street's speed limit: imported from OpenStreetMap's maxspeed, changed at the details panel's Street level. The 2.5D
// draws no sign for it.

test.describe("a street's speed limit", () => {
  // `openStreetWithSpeed` opens the city's streets one by one until one has the speed it wants: about 29 s on CI's
  // runner, against the default 30 s.
  test.describe.configure({ timeout: 120_000 });

  test("the imported speed is shown; choosing another changes it and Undo restores it, with no sign in 2.5D", async ({
    page,
  }) => {
    await openStreetWithSpeed(page, "40");
    await expect(speed(page)).toBeVisible();
    await expect(speed(page).locator("option").first()).toHaveText("Not set");

    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(sign(page)).toHaveCount(0);
    await expect(page.locator("#drawing")).not.toHaveAttribute("aria-label", /Speed limit/);

    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await expect(sign(page)).toHaveCount(0);
  });

  test("the control follows the street through Undo, Redo and Start over", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await page.locator("#redo").click();
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#undo").click();
    await expect(speed(page)).toHaveValue("40");
    await page.locator("#redo").click();
    await expect(speed(page)).toHaveValue("30");
    await page.locator("#reset").click();
    await expect(speed(page)).toHaveValue("40");
  });

  test("a chosen speed is kept across a reload", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await speed(page).selectOption("30");
    await expect(speed(page)).toHaveValue("30");
    await page.reload();
    // Nothing is chosen after a reload, so the panel shows the street again.
    await expect(speed(page)).toBeVisible();
    await expect(speed(page)).toHaveValue("30");
  });

  test("Not set is kept across a reload", async ({ page }) => {
    await openStreetWithSpeed(page, "30");
    await speed(page).selectOption("");
    await expect(speed(page)).toHaveValue("");
    await page.reload();
    await expect(speed(page)).toHaveValue("");
  });

  test("a street OpenStreetMap gives no speed shows Not set", async ({ page }) => {
    await openStreetWithSpeed(page, "");
    await expect(speed(page)).toHaveValue("");
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
  });

  test("the control keeps the focus after a change", async ({ page }) => {
    await openStreetWithSpeed(page, "40");
    await expect(speed(page)).toBeVisible();
    await speed(page).focus();
    await speed(page).selectOption("20");
    await expect(speed(page)).toHaveValue("20");
    await expect(speed(page)).toBeFocused();
  });
});

test.describe("the details panel's path", () => {
  const path = (page: Page) => page.locator(".panel-path");
  const step = (page: Page, level: string) => path(page).locator(`.path-step[data-level="${level}"]`);
  const here = (page: Page) => path(page).locator('[aria-current="location"]');

  test("with nothing chosen it shows the street, which holds the speed limit", async ({ page }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    await expect(here(page)).toHaveText("Street");
    await expect(path(page).locator("button")).toHaveCount(0);
    await expect(page.locator("#p-street")).toBeVisible();
    await expect(speed(page)).toBeVisible();
    await expect(page.locator("#p-piece")).toBeHidden();
  });

  test("a pressed piece shows under its zone, and the path's steps go back up with the piece still chosen", async ({
    page,
  }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    const first = page.locator("#piece-list [role=option]").first();
    await first.click();
    await expect(page.locator("#p-piece")).toBeVisible();
    await expect(page.locator("#p-street")).toBeHidden();
    await expect(path(page).locator("li")).toHaveCount(3);
    await expect(step(page, "zone")).toHaveText("Zone 1");

    await step(page, "zone").click();
    await expect(page.locator("#p-zone")).toBeVisible();
    await expect(page.locator("#zone-split")).toBeVisible();
    await expect(here(page)).toHaveText("Zone 1");
    await expect(here(page)).toBeFocused();
    await expect(first).toHaveAttribute("aria-selected", "true");

    await step(page, "street").click();
    await expect(speed(page)).toBeVisible();
    await expect(here(page)).toBeFocused();

    await first.click();
    await expect(page.locator("#p-piece")).toBeVisible();
  });

  test("from the street, one button shows the selected zone's commands", async ({ page }) => {
    await page.goto("/map.html");
    await page.goto("/" + (await placesOnMap(page)).streets[0]);
    await expect(page.locator("#zone-split")).toBeHidden();
    await page.locator("#zone-entry").click();
    await expect(page.locator("#p-zone")).toBeVisible();
    await expect(page.locator("#zone-split")).toBeVisible();
    await expect(here(page)).toHaveText("Zone 1");
    await expect(here(page)).toBeFocused();
  });

  test("a stretch of the curbside shows along the street, not under a zone", async ({ page }) => {
    await page.goto("/map.html");
    for (const href of (await placesOnMap(page)).streets) {
      await page.goto("/" + href);
      await expect(page.locator("#plan svg")).toBeVisible();
      if ((await page.locator("#curb-strip-right").count()) > 0) break;
    }
    await chooseCell(page, page.locator("#curb-right-0"));
    await expect(page.locator("#p-curb")).toBeVisible();
    await expect(page.locator("#curb-use")).toBeVisible();
    await expect(path(page).locator("li")).toHaveCount(2);
    await expect(here(page)).toHaveText("Free stretch");
  });
});

// A stretch has a speed limit of its own (#148): the select at the Street level sets the picked stretch's, and a stretch
// may follow the street's again.
test.describe("a stretch's speed limit", () => {
  test.describe.configure({ timeout: 120_000 });

  test("is set for the picked stretch only, the rest follows the street, and Undo takes it back", async ({ page }) => {
    await openAnchoredStreet(page, 30);
    const half = Math.floor((await streetLength(page)) / 2);
    await pickToEnd(page, half);
    await showStreetLevel(page);
    // One run: the street's own speed limit is the one shown, and there is no street to follow.
    await expect(speed(page).locator("option").first()).toHaveText("Not set");
    const street = await speed(page).inputValue();

    // Give the picked stretch a speed the street does not have: it becomes a run of its own.
    const chosen = street === "20" ? "30" : "20";
    await speed(page).selectOption(chosen);
    await expect(speed(page)).toHaveValue(chosen);
    await expect(planBoundaries(page)).toHaveCount(1);
    const own = await speed(page).locator("option").first().textContent();
    expect(own).toMatch(/^Same as the street: (\d+ km\/h|not set)$/);
    await pickRunAt(page, 1);
    await expect(speed(page)).toHaveValue("street");
    await pickRunAt(page, half + 1);
    await expect(speed(page)).toHaveValue(chosen);

    // Following the street again, the stretch is the same as the rest and joins it.
    await showStreetLevel(page);
    await speed(page).selectOption("street");
    await expect(planBoundaries(page)).toHaveCount(0);
    // One run again: the street's own speed limit is shown.
    await expect(speed(page)).toHaveValue(street);
    await page.locator("#undo").click();
    await expect(planBoundaries(page)).toHaveCount(1);
    await page.locator("#undo").click();
    await expect(planBoundaries(page)).toHaveCount(0);
  });
});

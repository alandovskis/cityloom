import { expect, JUNCTION, live, STREET, test } from "./fixtures";

test.describe("the city map", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#map")).toBeVisible();
  });

  test("shows the whole sample city and says whether it works", async ({ page }) => {
    await expect(page.locator("#street-name")).toHaveText("Sample city");
    await expect(page.locator(".insp-sub")).toHaveText("9 junctions, 23 streets");
    await expect(page.locator("#fit")).toHaveText("32 places. Every check passes.");
    await expect(page.locator("#fit .tick")).toBeVisible();
    await expect(page.locator(".places a.place-row")).toHaveCount(32);
    await expect(page.locator("#map-view")).toHaveAttribute("aria-label", "Map of the city, north up");
  });

  test("lists junctions and streets that link to their editors", async ({ page }) => {
    await expect(page.locator(`a.place-row[href="intersection.html?junction=${JUNCTION}"]`)).toContainText(
      "Junction 1",
    );
    await expect(page.locator(`a.place-row[href="street.html?street=${STREET}"]`)).toBeVisible();
  });

  test("a place in the list opens its editor", async ({ page }) => {
    await page.locator(`a.place-row[href="intersection.html?junction=${JUNCTION}"]`).click();
    await expect(page).toHaveURL(/intersection\.html\?junction=3$/);
    await expect(page.locator("#street-name")).toHaveText("Junction 1");
  });

  test("zooming in and out moves the view and the whole city button brings it back", async ({ page }) => {
    const box = () => page.locator("#map").getAttribute("viewBox");
    const fitted = await box();
    await page.locator("#zoom-in").click();
    await expect.poll(box).not.toBe(fitted);
    await page.locator("#zoom-out").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(box).toBe(fitted);
  });

  test("the keys zoom the map when it has focus", async ({ page }) => {
    const box = () => page.locator("#map").getAttribute("viewBox");
    const fitted = await box();
    await page.locator("#map-view").focus();
    await page.keyboard.press("+");
    await expect.poll(box).not.toBe(fitted);
  });

  test("units change the lengths the map shows", async ({ page }) => {
    const before = await page.locator("#map-view").innerHTML();
    await page.locator("#account-btn").click();
    await page.locator('[data-unit="ft"]').click();
    await expect.poll(() => page.locator("#map-view").innerHTML()).not.toBe(before);
  });

  test("start over is off until a street or junction has been changed", async ({ page }) => {
    await expect(page.locator("#reset")).toBeDisabled();
  });

  test("a change made in a street editor shows on the map", async ({ page }) => {
    await page.goto(`/street.html?street=${STREET}`);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/map.html");
    await expect(page.locator("#fit")).toHaveText(
      "1 place needs attention: Sample Avenue 2, the edge of the map to Junction 4.",
    );
    await expect(page.locator("#fit .tick")).toHaveCount(0);
    await expect(page.locator("#reset")).toBeEnabled();
    await page.locator("#t-changes").click();
    await expect(page.locator("#changes")).not.toBeEmpty();
  });

  test("start over asks to be pressed twice, and then puts the city back as first laid out", async ({ page }) => {
    await page.goto(`/street.html?street=${STREET}`);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/map.html");
    await page.locator("#reset").click();
    await expect(page.locator("#reset-label")).toHaveText("Press again to start over");
    await expect(live(page)).toHaveText(
      "This puts every street and junction back as first laid out. Press again to confirm.",
    );
    await page.locator("#reset").click();
    await expect(page.locator("#reset")).toBeDisabled();
    await expect(page.locator("#fit")).toHaveText("32 places. Every check passes.");
    await expect(page.locator("#fit .tick.fresh")).toBeVisible();
    await page.goto(`/street.html?street=${STREET}`);
    await expect(page.locator("#revs tbody tr")).toHaveCount(1);
  });
});

import type { Page } from "@playwright/test";

import { expect, JUNCTION, live, mapReady, mapStill, serveWorld, test } from "./fixtures";

test.describe("the search box on the city map", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#basemap canvas")).toBeVisible();
  });

  test("has no Search button in the header: Enter does it", async ({ page }) => {
    await expect(page.locator(".bar .search-go")).toBeHidden();
  });

  test("is the biggest thing in the header", async ({ page }) => {
    const box = await page.locator("#search").boundingBox();
    const bar = await page.locator(".bar").boundingBox();
    expect(box!.height).toBeGreaterThanOrEqual(44);
    expect(box!.width).toBeGreaterThan(400);
    expect(box!.y).toBeGreaterThanOrEqual(bar!.y);
    expect(box!.y + box!.height).toBeLessThanOrEqual(bar!.y + bar!.height);
    await expect(page.locator("#search")).toHaveAttribute("placeholder", "Search places");
  });

  test("the / key goes to it from anywhere, but not while typing in another field", async ({ page }) => {
    await page.keyboard.press("/");
    await expect(page.locator("#search")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.locator("#search")).not.toBeFocused();
    await page.locator("#account-btn").click();
    await page.locator("#region").focus();
    await page.keyboard.press("/");
    await expect(page.locator("#search")).not.toBeFocused();
  });

  test("lists the places found, says how many, and has nothing open until something is typed", async ({ page }) => {
    await expect(page.locator("#search")).toHaveAttribute("aria-expanded", "false");
    await expect(page.locator(".search-pop")).toBeHidden();
    await page.locator("#search").fill("avenue");
    await expect(page.locator("#search")).toHaveAttribute("aria-expanded", "true");
    const options = page.locator('#search-results [role="option"]');
    await expect(options.first()).toContainText("Sample Avenue");
    expect(await options.count()).toBeGreaterThan(1);
    await expect(page.locator(".search-note")).toContainText("match");
  });

  test("finds a junction by its name and the streets that end at it", async ({ page }) => {
    await page.locator("#search").fill("junction 4");
    const options = page.locator('#search-results [role="option"]');
    await expect(options.first()).toContainText("Junction 4");
    await expect(options.nth(1)).toContainText("Junction 4");
  });

  test("Enter opens the first place found", async ({ page }) => {
    await page.locator("#search").fill("junction 1");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/intersection\.html\?junction=\d+$/);
  });

  test("the arrow keys move down the results and Enter opens the one they are on", async ({ page }) => {
    await page.locator("#search").fill("avenue");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(page.locator('#search-results [role="option"][aria-selected="true"]')).toHaveCount(1);
    await expect(page.locator("#search")).toHaveAttribute("aria-activedescendant", "sr-1");
    const second = await page.locator("#sr-1 a").getAttribute("href");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL((url) => url.pathname.slice(1) + url.search === second);
  });

  test("Escape clears what was typed, then leaves the box", async ({ page }) => {
    await page.locator("#search").fill("avenue");
    await page.keyboard.press("Escape");
    await expect(page.locator("#search")).toHaveValue("");
    await expect(page.locator(".search-pop")).toBeHidden();
    await expect(page.locator("#search")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.locator("#search")).not.toBeFocused();
  });

  test("a search that finds nothing says so and how to get the places back", async ({ page }) => {
    await page.locator("#search").fill("zzz");
    await expect(page.locator('#search-results [role="option"]')).toHaveCount(0);
    await expect(page.locator(".search-note")).toHaveText("No places match “zzz”. Clear the search to see all 32.");
  });

  test("a result that needs attention says so", async ({ page }) => {
    await page.goto(`/street.html?street=1`);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/map.html");
    await page.locator("#search").fill("avenue");
    await expect(page.locator("#search-results .st.bad").first()).toHaveText("Needs attention");
  });

  test("clicking a result opens the place", async ({ page }) => {
    await page.locator("#search").fill("junction 1");
    await page.locator('#search-results [role="option"] a').first().click();
    await expect(page).toHaveURL(/intersection\.html\?junction=\d+$/);
    expect(JUNCTION).toBeGreaterThan(0);
  });
});

test.describe("the map under the floating panels", () => {
  test.use({ area: "world" });
  test.beforeEach(async ({ page }) => {
    await serveWorld(page);
    await page.goto("/map.html");
    await mapReady(page);
    await mapStill(page);
  });

  /** The box the places occupy on the screen, in page pixels. */
  const placesBox = (page: Page) =>
    page.evaluate(() => {
      const map = (window as any).cityloomMap;
      const r = map.getCanvas().getBoundingClientRect();
      let l = Infinity,
        t = Infinity,
        rt = -Infinity,
        b = -Infinity;
      for (const f of map.querySourceFeatures("places")) {
        const pts = f.geometry.type === "Point" ? [f.geometry.coordinates] : f.geometry.coordinates;
        for (const c of pts) {
          const p = map.project(c);
          l = Math.min(l, r.left + p.x);
          rt = Math.max(rt, r.left + p.x);
          t = Math.min(t, r.top + p.y);
          b = Math.max(b, r.top + p.y);
        }
      }
      return { l, r: rt, t, b };
    });

  test("the whole city is fitted between the panels and under the bar, and centred there", async ({ page }) => {
    const left = (await page.locator(".panel.left").boundingBox())!;
    const right = (await page.locator(".panel.right").boundingBox())!;
    const bar = (await page.locator(".bar").boundingBox())!;
    await expect.poll(async () => (await placesBox(page)).l).toBeGreaterThan(left.x + left.width - 1);
    const c = await placesBox(page);
    expect(c.r).toBeLessThan(right.x + 1);
    expect(c.t).toBeGreaterThan(bar.y + bar.height - 1);
    const open = { l: left.x + left.width, r: right.x };
    // The old SVG drawing was placed exactly; MapLibre fits with padding through a camera ease, so this allows
    // some room. Once still the fit is off by well under a pixel (0.1 px measured); a panel the fit ignored would
    // move the centre by half its width (180 px for the 360 px panel), far past 60.
    expect(Math.abs((c.l + c.r) / 2 - (open.l + open.r) / 2)).toBeLessThan(60);
  });

  test("hiding the panels gives the map the room back and Whole city centres it again", async ({ page }) => {
    const before = await placesBox(page);
    await page.locator("#inspector-toggle").click();
    await page.locator("#notes-toggle").click();
    await page.locator("#zoom-fit").click();
    await expect
      .poll(async () => (await placesBox(page)).r - (await placesBox(page)).l)
      .toBeGreaterThan(before.r - before.l);
    await mapStill(page); // measured where the fit ends, not on its way there
    const after = await placesBox(page);
    expect(Math.abs((after.l + after.r) / 2 - page.viewportSize()!.width / 2)).toBeLessThan(60);
  });
});

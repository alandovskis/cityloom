import { expect, JUNCTION, live, test } from "./fixtures";

test.describe("the search box on the city map", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#map")).toBeVisible();
  });

  test("is the biggest thing in the header", async ({ page }) => {
    const box = await page.locator("#search").boundingBox();
    const bar = await page.locator(".bar").boundingBox();
    expect(box!.height).toBeGreaterThanOrEqual(44);
    expect(box!.width).toBeGreaterThan(400);
    expect(box!.y).toBeGreaterThanOrEqual(bar!.y);
    expect(box!.y + box!.height).toBeLessThanOrEqual(bar!.y + bar!.height);
    await expect(page.locator("#search")).toHaveAttribute("placeholder", "Search junctions and streets");
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
    await expect(page).toHaveURL(new RegExp(second!.replace(/[?.]/g, "\\$&") + "$"));
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
  const discs = async (page: import("@playwright/test").Page) => {
    const boxes = await page
      .locator("#map .m-jc")
      .evaluateAll((els) =>
        els.map((e) => e.getBoundingClientRect()).map((r) => ({ l: r.left, r: r.right, t: r.top, b: r.bottom })),
      );
    return {
      l: Math.min(...boxes.map((b) => b.l)),
      r: Math.max(...boxes.map((b) => b.r)),
      t: Math.min(...boxes.map((b) => b.t)),
      b: Math.max(...boxes.map((b) => b.b)),
    };
  };

  test("the whole city is fitted between the panels and under the bar, and centred there", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#map .m-jc").first()).toBeVisible();
    const left = (await page.locator(".panel.left").boundingBox())!;
    const right = (await page.locator(".panel.right").boundingBox())!;
    const bar = (await page.locator(".bar").boundingBox())!;
    const c = await discs(page);
    expect(c.l).toBeGreaterThan(left.x + left.width);
    expect(c.r).toBeLessThan(right.x);
    expect(c.t).toBeGreaterThan(bar.y + bar.height);
    const open = { l: left.x + left.width, r: right.x };
    expect(Math.abs((c.l + c.r) / 2 - (open.l + open.r) / 2)).toBeLessThan(30);
  });

  test("hiding the panels gives the map the room back and Whole city centres it again", async ({ page }) => {
    await page.goto("/map.html");
    await expect(page.locator("#map .m-jc").first()).toBeVisible();
    const before = await discs(page);
    await page.locator("#inspector-toggle").click();
    await page.locator("#notes-toggle").click();
    await page.locator("#zoom-fit").click();
    await expect.poll(async () => (await discs(page)).r - (await discs(page)).l).toBeGreaterThan(before.r - before.l);
    const after = await discs(page);
    const viewport = page.viewportSize()!;
    expect(Math.abs((after.l + after.r) / 2 - viewport.width / 2)).toBeLessThan(30);
  });
});

import { expect, live, STREET, test } from "./fixtures";

test.describe("the home page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  });

  test("is the first thing at the site root, and says what CityLoom is for", async ({ page }) => {
    await expect(page).toHaveTitle("CityLoom: redesign the streets of your city");
    await expect(page.locator("h1")).toHaveText("Redesign the streets of your city.");
    await expect(page.locator(".lead")).toContainText("rearrange it");
    await expect(page.locator(".hero-note")).toContainText("placeholders");
  });

  test("puts the search box in the middle of the first screen, and it is big", async ({ page }) => {
    const box = (await page.locator("#search").boundingBox())!;
    const viewport = page.viewportSize()!;
    expect(box.height).toBeGreaterThanOrEqual(56);
    expect(box.width).toBeGreaterThan(400);
    expect(box.y).toBeLessThan(viewport.height * 0.7);
    await expect(page.locator("#search")).toHaveAttribute("placeholder", "Search places");
  });

  test("a search finds places and Enter opens the first", async ({ page }) => {
    await page.locator("#search").fill("avenue");
    await expect(page.locator('#search-results [role="option"]').first()).toContainText("Sample Avenue");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/street\.html\?street=\d+$/);
  });

  test("has a Search button, and it opens the first place found", async ({ page }) => {
    const go = page.getByRole("button", { name: "Search" });
    await expect(go).toBeVisible();
    await page.locator("#search").fill("junction 1");
    await go.click();
    await expect(page).toHaveURL(/intersection\.html\?junction=\d+$/);
  });

  test("the search is a white field with a clear edge, not a grey one", async ({ page }) => {
    const style = await page.locator("#search").evaluate((el) => {
      const c = getComputedStyle(el);
      return { bg: c.backgroundColor, border: c.borderTopWidth, edge: c.borderTopColor };
    });
    expect(style.bg).toBe("rgb(255, 255, 255)");
    expect(parseFloat(style.border)).toBeGreaterThanOrEqual(1.5);
    expect(style.edge).not.toBe("rgb(205, 211, 218)");
  });

  test("the / key goes to the search box", async ({ page }) => {
    await page.keyboard.press("/");
    await expect(page.locator("#search")).toBeFocused();
  });

  test("the way into the map and into a new street are one press away", async ({ page }) => {
    await page.getByRole("link", { name: "Open the city map" }).click();
    await expect(page).toHaveURL(/map\.html$/);
    await page.goBack();
    await page.getByRole("link", { name: "Start a new street" }).click();
    await expect(page).toHaveURL(/street\.html$/);
  });

  test("the top bar links to the editors and the brand stays home", async ({ page }) => {
    await expect(page.locator('.surfaces a[href="map.html"]')).toBeVisible();
    await expect(page.locator('.surfaces a[href="street.html"]')).toBeVisible();
    await expect(page.locator('.surfaces a[href="intersection.html"]')).toBeVisible();
    await expect(page.locator(".brand")).toHaveAttribute("href", "index.html");
  });

  test("shows the city behind the card, which cannot be pressed or reached", async ({ page }) => {
    await expect(page.locator(".hero-map #map .m-jc")).toHaveCount(9);
    await expect(page.locator(".hero-map")).toHaveAttribute("inert", "");
    await expect(page.locator(".hero-map")).toHaveAttribute("aria-hidden", "true");
    const reachable = await page
      .locator(".hero-map")
      .evaluate((el) => (el as HTMLElement).matches(":focus-within") || el.querySelectorAll("a[href]").length > 0);
    expect(reachable).toBe(true); // the links are in the drawing, and inert is what keeps them out of reach
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    const inside = await page.evaluate(() => !!document.activeElement?.closest(".hero-map"));
    expect(inside).toBe(false);
  });

  test("says how the city stands, and says so when a place needs attention", async ({ page }) => {
    await expect(page.locator(".hero-facts")).toContainText("Sample city: 9 junctions, 23 streets.");
    await expect(page.locator(".hero-facts")).toContainText("Every check passes.");
    await expect(page.locator(".hero-facts .tick")).toBeVisible();
    await page.goto(`/street.html?street=${STREET}`);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/");
    await expect(page.locator(".hero-facts.bad")).toContainText("needs attention");
    await expect(page.locator(".hero-facts .tick")).toHaveCount(0);
  });

  test("has the settings every page has: a theme that is remembered", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  });
});

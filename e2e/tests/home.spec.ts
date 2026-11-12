import { expect, live, openStreet, test } from "./fixtures";

test.describe("the home page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  });

  test("is the first thing at the site root: the projects, then what CityLoom is for", async ({ page }) => {
    await expect(page).toHaveTitle("CityLoom: redesign the streets of your city");
    await expect(page.locator("h1")).toHaveText("Your projects");
    await expect(page.getByRole("heading", { level: 2, name: "About CityLoom" })).toBeVisible();
    await expect(page.locator(".about-title")).toHaveText("Redesign the streets of your city.");
    await expect(page.locator(".lead")).toContainText("rearrange it");
    await expect(page.locator(".hero-note")).toContainText("OpenStreetMap");
    // a project's area is fixed when it is made: there is no place to search for
    await expect(page.locator("#search, #area-results")).toHaveCount(0);
  });

  test("the way into the map is one press away, and there is no way into an editor without a place", async ({
    page,
  }) => {
    await page.getByRole("link", { name: "Open the city map" }).click();
    await expect(page).toHaveURL(/map\.html\?project=first$/);
    await page.goBack();
    await expect(page.locator('a[href="street.html"], a[href="intersection.html"]')).toHaveCount(0);
  });

  test("the top bar links to the map and the brand stays home", async ({ page }) => {
    await expect(page.locator('.surfaces a[href="map.html?project=first"]')).toBeVisible();
    await expect(page.locator(".brand")).toHaveAttribute("href", "index.html");
  });

  test("shows the city behind the card, which cannot be pressed or reached", async ({ page }) => {
    expect(await page.locator(".hero-map #map .m-jc").count()).toBeGreaterThan(5);
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

  test("fits the city beside the card, so no street is cut by it, and lets it run off the right edge", async ({
    page,
  }) => {
    const card = (await page.locator(".hero-card").boundingBox())!;
    const west = await page
      .locator(".hero-map #map .m-jc")
      .evaluateAll((els) => Math.min(...els.map((e) => e.getBoundingClientRect().left)));
    // A junction's marker reaches a few pixels beyond its centre, which is what the city is fitted by.
    expect(west).toBeGreaterThan(card.x + card.width - 4);
    const east = await page
      .locator(".hero-map #map .m-jc")
      .evaluateAll((els) => Math.max(...els.map((e) => e.getBoundingClientRect().right)));
    expect(east).toBeLessThanOrEqual(page.viewportSize()!.width + 0.14 * page.viewportSize()!.width);
    await expect(page.locator(".hero-map #map .m-name").first()).toBeHidden();
  });

  test("says how the city stands, and says so when a place needs attention", async ({ page }) => {
    await expect(page.locator(".hero-facts")).toContainText(
      /^Plateau Mont-Royal, Montréal: \d+ junctions, \d+ streets\./,
    );
    await expect(page.locator(".hero-facts")).toContainText("Every check passes.");
    await expect(page.locator(".hero-facts .tick")).toBeVisible();
    await openStreet(page);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await page.goto("/");
    await expect(page.locator(".hero-facts.bad")).toContainText(/needs? attention/);
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

test("the city behind the card is read from the metro tile, and nothing else is asked", async ({ page, baseURL }) => {
  const asked: string[] = [];
  page.on("request", (request) => asked.push(request.url()));
  await page.goto("/");
  await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  expect(asked.filter((url) => url.endsWith("/data/metro/0_0.json")).length).toBeGreaterThan(0);
  // the site itself and no other host, Overpass least of all
  expect([...new Set(asked.map((url) => new URL(url).host))]).toEqual([new URL(baseURL!).host]);
});

test("the credits page is in the same world and links home", async ({ page }) => {
  await page.goto("/credits.html");
  await expect(page.locator("h1")).toHaveText("Credits");
  await expect(page.locator(".bar .brand")).toHaveAttribute("href", "index.html");
  await expect(page.locator(".read-card")).toContainText("Overpass");
  await expect(page.locator(".read-card")).not.toContainText("Barlow");
});

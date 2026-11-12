import { expect, test } from "./fixtures";

test.describe("the home page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#region option")).toHaveCount(6);
  });

  test("is the first thing at the site root: the projects, and nothing else", async ({ page }) => {
    await expect(page).toHaveTitle("CityLoom projects");
    await expect(page.locator("h1")).toHaveText("Your projects");
    // What CityLoom is for is the About page's, with the city it draws.
    await expect(page.locator(".about-title, .hero-card, .hero-map, #hero-facts-slot")).toHaveCount(0);
    // a project's area is fixed when it is made: there is no place to search for
    await expect(page.locator("#search, #area-results")).toHaveCount(0);
  });

  test("there is no way into an editor without a place", async ({ page }) => {
    await expect(page.locator('a[href="street.html"], a[href="intersection.html"]')).toHaveCount(0);
  });

  test("has the settings every page has: a theme that is remembered", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-theme-set="dark"]').click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  });
});

test("the list asks for no city: no road tile and no other host", async ({ page, baseURL }) => {
  const asked: string[] = [];
  page.on("request", (request) => asked.push(request.url()));
  await page.goto("/");
  await expect(page.locator("[data-version]")).not.toBeEmpty();
  await expect(page.locator(".project-row").first()).toBeVisible();
  expect(asked.filter((url) => url.includes("/data/metro/"))).toEqual([]);
  expect([...new Set(asked.map((url) => new URL(url).host))]).toEqual([new URL(baseURL!).host]);
});

test("the credits page is in the same world and links home", async ({ page }) => {
  await page.goto("/credits.html");
  await expect(page.locator("h1")).toHaveText("Credits");
  await expect(page.locator(".bar .brand")).toHaveAttribute("href", "index.html");
  await expect(page.locator(".read-card")).toContainText("Overpass");
  await expect(page.locator(".read-card")).not.toContainText("Barlow");
});

// The About page: what CityLoom is for, with the city of the project opened last drawn behind it; and the top bar
// every page shares, which leads to it.

import type { Page } from "@playwright/test";

import { expect, live, mapReady, openJunction, openStreet, test } from "./fixtures";

/** Opens the About page and waits for the city behind it, which is drawn once its roads are got. */
async function openAbout(page: Page) {
  await page.goto("/about.html");
  await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
}

test.describe("the About page", () => {
  test("says what CityLoom is for, under its own heading", async ({ page }) => {
    await openAbout(page);
    await expect(page).toHaveTitle("About CityLoom");
    await expect(page.locator("h1")).toHaveText("About CityLoom");
    await expect(page.locator(".about-title")).toHaveText("Redesign the streets of your city.");
    await expect(page.locator(".lead")).toContainText("rearrange it");
    await expect(page.locator(".hero-note")).toContainText("OpenStreetMap");
    await expect(page.locator(".hero-note a")).toHaveAttribute("href", "https://www.openstreetmap.org/copyright");
  });

  test("sends to the projects, since it is in no project and has no map to open", async ({ page }) => {
    await openAbout(page);
    // The drawing behind the card is inert: what it links cannot be pressed or reached (see below).
    const reachable = page.locator(
      ':is(a[href^="map.html"], a[href^="street.html"], a[href^="intersection.html"]):not(.hero-map *)',
    );
    await expect(reachable).toHaveCount(0);
    await page.getByRole("link", { name: "Open your projects" }).click();
    await expect(page).toHaveURL(/\/index\.html$/);
    await expect(page.locator("h1")).toHaveText("Your projects");
  });

  test("the first stop is a skip link to its heading", async ({ page }) => {
    await openAbout(page);
    const skip = page.locator(".skip-link");
    await page.keyboard.press("Tab");
    await expect(skip).toBeFocused();
    await expect(skip).toHaveText("Skip to the introduction");
    await page.keyboard.press("Enter");
    await expect(page.locator("#about-heading")).toBeFocused();
  });

  test("shows the city behind the card, which cannot be pressed or reached", async ({ page }) => {
    await openAbout(page);
    expect(await page.locator(".hero-map #map .m-jc").count()).toBeGreaterThan(5);
    await expect(page.locator(".hero-map")).toHaveAttribute("inert", "");
    await expect(page.locator(".hero-map")).toHaveAttribute("aria-hidden", "true");
    for (let i = 0; i < 8; i++) {
      await page.keyboard.press("Tab");
      expect(await page.evaluate(() => !!document.activeElement?.closest(".hero-map"))).toBe(false);
    }
  });

  test("fits the city beside the card, so no street is cut by it, and lets it run off the right edge", async ({
    page,
  }) => {
    await openAbout(page);
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
    await openAbout(page);
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
    await openAbout(page);
    await expect(page.locator(".hero-facts.bad")).toContainText(/needs? attention/);
    await expect(page.locator(".hero-facts .tick")).toHaveCount(0);
  });

  test("can be read in French and the choice is kept", async ({ page }) => {
    await openAbout(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("h1")).toHaveText("À propos de CityLoom");
    await expect(page).toHaveTitle("À propos de CityLoom");
    await expect(page.locator(".about-title")).toHaveText("Réaménagez les rues de votre ville.");
    await expect(page.locator(".lead")).toContainText("Ouvrez un projet");
    await expect(page.getByRole("link", { name: "Ouvrir vos projets" })).toBeVisible();
    await expect(page.locator(".skip-link")).toHaveText("Aller à la présentation");
    await expect(page.locator(".hero-note a")).toHaveText("les contributeurs d’OpenStreetMap");
    await expect(page.locator(".hero-facts")).toContainText("rues");
    await page.reload();
    await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expect(page.locator("h1")).toHaveText("À propos de CityLoom");
  });
});

// The top bar of every page: Projects and About, the page's own marked as current, no Map (a map is opened from its
// project), and the logo goes to the list of projects.
const PAGES: { name: string; open: (page: Page) => Promise<unknown>; current?: "Projects" | "About" }[] = [
  { name: "home", open: (page) => page.goto("/"), current: "Projects" },
  { name: "about", open: (page) => page.goto("/about.html"), current: "About" },
  { name: "map", open: (page) => page.goto("/map.html").then(() => mapReady(page)) },
  { name: "street", open: openStreet },
  { name: "junction", open: openJunction },
  { name: "account", open: (page) => page.goto("/account.html") },
  { name: "credits", open: (page) => page.goto("/credits.html") },
];

for (const { name, open, current } of PAGES) {
  test(`the ${name} page's bar links the projects and About, and no map`, async ({ page }) => {
    await open(page);
    const nav = page.getByRole("navigation", { name: "Pages" });
    await expect(nav.getByRole("link")).toHaveText(["Projects", "About"]);
    for (const link of ["Projects", "About"]) {
      const item = nav.getByRole("link", { name: link, exact: true });
      if (link === current) await expect(item).toHaveAttribute("aria-current", "page");
      else await expect(item).not.toHaveAttribute("aria-current");
    }
    await expect(page.locator(".bar").getByRole("link", { name: /^(Map|City map)$/ })).toHaveCount(0);
    await expect(page.locator('.bar a[href^="map.html"]')).toHaveCount(0);
    await expect(page.locator(".bar .brand")).toHaveAttribute("href", "index.html");
    await nav.getByRole("link", { name: "About", exact: true }).click();
    await expect(page).toHaveURL(/\/about\.html$/);
    await expect(page.locator("h1")).toHaveText("About CityLoom");
    await page.locator(".bar .brand").click();
    await expect(page).toHaveURL(/\/index\.html$/);
    await expect(page.locator("h1")).toHaveText("Your projects");
  });
}

test("the bar's links are in French for a French reader, the current one still marked", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#region option")).toHaveCount(6);
  await page.locator("#account-btn").click();
  await page.locator('[data-lang="fr-CA"]').click();
  const nav = page.getByRole("navigation", { name: "Pages" });
  await expect(nav.getByRole("link")).toHaveText(["Projets", "À propos"]);
  await expect(nav.getByRole("link", { name: "Projets" })).toHaveAttribute("aria-current", "page");
  await nav.getByRole("link", { name: "À propos" }).click();
  await expect(page.locator("h1")).toHaveText("À propos de CityLoom");
  await expect(nav.getByRole("link", { name: "À propos" })).toHaveAttribute("aria-current", "page");
});

test("the city behind the card is read from the metro tile, and nothing else is asked", async ({ page, baseURL }) => {
  const asked: string[] = [];
  page.on("request", (request) => asked.push(request.url()));
  await openAbout(page);
  expect(asked.filter((url) => url.endsWith("/data/metro/0_0.json")).length).toBeGreaterThan(0);
  // the site itself and no other host, Overpass least of all
  expect([...new Set(asked.map((url) => new URL(url).host))]).toEqual([new URL(baseURL!).host]);
});

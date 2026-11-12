import type { Page } from "@playwright/test";

import { expect, live, openStreet, placesOnMap, test } from "./fixtures";

const CORS = { "access-control-allow-origin": "*" };

/** Nominatim, answered without a network with the places in `found`; every request it gets is put in `requests`. */
const stubSearch = async (page: Page, found: object[], requests: string[] = []) => {
  await page.route("https://nominatim.openstreetmap.org/**", (route) => {
    requests.push(route.request().url());
    return route.fulfill({ headers: CORS, json: found });
  });
};

/** Two places of the Plateau, which the one metro tile the tests serve (fixtures.ts) holds. */
const PLATEAU_PLACES = [
  {
    display_name: "Mile End, Montréal, Québec",
    lat: "45.5265",
    lon: "-73.5970",
    boundingbox: ["45.5", "45.55", "-73.62", "-73.57"],
  },
  {
    display_name: "Plateau Mont-Royal, Montréal, Québec",
    lat: "45.5261",
    lon: "-73.5978",
    boundingbox: ["45.51", "45.54", "-73.61", "-73.56"],
  },
];

/** A place far from any tile. */
const KREUZBERG = [
  {
    display_name: "Kreuzberg, Friedrichshain-Kreuzberg, Berlin, 10999, Germany",
    lat: "52.5010",
    lon: "13.4235",
    boundingbox: ["52.48", "52.51", "13.38", "13.43"],
  },
];

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
  });

  test("puts the search box in the About section, below the list, and it is big", async ({ page }) => {
    const box = (await page.locator("#search").boundingBox())!;
    const list = (await page.locator(".projects-card").boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(56);
    expect(box.width).toBeGreaterThan(400);
    expect(box.y).toBeGreaterThan(list.y + list.height);
    await expect(page.locator("#search")).toHaveAttribute("placeholder", /Find a place/);
  });

  test("a search finds places, and Enter opens the first one's streets on the map", async ({ page }) => {
    const requests: string[] = [];
    await stubSearch(page, PLATEAU_PLACES, requests);
    await page.locator("#search").fill("Montréal");
    expect(requests).toEqual([]); // nothing is asked until the person asks
    await page.keyboard.press("Enter");
    await expect(page.locator('#area-results [role="option"]')).toHaveCount(2);
    await expect(page.locator('#area-results [role="option"]').first()).toContainText("Mile End, Montréal");
    await expect(page.locator(".search-note")).toContainText("2 places found");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/map\.html\?project=first$/);
    await expect(page.locator("#title-block")).toContainText("Mile End, Montréal");
    expect(Number(await page.locator("#tb-places").innerText())).toBeGreaterThan(20);
  });

  test("has a Find button, which becomes Open once there are places to open", async ({ page }) => {
    await stubSearch(page, PLATEAU_PLACES);
    const go = page.getByRole("button", { name: "Find" });
    await expect(go).toBeVisible();
    await page.locator("#search").fill("Montréal");
    await go.click();
    await expect(page.getByRole("button", { name: "Open" })).toBeVisible();
    await page.locator('#area-results [role="option"]').nth(0).click();
    await expect(page).toHaveURL(/map\.html\?project=first$/);
  });

  test("a place a metro tile holds is read from the tile, and nothing else is asked", async ({ page, baseURL }) => {
    const asked: string[] = [];
    page.on("request", (request) => asked.push(request.url()));
    await stubSearch(page, PLATEAU_PLACES.slice(0, 1));
    await page.locator("#search").fill("Mile End");
    await page.keyboard.press("Enter");
    await expect(page.locator('#area-results [role="option"]')).toHaveCount(1);
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/map\.html\?project=first$/);
    await expect(page.locator("#title-block")).toContainText("Mile End, Montréal");
    expect((await placesOnMap(page)).streets.length).toBeGreaterThan(20);
    expect(asked.filter((url) => url.endsWith("/data/metro/0_0.json")).length).toBeGreaterThan(0);
    // the place search, and the site itself: no other host, Overpass least of all
    const hosts = new Set(asked.map((url) => new URL(url).host));
    expect(hosts).not.toContain("overpass-api.de");
    expect([...hosts].sort()).toEqual(["nominatim.openstreetmap.org", new URL(baseURL!).host].sort());
  });

  test("a place no tile holds says so, in the page's language, and keeps nothing", async ({ page }) => {
    const before = await page.evaluate(() => ({ ...localStorage }));
    await stubSearch(page, KREUZBERG);
    await page.locator("#search").fill("Kreuzberg");
    await page.keyboard.press("Enter");
    await expect(page.locator('#area-results [role="option"]')).toHaveCount(1);
    await page.keyboard.press("Enter");
    await expect(page.locator(".search-note")).toContainText("CityLoom has no street data for Kreuzberg");
    await expect(page).toHaveURL(/\/$/);
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator(".search-note")).toContainText("CityLoom n’a pas de données de rues pour Kreuzberg");
    // nothing of Kreuzberg was kept, and the place chosen before is still the one the pages open
    const after = await page.evaluate(() => ({ ...localStorage }));
    expect(Object.entries(after).filter(([k, v]) => /kreuzberg|52\.50/i.test(k + v))).toEqual([]);
    const butLanguage = (s: Record<string, string>) => Object.entries(s).filter(([k]) => k !== "cityloom-lang");
    expect(butLanguage(after)).toEqual(butLanguage(before));
  });

  test("a place whose streets cannot be got says so and stays on the page", async ({ page, errors }) => {
    await stubSearch(page, PLATEAU_PLACES);
    await page.route("**/data/metro/0_0.json", (route) => route.fulfill({ status: 504, body: "" }));
    await page.locator("#search").fill("Montréal");
    await page.keyboard.press("Enter");
    await expect(page.locator('#area-results [role="option"]')).toHaveCount(2);
    await page.keyboard.press("Enter");
    await expect(page.locator(".search-note")).toContainText("could not be fetched");
    await expect(page).toHaveURL(/\/$/);
    errors.length = 0; // the browser logs the failed request, which is the point
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

test("the credits page is in the same world and links home", async ({ page }) => {
  await page.goto("/credits.html");
  await expect(page.locator("h1")).toHaveText("Credits");
  await expect(page.locator(".bar .brand")).toHaveAttribute("href", "index.html");
  await expect(page.locator(".read-card")).toContainText("Overpass");
  await expect(page.locator(".read-card")).not.toContainText("Barlow");
});

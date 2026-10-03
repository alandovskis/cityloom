import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import type { Page } from "@playwright/test";

import { expect, live, STREET, test } from "./fixtures";

const CORS = { "access-control-allow-origin": "*" };
const EXTRACT = fileURLToPath(new URL("../../osm_import/tests/data/kreuzberg.osm", import.meta.url));

/** The two services a place is found and read from, answered without a network: Nominatim with two
 *  places, and Overpass with the real extract of Kreuzberg, which the OSM reader in the browser reads. */
const stubWorld = async (page: Page, requests: string[] = []) => {
  await page.route("https://nominatim.openstreetmap.org/**", (route) => {
    requests.push(route.request().url());
    return route.fulfill({
      headers: CORS,
      json: [
        {
          display_name: "Kreuzberg, Friedrichshain-Kreuzberg, Berlin, 10999, Germany",
          lat: "52.4990",
          lon: "13.4030",
          boundingbox: ["52.48", "52.51", "13.38", "13.43"],
        },
        {
          display_name: "Kreuzberg, Bavaria, Germany",
          lat: "50.0",
          lon: "10.0",
          boundingbox: ["49.9", "50.1", "9.9", "10.1"],
        },
      ],
    });
  });
  await page.route("https://overpass-api.de/**", (route) => {
    requests.push(route.request().url());
    return route.fulfill({ headers: CORS, contentType: "application/xml", body: readFileSync(EXTRACT) });
  });
};

test.describe("the home page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  });

  test("is the first thing at the site root, and says what CityLoom is for", async ({ page }) => {
    await expect(page).toHaveTitle("CityLoom: redesign the streets of your city");
    await expect(page.locator("h1")).toHaveText("Redesign the streets of your city.");
    await expect(page.locator(".lead")).toContainText("rearrange it");
    await expect(page.locator(".hero-note")).toContainText("OpenStreetMap");
  });

  test("puts the search box in the middle of the first screen, and it is big", async ({ page }) => {
    const box = (await page.locator("#search").boundingBox())!;
    const viewport = page.viewportSize()!;
    expect(box.height).toBeGreaterThanOrEqual(56);
    expect(box.width).toBeGreaterThan(400);
    expect(box.y).toBeLessThan(viewport.height * 0.7);
    await expect(page.locator("#search")).toHaveAttribute("placeholder", /Find a place/);
  });

  test("a search finds places, and Enter opens the first one's streets on the map", async ({ page }) => {
    const requests: string[] = [];
    await stubWorld(page, requests);
    await page.locator("#search").fill("Kreuzberg");
    expect(requests).toEqual([]); // nothing is asked until the person asks
    await page.keyboard.press("Enter");
    await expect(page.locator('#area-results [role="option"]')).toHaveCount(2);
    await expect(page.locator('#area-results [role="option"]').first()).toContainText(
      "Kreuzberg, Friedrichshain-Kreuzberg",
    );
    await expect(page.locator(".search-note")).toContainText("2 places found");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/map\.html$/);
    await expect(page.locator("#title-block")).toContainText("Kreuzberg, Friedrichshain-Kreuzberg");
    expect(await page.locator("#map-slot a[href^='street.html']").count()).toBeGreaterThan(50);
    expect(requests.some((r) => r.startsWith("https://overpass-api.de/"))).toBe(true);
  });

  test("has a Find button, which becomes Open once there are places to open", async ({ page }) => {
    await stubWorld(page);
    const go = page.getByRole("button", { name: "Find" });
    await expect(go).toBeVisible();
    await page.locator("#search").fill("Kreuzberg");
    await go.click();
    await expect(page.getByRole("button", { name: "Open" })).toBeVisible();
    await page.locator('#area-results [role="option"]').nth(0).click();
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("a place whose streets cannot be got says so and stays on the page", async ({ page, errors }) => {
    await stubWorld(page);
    await page.route("https://overpass-api.de/**", (route) => route.fulfill({ status: 504, headers: CORS, body: "" }));
    await page.locator("#search").fill("Kreuzberg");
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

  test("fits the city beside the card, so no street is cut by it, and lets it run off the right edge", async ({
    page,
  }) => {
    const card = (await page.locator(".hero-card").boundingBox())!;
    const west = await page
      .locator(".hero-map #map .m-jc")
      .evaluateAll((els) => Math.min(...els.map((e) => e.getBoundingClientRect().left)));
    expect(west).toBeGreaterThan(card.x + card.width);
    const east = await page
      .locator(".hero-map #map .m-jc")
      .evaluateAll((els) => Math.max(...els.map((e) => e.getBoundingClientRect().right)));
    expect(east).toBeLessThanOrEqual(page.viewportSize()!.width + 0.14 * page.viewportSize()!.width);
    await expect(page.locator(".hero-map #map .m-name").first()).toBeHidden();
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

test("the credits page is in the same world and links home", async ({ page }) => {
  await page.goto("/credits.html");
  await expect(page.locator("h1")).toHaveText("Credits");
  await expect(page.locator(".bar .brand")).toHaveAttribute("href", "index.html");
  await expect(page.locator(".read-card")).toContainText("Overpass");
  await expect(page.locator(".read-card")).not.toContainText("Barlow");
});

import type { Page } from "@playwright/test";

import { expect, live, mapReady, test, transform } from "./fixtures";

// The network checks look across the whole city: a row for each in the city's level of the map's details, with its
// count, leading to the check's own level, where its findings name the places they concern; a place pressed there is
// shown in the details, from where Open opens it. They are worked out when the map reads the city, so an edit made on
// a street page shows on the map's next load.

const RULES = ["cycling", "access", "crossings", "lanes", "transit", "walking"];

/** The city's row for the check `id`. */
const row = (page: Page, id: string) => page.locator(`#network-slot .net-rule[data-rule="${id}"]`);
/** The check shown at its own level. */
const shown = (page: Page) => page.locator("#map-check-slot");

/** The count each check shows on its row, by rule. */
const counts = (page: Page) =>
  page.evaluate(() =>
    Object.fromEntries(
      [...document.querySelectorAll<HTMLElement>("#network-slot .net-rule")].map((s) => [
        s.dataset.rule!,
        Number(s.querySelector(".net-count")!.getAttribute("data-count")),
      ]),
    ),
  );

/** Goes from the city to the check `id`. */
async function openCheck(page: Page, id: string) {
  if (!(await page.locator("#map-city").isVisible())) await page.locator("#map-path-city").click();
  await page.locator(`#net-go-${id}`).click();
  await expect(page.locator("#map-check")).toBeVisible();
  await expect(shown(page).locator(`ul.net-findings[data-rule="${id}"]`)).toHaveCount(1);
}

/** The editor's address of a place a finding names. */
const hrefOf = (hot: string) =>
  hot.startsWith("s-")
    ? `street.html?street=${hot.slice(2)}&project=first`
    : `intersection.html?junction=${hot.slice(2)}&project=first`;

async function openMap(page: Page) {
  await page.goto("/map.html");
  await mapReady(page);
  await expect(page.locator("#network-slot .net-rule")).toHaveCount(6);
}

test.describe("the network checks on the map page", () => {
  test("have a heading, a lead, a row with its count for each of the six checks, and each check a level", async ({
    page,
  }) => {
    await openMap(page);
    await expect(page.locator("#network-panel")).toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "Network checks" })).toBeVisible();
    for (const id of RULES) {
      const n = (await counts(page))[id];
      const words = n === 0 ? "no findings" : n === 1 ? "1 finding" : `${n} findings`;
      // the row is a button whose name says the check and its count
      const go = page.locator(`#net-go-${id}`);
      await expect(go).toBeVisible();
      await expect(go).toContainText(words);
      await openCheck(page, id);
      const name = (await go.locator("b").textContent())!;
      await expect(page.getByRole("heading", { level: 2, name })).toBeVisible();
      await expect(page.locator("#map-path-current")).toHaveText(name);
      await expect(shown(page).locator(".net-count")).toHaveText(words);
      await expect(shown(page)).toContainText("These look across the whole network");
      await expect(shown(page).locator("ul.net-findings > li")).toHaveCount(Math.min(n, 50));
      await page.locator("#map-path-city").click();
      await expect(page.locator("#map-city")).toBeVisible();
    }
    const total = Object.values(await counts(page)).reduce((a, b) => a + b, 0);
    await expect(page.locator("#network-lead")).toHaveText(
      total === 0
        ? "The network checks found nothing across the streets and junctions."
        : total === 1
          ? "The network checks found 1 finding across the streets and junctions."
          : `The network checks found ${total} findings across the streets and junctions.`,
    );
  });

  test("are counted apart from the places' own checks", async ({ page }) => {
    await openMap(page);
    const total = Object.values(await counts(page)).reduce((a, b) => a + b, 0);
    expect(total, "the Plateau has findings").toBeGreaterThan(0);
    // the Notes' lead and the status line count places that fail their own checks: none of the findings
    await expect(page.locator("#checks-lead")).not.toContainText("network");
    await expect(page.locator("#fit-slot")).not.toContainText(String(total) + " findings");
  });

  test("each finding says why it failed, and a place it names is shown in the details and opened from there", async ({
    page,
  }) => {
    await openMap(page);
    const id = Object.entries(await counts(page)).find(([, n]) => n > 0)![0];
    await openCheck(page, id);
    const finding = shown(page).locator("li[data-severity]").first();
    await expect(finding.locator("b")).not.toBeEmpty();
    await expect(finding.locator(".net-severity")).toHaveText(/^(Problem|Notice)$/);
    const place = finding.locator(".net-link").first();
    const [hot, name] = [(await place.getAttribute("data-hot"))!, (await place.textContent())!];
    expect(hot).toMatch(/^[sj]-\d+$/);
    await place.focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#map-place")).toBeVisible();
    // the finding names a street with its ends; the path, by its name
    const here = (await page.locator("#map-path-current").textContent())!;
    expect(here.length).toBeGreaterThan(0);
    expect(name.startsWith(here), `${name} names ${here}`).toBe(true);
    await expect(page.locator("#map-open-place")).toHaveAttribute("href", hrefOf(hot));
    await page.locator("#map-open-place").click();
    await expect(page).toHaveURL(new RegExp(`/${hrefOf(hot).replace("?", "\\?")}$`));
  });

  test("the places of a finding are a list of links that are apart from one another", async ({ page }) => {
    await openMap(page);
    await openCheck(page, "walking");
    const finding = shown(page)
      .locator("li[data-severity]", { has: page.locator("ul.net-links li + li") })
      .first();
    const list = finding.locator("ul.net-links");
    await expect(list).toHaveCSS("display", "flex");
    await expect(list.locator("li").first()).toHaveCSS("display", "block");
    const boxes = await list.locator(".net-link").evaluateAll((els) =>
      els.map((e) => {
        const r = e.getBoundingClientRect();
        return { x: r.x, y: r.y, w: r.width, h: r.height };
      }),
    );
    expect(boxes.length).toBeGreaterThan(1);
    for (let i = 1; i < boxes.length; i++) {
      const [a, b] = [boxes[i - 1], boxes[i]];
      // the next link starts to the right of the one before it, with a gap, or on a line below it
      const apart = b.y >= a.y + a.h - 1 || b.x >= a.x + a.w + 8;
      expect(apart, `link ${i} is apart from the one before`).toBe(true);
    }
    // and no more of them are listed than the panel caps
    expect(await list.locator(".net-link").count()).toBeLessThanOrEqual(8);
  });

  test("a finding goes when the street it concerns is edited, at the map's next load", async ({ page }) => {
    await openMap(page);
    await openCheck(page, "cycling");
    // the smallest cycling link that is cut off, and the street at the head of it
    const target = await page.evaluate(() => {
      const items = [...document.querySelectorAll<HTMLElement>("#map-check-slot ul.net-findings > li")];
      const islands = items
        .filter((li) => /cut off/.test(li.querySelector("b")!.textContent!))
        .map((li) => ({ places: [...li.querySelectorAll(".net-link")].map((a) => a.getAttribute("data-hot")!) }));
      islands.sort((a, b) => a.places.length - b.places.length);
      return { hot: islands[0].places[0], size: islands[0].places.length };
    });
    const before = await shown(page).innerText();
    await page.goto("/" + hrefOf(target.hot));
    // take every bike lane off the street
    const bikes = page.locator("#drawing .iso-piece:has(.k-bike)");
    for (let guard = 0; (await bikes.count()) > 0 && guard < 4; guard++) {
      // press a bike lane to select it, then take it away
      await bikes.first().locator(".k-bike").first().click({ force: true });
      await page.locator("#wrap").focus();
      await page.keyboard.press("Delete");
    }
    await expect(bikes).toHaveCount(0);
    await page.goto("/map.html"); // leaving the page keeps what was waiting to be kept
    await mapReady(page);
    await expect(page.locator("#network-slot .net-rule")).toHaveCount(6);
    await openCheck(page, "cycling");
    await expect(shown(page).locator(`.net-link[data-hot="${target.hot}"]`)).toHaveCount(0);
    expect(await shown(page).innerText()).not.toEqual(before);
  });

  test("a walking finding names the streets and junctions of its route, and one pressed is shown and opened", async ({
    page,
  }) => {
    await openMap(page);
    await expect(row(page, "walking")).toContainText("Walking routes");
    await openCheck(page, "walking");
    const finding = shown(page).locator("li[data-severity]").first();
    await expect(finding).toBeVisible();
    const hots = await finding.locator(".net-link").evaluateAll((els) => els.map((e) => e.getAttribute("data-hot")!));
    expect(hots.length).toBeGreaterThan(0);
    for (const hot of hots) expect(hot).toMatch(/^[sj]-\d+$/);
    await finding.locator(".net-link").first().click();
    await page.locator("#map-open-place").click();
    await expect(page).toHaveURL(new RegExp(`/${hrefOf(hots[0]).replace("?", "\\?")}$`));
  });

  test("a route cut off at a junction is a finding, and a fix on its street clears it and lowers the lead's count", async ({
    page,
  }) => {
    await openMap(page);
    const total = async () => Object.values(await counts(page)).reduce((a, b) => a + b, 0);
    const island = (street: string) =>
      shown(page).locator(`li[data-severity]:has-text("from this street"):has(.net-link[data-hot="${street}"])`);
    // a street that is a dead end (Rue Waverly, the end of the map to a junction) and walkable: it is found by its long block
    await openCheck(page, "walking");
    const block = shown(page).locator('li[data-severity]:has-text("Rue Waverly is")');
    const street = (await block.locator('.net-link[data-hot^="s-"]').first().getAttribute("data-hot"))!;
    const junction = (await block.locator('.net-link[data-hot^="j-"]').first().getAttribute("data-hot"))!;
    await expect(island(street)).toHaveCount(0);
    const clean = await total();
    // take the street's own crossing off the junction (the panel names the street of the one pressed): its arm
    // can no longer be crossed to, so the street is cut off there
    await page.goto("/" + hrefOf(junction));
    await expect(page.locator("#drawing")).toBeVisible();
    const crossings = page.locator('#drawing path.iso-hit[data-role="crossing"]');
    const n = await crossings.count();
    // The keys step through the junction's parts; the crossing across Rue Waverly says its street when it is selected.
    await page.locator("#wrap").focus();
    let removed = false;
    for (let i = 0; i < 6 * n && !removed; i++) {
      const before = await live(page).innerText();
      await page.keyboard.press("ArrowRight");
      // The status is emptied before it says what is selected: wait for the new words, not for the gap.
      await expect(live(page)).not.toHaveText(before);
      await expect(live(page)).toHaveText(/\S/);
      if (/^(Crossing on|Continuous sidewalk across) .*Waverly/.test(await live(page).innerText())) {
        await page.keyboard.press("Delete");
        removed = true;
      }
    }
    expect(removed, "the junction has a crossing across Rue Waverly").toBe(true);
    await expect(crossings).toHaveCount(n - 1);
    await page.goto("/map.html"); // leaving the page keeps what was waiting to be kept
    await mapReady(page);
    await expect(page.locator("#network-slot .net-rule")).toHaveCount(6);
    await openCheck(page, "walking");
    await expect(island(street)).toHaveCount(1);
    await expect(island(street).locator(`.net-link[data-hot="${junction}"]`)).toHaveCount(1);
    const cut = await total();
    expect(cut).toBeGreaterThan(clean);
    // a pedestrian street needs no crossing to be walked on to: making the street one joins the route
    await page.goto("/" + hrefOf(street));
    await transform(page, ".ped-item", "ped");
    await page.goto("/map.html");
    await mapReady(page);
    await expect(page.locator("#network-slot .net-rule")).toHaveCount(6);
    await openCheck(page, "walking");
    await expect(island(street)).toHaveCount(0);
    expect(await total()).toBeLessThan(cut);
    const now = await total();
    await expect(page.locator("#network-lead")).toContainText(`found ${now === 1 ? "1 finding" : now + " findings"}`);
  });

  test("are worded in French when the page is", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await openMap(page);
    await expect(page.getByRole("heading", { level: 2, name: "Vérifications du réseau" })).toBeVisible();
    await expect(row(page, "cycling")).toContainText("Liens cyclables");
    await openCheck(page, "cycling");
    await expect(page.getByRole("heading", { level: 2, name: "Liens cyclables" })).toBeVisible();
  });
});

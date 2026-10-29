import type { Page } from "@playwright/test";

import { expect, mapReady, test } from "./fixtures";

// The network checks look across the whole city: a section in the map page's left panel with a count for each check
// and the findings under it, each linking the places it concerns. They are worked out when the map reads the city, so
// an edit made on a street page shows on the map's next load.

const RULES = ["cycling", "access", "crossings", "lanes", "transit"];

const rule = (page: Page, id: string) => page.locator(`#network-slot .net-rule[data-rule="${id}"]`);

/** The count each check shows, by rule. */
const counts = (page: Page) =>
  page.evaluate(() =>
    Object.fromEntries(
      [...document.querySelectorAll<HTMLElement>("#network-slot .net-rule")].map((s) => [
        s.dataset.rule!,
        Number(s.querySelector("ul")!.getAttribute("data-count")),
      ]),
    ),
  );

async function openMap(page: Page) {
  await page.goto("/map.html");
  await mapReady(page);
  await expect(page.locator("#network-slot .net-rule")).toHaveCount(5);
}

test.describe("the network checks on the map page", () => {
  test("have a heading, a lead and a count and a list for each of the five checks", async ({ page }) => {
    await openMap(page);
    await expect(page.locator("#network-panel")).toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "Network checks" })).toBeVisible();
    for (const id of RULES) {
      const heading = rule(page, id).getByRole("heading", { level: 3 });
      await expect(heading).toBeVisible();
      // the count is part of the heading, so a screen reader hears it with the name
      const n = (await counts(page))[id];
      await expect(heading).toContainText(n === 0 ? "no findings" : n === 1 ? "1 finding" : `${n} findings`);
      await expect(rule(page, id).locator("ul > li")).toHaveCount(n);
    }
    const total = Object.values(await counts(page)).reduce((a, b) => a + b, 0);
    await expect(page.locator("#network-lead")).toHaveText(
      total === 0
        ? "The network checks found nothing across the streets and junctions."
        : total === 1
          ? "The network checks found 1 thing across the streets and junctions."
          : `The network checks found ${total} things across the streets and junctions.`,
    );
  });

  test("are counted apart from the places' own checks", async ({ page }) => {
    await openMap(page);
    const total = Object.values(await counts(page)).reduce((a, b) => a + b, 0);
    expect(total, "the Plateau has findings").toBeGreaterThan(0);
    // the Notes' lead and the status line count places that fail their own checks: none of the findings
    await expect(page.locator("#checks-lead")).not.toContainText("network");
    await expect(page.locator("#fit-slot")).not.toContainText(String(total) + " things");
  });

  test("each finding says why it failed and a link opens the place it concerns", async ({ page }) => {
    await openMap(page);
    const finding = page.locator("#network-slot li").first();
    await expect(finding.locator("b")).not.toBeEmpty();
    await expect(finding.locator(".net-severity")).toHaveText(/^(Problem|Notice)$/);
    const link = finding.locator("a").first();
    const href = await link.getAttribute("href");
    expect(href).toMatch(/^(street\.html\?street|intersection\.html\?junction)=\d+$/);
    await link.focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(new RegExp(`/${href!.replace("?", "\\?")}$`));
  });

  test("a finding goes when the street it concerns is edited, at the map's next load", async ({ page }) => {
    await openMap(page);
    // the smallest cycling link that is cut off, and the street at the head of it
    const target = await page.evaluate(() => {
      const items = [...document.querySelectorAll<HTMLElement>('#network-slot .net-rule[data-rule="cycling"] li')];
      const islands = items
        .map((li) => ({ links: [...li.querySelectorAll("a")].map((a) => a.getAttribute("href")!) }))
        .filter((i) => i.links.every((h) => h.startsWith("street.html")));
      islands.sort((a, b) => a.links.length - b.links.length);
      return { href: islands[0].links[0], size: islands[0].links.length };
    });
    const before = await rule(page, "cycling").innerText();
    await page.goto("/" + target.href);
    // take every bike lane off the street
    const bikes = page.locator("#drawing [data-role='seg']:has(rect.k-bike)");
    for (let guard = 0; (await bikes.count()) > 0 && guard < 4; guard++) {
      const index = await page
        .locator("#drawing [data-role='seg']")
        .evaluateAll((els) => els.findIndex((e) => e.querySelector("rect.k-bike")));
      await page.locator("#wrap").focus();
      await page.keyboard.press("Escape");
      for (let i = 0; i <= index; i++) await page.keyboard.press("ArrowRight");
      await page.keyboard.press("Delete");
    }
    await expect(bikes).toHaveCount(0);
    await page.goto("/map.html"); // leaving the page keeps what was waiting to be kept
    await mapReady(page);
    await expect(page.locator("#network-slot .net-rule")).toHaveCount(5);
    await expect(rule(page, "cycling").locator(`a[href="${target.href}"]`)).toHaveCount(0);
    expect(await rule(page, "cycling").innerText()).not.toEqual(before);
  });

  test("are worded in French when the page is", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await openMap(page);
    await expect(page.getByRole("heading", { level: 2, name: "Vérifications du réseau" })).toBeVisible();
    await expect(rule(page, "cycling").getByRole("heading", { level: 3 })).toContainText("Liens cyclables");
  });
});

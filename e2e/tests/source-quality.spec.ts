import type { Page } from "@playwright/test";

import { expect, mapReady, test } from "./fixtures";

// The Provenance tab of the map page's Notes panel lists where the imported OpenStreetMap data may be missing or wrong.
// Each finding says what looks doubtful, its source and its design impact, and links the places it concerns. It is
// counted apart from the design checks: the status line, the Notes lead and the network checks do not change.

const KINDS = ["name", "disconnected", "ends", "width", "speed"];

const group = (page: Page, id: string) => page.locator(`#source-slot .src-group[data-kind="${id}"]`);

async function openMap(page: Page) {
  await page.goto("/map.html");
  await mapReady(page);
}

async function openProvenance(page: Page) {
  await openMap(page);
  await page.getByRole("tab", { name: "Provenance" }).click();
  await expect(page.locator("#source-slot .src-group")).toHaveCount(KINDS.length);
}

test.describe("the Provenance tab on the map page", () => {
  test("is a tab of the Notes panel that the arrow keys reach, and is not shown until chosen", async ({ page }) => {
    await openMap(page);
    const tab = page.getByRole("tab", { name: "Provenance" });
    await expect(tab).toHaveAttribute("aria-selected", "false");
    await expect(page.locator("#p-provenance")).toBeHidden();
    await page.getByRole("tab", { name: "Checks" }).focus();
    await page.keyboard.press("End");
    await expect(tab).toBeFocused();
    await expect(tab).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#p-provenance")).toBeVisible();
    await expect(page.locator("#p-checks")).toBeHidden();
    await expect(page.getByRole("heading", { level: 2, name: "Source data" })).toBeVisible();
    await expect(page.getByRole("tabpanel", { name: "Provenance" })).toBeVisible();
  });

  test("has a heading and a count for each kind and a lead that adds them up", async ({ page }) => {
    await openProvenance(page);
    let total = 0;
    for (const id of KINDS) {
      const heading = group(page, id).getByRole("heading", { level: 3 });
      await expect(heading).toBeVisible();
      expect((await heading.textContent())!.replace(/\s+/g, " ")).toMatch(
        /^[^:]+: (nothing found|\d+ possible gaps?)$/,
      );
      const n = Number(await group(page, id).locator("ul.places-list").getAttribute("data-count"));
      total += n;
      await expect(group(page, id).locator("ul.places-list > li")).toHaveCount(Math.min(n, 50));
    }
    expect(total, "the Plateau has findings").toBeGreaterThan(0);
    await expect(page.locator("#source-lead")).toHaveText(
      total === 1
        ? "1 place in the source data may be missing or wrong."
        : `${total} places in the source data may be missing or wrong.`,
    );
  });

  test("each finding says its source and its design impact, and is no design failure", async ({ page }) => {
    await openProvenance(page);
    const finding = page.locator("#source-slot li[data-kind]").first();
    await expect(finding.locator("b")).not.toBeEmpty();
    await expect(finding.locator(".src-line").nth(0)).toContainText(/^Source: \S/);
    await expect(finding.locator(".src-line").nth(1)).toContainText(/^Design impact: \S/);
    await expect(page.locator("#source-slot li.bad")).toHaveCount(0);
    await expect(page.locator("#source-slot [data-severity]")).toHaveCount(0);
    await expect(page.locator("#p-provenance")).not.toContainText("Problem");
    await expect(page.locator("#p-provenance")).toContainText("not design failures");
  });

  test("a width finding says the importer cannot tell a tagged width from the default", async ({ page }) => {
    await openProvenance(page);
    const widths = group(page, "width").locator("ul.places-list > li");
    if ((await widths.count()) > 0) {
      await expect(widths.first()).toContainText("cannot tell a width the road is tagged with");
    }
    await expect(page.locator("#p-provenance")).toContainText("cannot tell a width the road is tagged with");
  });

  test("a link opens the place the finding concerns", async ({ page }) => {
    await openProvenance(page);
    const link = page.locator("#source-slot li[data-kind] a").first();
    const href = await link.getAttribute("href");
    expect(href).toMatch(/^(street\.html\?street|intersection\.html\?junction)=\d+$/);
    await link.focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(new RegExp(`/${href!.replace("?", "\\?")}$`));
  });

  test("leaves the status line, the Notes lead and the Checks counts as they were", async ({ page }) => {
    await openMap(page);
    const read = () =>
      page.evaluate(() => ({
        status: document.getElementById("fit-slot")!.innerText,
        lead: document.getElementById("checks-lead")!.innerText,
        network: document.getElementById("network-lead")!.innerText,
        checks: [...document.querySelectorAll("#network-slot .net-rule ul")].map((u) => u.getAttribute("data-count")),
      }));
    const before = await read();
    await page.getByRole("tab", { name: "Provenance" }).click();
    await expect(page.locator("#source-slot .src-group")).toHaveCount(KINDS.length);
    expect(await read()).toEqual(before);
    for (const text of Object.values(before)) {
      expect(JSON.stringify(text)).not.toMatch(/possible gap|source data/i);
    }
  });
});

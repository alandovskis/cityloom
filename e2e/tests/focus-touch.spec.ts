import type { Locator, Page } from "@playwright/test";

import { expect, mapReady, openJunction, openStreet, test } from "./fixtures";

/** What a keyboard user sees on a focused control: a ring of its own, not only a tint. */
const ring = (field: Locator) =>
  field.evaluate((el) => {
    const s = getComputedStyle(el);
    const probe = document.createElement("span");
    probe.style.color = "var(--accent)";
    document.body.append(probe);
    const accent = getComputedStyle(probe).color;
    probe.remove();
    return { style: s.outlineStyle, width: parseFloat(s.outlineWidth), color: s.outlineColor, accent };
  });

const touchAction = (el: Locator) => el.first().evaluate((e) => getComputedStyle(e).touchAction);

for (const theme of ["light", "dark"]) {
  test.describe(`the focus ring, ${theme} theme`, () => {
    test("the home search field has a 2px ring in the accent", async ({ page }) => {
      await page.goto("/");
      await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
      const field = page.locator(".search input").first();
      await field.focus();
      const r = await ring(field);
      expect(r.style).not.toBe("none");
      expect(r.width).toBeGreaterThanOrEqual(2);
      expect(r.color).toBe(r.accent);
    });

    test("the street's width field has a 2px ring in the accent", async ({ page }) => {
      await openStreet(page);
      await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
      await page.locator("#wrap").focus();
      await page.keyboard.press("ArrowRight");
      const field = page.locator("#inspector .wfield input");
      await field.focus();
      const r = await ring(field);
      expect(r.style).not.toBe("none");
      expect(r.width).toBeGreaterThanOrEqual(2);
      expect(r.color).toBe(r.accent);
    });
  });
}

test.describe("a touch drag moves the piece, it does not scroll the page", () => {
  test("on the junction, every grip takes no browser pan", async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    for (const role of ["grip-arm", "grip-corner", "grip-crossing"]) {
      const grips = page.locator(`#drawing [data-role="${role}"]`);
      if ((await grips.count()) === 0) continue;
      expect(await touchAction(grips), role).toBe("none");
    }
    expect(await page.locator('#drawing [data-role="grip-arm"]').count()).toBeGreaterThan(0);
  });
});

const pages: [string, (p: Page) => Promise<unknown>][] = [
  ["home", (p) => p.goto("/")],
  [
    "map",
    async (p) => {
      await p.goto("/map.html");
      await mapReady(p);
    },
  ],
  ["street", openStreet],
  ["junction", openJunction],
];
for (const [name, open] of pages) {
  test(`the settings menu fits a 360px phone (${name})`, async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 740 });
    await open(page);
    // The shell is bound once the footer holds the version.
    await expect(page.locator("[data-version]").first()).not.toBeEmpty();
    await page.locator("#account-btn").click();
    await expect(page.locator("#account-menu")).toBeVisible();
    const menu = (await page.locator("#account-menu").boundingBox())!;
    expect(menu.x).toBeGreaterThanOrEqual(0);
    expect(menu.x + menu.width).toBeLessThanOrEqual(360);
  });
}

import type { Locator } from "@playwright/test";

import { expect, openStreet, test } from "./fixtures";

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
      await page.locator('#drawing [data-role="seg"]').nth(1).click();
      const field = page.locator("#inspector .wfield input");
      await field.focus();
      const r = await ring(field);
      expect(r.style).not.toBe("none");
      expect(r.width).toBeGreaterThanOrEqual(2);
      expect(r.color).toBe(r.accent);
    });
  });
}

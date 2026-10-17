import { expect, openJunction, openStreet, test } from "./fixtures";

// Keyboard and small-window behaviour that the audit (WCAG 2.2 AA) found wanting.

for (const [name, open] of [
  ["street", openStreet],
  ["junction", openJunction],
] as const) {
  test(`the ${name} editor's first stop is a skip link that shows when focused and leads to the drawing`, async ({
    page,
  }) => {
    await open(page);
    await expect(page.locator("#wrap")).toBeVisible();
    const skip = page.locator(".skip-link");
    await expect(skip).toHaveText("Skip to the drawing");
    expect((await skip.boundingBox())!.y + (await skip.boundingBox())!.height).toBeLessThanOrEqual(0);
    await page.keyboard.press("Tab");
    await expect(skip).toBeFocused();
    expect((await skip.boundingBox())!.y).toBeGreaterThanOrEqual(0);
    await page.keyboard.press("Enter");
    await expect(page.locator("#wrap")).toBeFocused();
  });
}

test("at 400% zoom the sticky bar does not cover the control the keyboard has reached", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 256 });
  await openStreet(page);
  await expect(page.locator("#wrap")).toBeVisible();
  // The page keeps how far the bar covers the window, and scrolls controls clear of it.
  const covered = await page.evaluate(() => parseFloat(document.documentElement.style.getPropertyValue("--bar-stuck")));
  const bar = (await page.locator(".bar").boundingBox())!;
  expect(covered).toBeGreaterThanOrEqual(bar.y + bar.height);
  for (const id of ["#add-btn", "#view-section", "#wrap"]) {
    await page.locator(id).focus();
    const box = (await page.locator(id).boundingBox())!;
    const barNow = (await page.locator(".bar").boundingBox())!;
    // Not hidden by the bar (WCAG 2.4.11): a part of it, or all of it when it is small, is clear of the bar.
    const clear = Math.min(box.y + box.height, 256) - Math.max(box.y, barNow.y + barNow.height);
    expect(clear, `${id} is clear of the bar`).toBeGreaterThanOrEqual(Math.min(box.height, 24) - 1);
  }
});

test("under 380 px the end buttons do not run over the logo", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await openStreet(page);
  const logo = (await page.locator(".brand").boundingBox())!;
  for (const id of ["#share-btn", "#help-btn", "#account-btn"]) {
    const b = (await page.locator(id).boundingBox())!;
    expect(b.y, id).toBeGreaterThanOrEqual(logo.y + logo.height - 1);
  }
});

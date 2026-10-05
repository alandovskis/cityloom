import { expect, test } from "./fixtures";

test("the browser can draw the map: MapLibre starts and its canvas has a WebGL context", async ({ page }) => {
  await page.goto("/map.html");
  await expect(page.locator("#basemap canvas")).toBeVisible();
  // The map is published only once MapLibre has started, which it cannot without WebGL; asked after that, the
  // canvas gives back the context MapLibre made rather than a new one.
  expect(await page.evaluate(() => !!(window as any).cityloomMap)).toBe(true);
  const gl = await page.evaluate(() => {
    const c = document.querySelector<HTMLCanvasElement>("#basemap canvas")!;
    return !!(c.getContext("webgl2") || c.getContext("webgl"));
  });
  expect(gl).toBe(true);
});

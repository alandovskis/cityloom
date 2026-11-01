import type { Page } from "@playwright/test";

import { expect, live, openStreet, placesOnMap, test } from "./fixtures";

// The street page draws the street in 2.5D only (#139). A press on a piece selects it; its width and kind are
// edited in the inspector and with the keys, since a perspective has no handles to drag.

const pieces = (page: Page) => page.locator("#drawing .iso-piece");
const pressPiece = (page: Page, i: number) =>
  pieces(page).nth(i).locator(".iso-top, .iso-front, polygon").first().click({ force: true });

test.describe("the street in 2.5D", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("is the only drawing: no toggle, no section, no handles, and V changes nothing", async ({ page }) => {
    await expect(page.locator("#drawing.iso")).toBeVisible();
    await expect(page.locator("#view-mode")).toHaveCount(0);
    await expect(page.locator('#drawing [data-role="seg"]')).toHaveCount(0);
    await expect(page.locator('#drawing [data-role="handle"]')).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /in 2\.5D, along a 40 m stretch\. \d+ segments?\.( Speed limit \d+ km\/h\.)?$/,
    );
    await page.locator("#wrap").focus();
    await page.keyboard.press("v");
    await expect(page.locator("#drawing.iso")).toBeVisible();
    await expect(live(page)).toHaveText("");
  });

  test("a view kept by an earlier visit, the section included, does not change what opens", async ({ page }) => {
    await page.evaluate(() => localStorage.setItem("cityloom-street-view", "section"));
    await page.reload();
    await expect(page.locator("#drawing.iso")).toBeVisible();
    await expect(page.locator('#drawing [data-role="seg"]')).toHaveCount(0);
  });

  test("a press on a piece selects it, outlines it, announces it and shows its details", async ({ page }) => {
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
    await pressPiece(page, 0);
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    await expect(live(page)).toHaveText(/, [\d.]+ m, \d+ of \d+$/);
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    await expect(page.locator("#wrap")).toBeFocused();
  });

  test("every piece can be pressed, the narrow far ones too", async ({ page }) => {
    const n = await pieces(page).count();
    expect(n).toBeGreaterThan(0);
    for (let i = 0; i < n; i++) {
      await page.keyboard.press("Escape");
      await pressPiece(page, i);
      await expect(page.locator("#drawing .iso-sel"), `piece ${i} is selected`).toHaveCount(1);
    }
  });

  test("the 2.5D follows an edit made with the keys and outlines the selected piece", async ({ page }) => {
    const tops = () =>
      page.locator("#drawing .iso-top").evaluateAll((els) => els.map((e) => e.getAttribute("points")).join("|"));
    const before = await tops();
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    await page.keyboard.press("+");
    await expect.poll(tops).not.toBe(before);
    await expect(page.locator("#undo")).toBeEnabled();
    await page.keyboard.press("Control+z");
    await expect.poll(tops).toBe(before);
  });

  test("a piece is made wider in the inspector's width field", async ({ page }) => {
    await pressPiece(page, 0);
    const width = page.locator("#width");
    await expect(width).toBeVisible();
    const before = Number(await width.inputValue());
    await width.fill(String(before + 0.1));
    await width.press("Enter");
    await expect(page.locator("#undo")).toBeEnabled();
    expect(Number(await width.inputValue())).toBeCloseTo(before + 0.1, 1);
  });

  test("choosing another surface material in the inspector changes the selected piece's top in 2.5D", async ({
    page,
  }) => {
    const tops = () =>
      page.locator("#drawing .iso-top").evaluateAll((els) =>
        els.map((el) => {
          const s = getComputedStyle(el);
          const hatch = el.nextElementSibling?.getAttribute("fill") ?? "";
          return { class: el.getAttribute("class") ?? "", fill: s.fill, hatch };
        }),
      );
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    const before = await tops();
    for (const t of before) {
      const material = /mat-(\w+)/.exec(t.class)?.[1];
      expect(material, t.class).toBeTruthy();
      expect(t.hatch).toBe(`url(#iso-m-${material})`);
    }
    const other = page.locator('#i-h-surface ~ [role="radiogroup"] [role="radio"][aria-checked="false"]').first();
    await other.click();
    const after = await tops();
    expect(after).toHaveLength(before.length);
    const changed = after.filter((t, i) => t.class !== before[i].class);
    expect(changed).toHaveLength(1);
    expect(changed[0].fill).not.toBe(before[after.indexOf(changed[0])].fill);
    expect(changed[0].hatch).toMatch(/^url\(#iso-m-/);
    // The pattern is laid on the ground: its definition carries a transform.
    const id = /#(iso-m-\w+)/.exec(changed[0].hatch)![1];
    await expect(page.locator(`#drawing pattern#${id}`)).toHaveAttribute("patternTransform", /^matrix\(/);
  });
});

test("a street with opposing lanes shows yellow paint in 2.5D, and symbols in place of the section's arrows", async ({
  page,
}) => {
  await page.goto("/map.html");
  const streets = (await placesOnMap(page)).streets;
  let yellow = 0;
  // The first streets of the Plateau: stop at the first with a line between lanes that run opposite ways.
  for (const href of streets.slice(0, 8)) {
    await page.goto("/" + href);
    await expect(page.locator("#drawing .iso-markings")).toHaveCount(1);
    expect(await page.locator("#drawing .iso-mark").count(), "symbols take the arrows' place").toBeGreaterThan(0);
    yellow = await page.locator("#drawing .iso-paint.yellow").count();
    if (yellow > 0) break;
  }
  expect(yellow, "some street of the fixture has opposing lanes").toBeGreaterThan(0);
  // The paint lies on the road: it takes no pointer, so the piece under it is still the one hovered.
  const pointer = await page
    .locator("#drawing .iso-paint")
    .first()
    .evaluate((el) => getComputedStyle(el).pointerEvents);
  expect(pointer).toBe("none");
  // And the paint follows the theme tokens, in both themes: yellow here, with the dark hairline in both.
  const yellowPaint = page.locator("#drawing .iso-paint.yellow").first();
  const look = () =>
    yellowPaint.evaluate((el) => {
      const s = getComputedStyle(el);
      return [s.fill, s.stroke];
    });
  await page.emulateMedia({ colorScheme: "light" });
  expect(await look()).toEqual(["rgb(227, 166, 0)", "rgb(16, 21, 26)"]);
  await page.emulateMedia({ colorScheme: "dark" });
  const [darkFill, darkEdge] = await look();
  expect(darkFill).toBe("rgb(240, 192, 46)");
  expect(darkEdge, "the hairline is in both themes: the materials are the same colour in either").toBe(
    "rgb(16, 21, 26)",
  );
  // A material is the same colour in both themes, and so is its pattern.
  const road = () =>
    page
      .locator("#drawing .iso-piece .iso-top")
      .first()
      .evaluate((el) => getComputedStyle(el).fill);
  await page.emulateMedia({ colorScheme: "light" });
  const lightRoad = await road();
  await page.emulateMedia({ colorScheme: "dark" });
  expect(await road()).toBe(lightRoad);
  // In forced colours the paint is the text colour and the roadway under it is not.
  await page.emulateMedia({ forcedColors: "active" });
  const forced = await page.evaluate(() => {
    const paint = getComputedStyle(document.querySelector("#drawing .iso-paint.yellow")!);
    const road = getComputedStyle(document.querySelector("#drawing .iso-piece .iso-top")!);
    return [paint.fill, paint.stroke, road.fill];
  });
  expect(forced[0]).not.toBe(forced[2]);
  expect(forced[1]).toBe("none");
});

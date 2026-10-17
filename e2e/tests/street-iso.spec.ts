import type { Page } from "@playwright/test";

import { expect, live, openStreet, placesOnMap, test } from "./fixtures";

const iso = (page: Page) => page.locator("#view-iso");
const section = (page: Page) => page.locator("#view-section");

// The street page's drawing shows the section, which is edited, or the street in 2.5D, which follows it.

test.describe("the street in 2.5D", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("the toggle shows the street in 2.5D without handles, and V brings the section back", async ({ page }) => {
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    const n = await page.locator('#drawing [data-role="seg"]').count();
    await iso(page).click();
    await expect(iso(page)).toHaveAttribute("aria-pressed", "true");
    await expect(section(page)).toHaveAttribute("aria-pressed", "false");
    await expect(live(page)).toHaveText("Showing the street in 2.5D");
    await expect(page.locator("#drawing .iso-top")).toHaveCount(n);
    await expect(page.locator('#drawing [data-role="handle"]')).toHaveCount(0);
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /in 2\.5D, along a 40 m stretch\. \d+ segments?\.$/,
    );
    await page.keyboard.press("v");
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    await expect(live(page)).toHaveText("Showing the section");
    await expect(page.locator('#drawing [data-role="handle"]').first()).toBeVisible();
  });

  test("the 2.5D follows an edit made with the keys and outlines the selected piece", async ({ page }) => {
    const second = await page.locator('#drawing [data-role="seg"]').nth(1).getAttribute("data-uid");
    await iso(page).click();
    const top = page.locator(`#drawing .iso-piece[data-uid="${second}"] .iso-top`);
    const before = await top.getAttribute("points");
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    await page.keyboard.press("+");
    await expect(top).not.toHaveAttribute("points", before ?? "");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator('#drawing [data-role="handle"]')).toHaveCount(0);
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
    await iso(page).click();
    const before = await tops();
    for (const t of before) {
      const material = /mat-(\w+)/.exec(t.class)?.[1];
      expect(material, t.class).toBeTruthy();
      expect(t.hatch).toBe(`url(#iso-m-${material})`);
    }
    await section(page).click();
    const other = page.locator('#i-h-surface ~ [role="radiogroup"] [role="radio"][aria-checked="false"]').first();
    await other.click();
    await iso(page).click();
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

  test("the view chosen is kept across a reload", async ({ page }) => {
    await iso(page).click();
    await page.reload();
    await expect(iso(page)).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
  });

  test("V typed in the width field is a letter, not the view", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Enter");
    const width = page.locator("#width");
    await expect(width).toBeFocused();
    await width.press("v");
    await expect(section(page)).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#drawing .iso-top")).toHaveCount(0);
  });

  test("in French the toggle reads Coupe and 2,5D and the change is said in French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await page.keyboard.press("Escape");
    await expect(section(page)).toHaveText("Coupe");
    await expect(iso(page)).toHaveText("2,5D");
    await iso(page).click();
    await expect(live(page)).toHaveText("Rue affichée en 2,5D");
  });
});

test("at 400 px the Notes edge tab leaves the toggle clear, wherever the page is scrolled", async ({ page }) => {
  // A short window puts the tab, half-way down it, level with the toggle when the page is at the top.
  await page.setViewportSize({ width: 400, height: 700 });
  await openStreet(page);
  await expect(iso(page)).toBeVisible();
  const tab = (await page.locator(".edge-tab.right").boundingBox())!;
  for (const id of ["#view-section", "#view-iso"]) {
    const b = (await page.locator(id).boundingBox())!;
    // The tab is fixed and the toggle scrolls past it, so only across can they meet.
    expect(b.x + b.width, id).toBeLessThanOrEqual(tab.x);
  }
});

test("in forced colours the view shown is still marked as pressed", async ({ page }) => {
  // Forced colours drop the pressed button's background and shadow; an outline in Highlight stays.
  await page.emulateMedia({ forcedColors: "active" });
  await openStreet(page);
  const outline = (id: string) => page.locator(id).evaluate((b) => getComputedStyle(b).outlineStyle);
  expect(await outline("#view-section")).toBe("solid");
  expect(await outline("#view-iso")).toBe("none");
  await iso(page).click();
  await page.mouse.click(1, 1);
  expect(await outline("#view-iso")).toBe("solid");
  expect(await outline("#view-section")).toBe("none");
});

test("the toggle is not printed", async ({ page }) => {
  await openStreet(page);
  await expect(page.locator("#view-mode")).toBeVisible();
  await page.emulateMedia({ media: "print" });
  await expect(page.locator("#view-mode")).toBeHidden();
});

test("the toggle sits on the street name's line, so the page's head is no taller for it", async ({ page }) => {
  // A row of its own, under the tools, pushed the drawing out of the window wherever the text is wider (CI's fonts).
  await openStreet(page);
  await expect(iso(page)).toBeVisible();
  const toggle = (await page.locator("#view-mode").boundingBox())!;
  const name = (await page.locator("#street-name").boundingBox())!;
  const sub = (await page.locator("#street-sub").boundingBox())!;
  expect(toggle.y + toggle.height).toBeLessThanOrEqual(sub.y + 1);
  expect(toggle.x).toBeGreaterThanOrEqual(name.x + name.width);
  expect(toggle.height).toBeLessThanOrEqual(name.height + 6);
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
    await iso(page).click();
    await expect(iso(page)).toHaveAttribute("aria-pressed", "true");
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

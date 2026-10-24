import { expect, mapReady, openStreet, placesOnMap, test } from "./fixtures";
import type { Page } from "@playwright/test";

// What a resident edits is kept in one document, `cityloom-edits`, under the OpenStreetMap-derived key of the
// place (`w<way>~n<end>~n<end>` for a street, `w<way>~n<end>~x~<heading>` where the area's box cuts it, `n<node>`
// for a junction), so an edit follows its place into another area that holds it.

const DEFAULT_AREA = "45.5261,-73.5978";
// about 100 m north of the default area: the two overlap almost entirely
// a street well inside both areas (the streets are numbered from one corner of the first)
const MIDDLE = 10;
const NORTH_AREA = "area=45.5270,-73.5978&name=North";
// 150 m east of the default area: what lies near its east edge is whole there, or cut elsewhere
const EAST_AREA = "area=45.5261,-73.5959&name=East";

const selectFirstPiece = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
};

/** The width of the first piece of the street on the page, as the inspector shows it. */
const firstPieceWidth = async (page: Page) => {
  await selectFirstPiece(page);
  await page.keyboard.press("Enter");
  const width = page.locator("#inspector input").first();
  await expect(width).toBeFocused();
  return width.inputValue();
};

const edits = (page: Page) => page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? "null"));

/** Widens the first piece of a street and leaves the page, which keeps the edit. Says which street it
 *  was, and the width its piece has now. */
const editStreet = async (page: Page, n: number | string = 0) => {
  let href: string;
  if (typeof n === "number") href = await openStreet(page, n);
  else await page.goto("/" + (href = n));
  const title = ((await page.locator("#tb-street").textContent()) ?? "").trim();
  await selectFirstPiece(page);
  await page.keyboard.press("+");
  await expect(page.locator("#tb-changes")).toHaveText("1");
  await page.keyboard.press("Enter");
  const input = page.locator("#inspector input").first();
  await expect(input).toBeFocused();
  const after = await input.inputValue();
  // The edit is kept after a debounce; leaving the page flushes it.
  await page.goto("/map.html");
  await mapReady(page);
  return { href, title, after };
};

/** Every position the map has of each place, by its hot id (`s-<uid>`, `j-<uid>`), as [lon, lat]. */
const placeShapes = async (page: Page): Promise<Record<string, number[][]>> => {
  await placesOnMap(page);
  return page.evaluate(() => {
    const out: Record<string, number[][]> = {};
    for (const f of (window as any).cityloomMap.querySourceFeatures("places")) {
      const g = f.geometry;
      const points =
        g.type === "Point" ? [g.coordinates] : g.type === "LineString" ? g.coordinates : g.coordinates.flat();
      (out[f.properties.hot] ??= []).push(...points);
    }
    return out;
  });
};

/** How far apart two [lon, lat] positions are, in metres (near enough at this size). */
const metres = (a: number[], b: number[]) =>
  Math.hypot((a[0] - b[0]) * 111_320 * Math.cos((a[1] * Math.PI) / 180), (a[1] - b[1]) * 111_320);

const href = (hot: string) =>
  hot.startsWith("j-") ? `intersection.html?junction=${hot.slice(2)}` : `street.html?street=${hot.slice(2)}`;

/** Of the places of `kind` (`s` or `j`), the one that reaches furthest east. */
const eastmost = (shapes: Record<string, number[][]>, kind: string) =>
  Object.entries(shapes)
    .filter(([hot]) => hot.startsWith(kind + "-"))
    .map(([hot, points]) => [hot, Math.max(...points.map((p) => p[0]))] as const)
    .sort((a, b) => b[1] - a[1])[0][0];

const switchTo = async (page: Page, query: string, name: string) => {
  await page.evaluate((q) => localStorage.setItem("cityloom-area", q), query);
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText(name);
};

const provenanceState = async (page: Page) => {
  await page.locator("#t-provenance").click();
  return (await page.locator("#tb-state").textContent())?.trim();
};

test("an edit is kept under the street's OSM key", async ({ page }) => {
  await editStreet(page);
  const doc = await edits(page);
  // The speed is unchanged from OpenStreetMap's, so the record holds none (it follows the import): version 5.
  expect(doc.version).toBe(5);
  expect(Object.values<any>(doc.streets)[0].street).not.toHaveProperty("speed_kmh");
  expect(Object.keys(doc.streets)).toHaveLength(1);
  expect(Object.keys(doc.streets)[0]).toMatch(/^w\d+~n\d+~(n\d+|x~[nesw])$/);
  expect(Object.keys(doc.junctions ?? {})).toEqual([]);
  const keys = await page.evaluate(() => Object.keys(localStorage));
  expect(keys.filter((k) => k.startsWith("cityloom-city:"))).toEqual([]);
});

test("a speed the designer changed is kept with the record: version 7", async ({ page }) => {
  const href = await openStreet(page, 0);
  const speed = page.locator("#speed-limit");
  const chosen = (await speed.inputValue()) === "10" ? "20" : "10";
  await speed.selectOption(chosen);
  await expect(speed).toHaveValue(chosen);
  // The edit is kept after a debounce; leaving the page flushes it.
  await page.goto("/map.html");
  await mapReady(page);
  const doc = await edits(page);
  expect(doc.version).toBe(7);
  expect(Object.values<any>(doc.streets)[0].street.speed_kmh).toBe(Number(chosen));
  await page.goto("/" + href);
  await expect(speed).toHaveValue(chosen);
});

test("the edit is there from another area of the same tile", async ({ page }) => {
  test.setTimeout(180_000);
  const { title, after } = await editStreet(page, MIDDLE);
  const edited = Object.values<any>((await edits(page)).streets)[0].street.segments[0].width_mm;
  expect(edited).toBeGreaterThan(0);

  await page.evaluate((query) => localStorage.setItem("cityloom-area", query), NORTH_AREA);
  await page.goto("/map.html");
  await expect(page.locator("#title-block")).toContainText("North");
  const streets = (await placesOnMap(page)).streets;

  // The street's number is not the same here: find it by its name, then by the width it now has.
  let found: string | null = null;
  const seen: string[] = [];
  for (const href of streets) {
    await page.goto("/" + href);
    if (((await page.locator("#tb-street").textContent()) ?? "").trim() !== title) continue;
    seen.push(await firstPieceWidth(page));
    await page.locator("#t-provenance").click();
    const edited = (await page.locator("#tb-state").textContent()) === "Edited";
    if (edited && seen[seen.length - 1] === after) {
      found = href;
      break;
    }
  }
  expect(
    found,
    `the edited street is in the other area, with its edit (edited to ${after}; ${title}; saw ${seen})`,
  ).not.toBeNull();
});

test("an old per-area save is no longer read, and is left as it was", async ({ page }) => {
  // The legacy save is made from what a visit wrote: the edited street's record, re-keyed by its number in the
  // area, as a version that kept edits per area wrote it.
  const { href } = await editStreet(page);
  const uid = new URL(href, "http://x/").searchParams.get("street")!;
  const record = Object.values<any>((await edits(page)).streets)[0];
  const [key, legacy] = await page.evaluate(
    ([area, uid, street]) => {
      const text = JSON.stringify({ version: 2, streets: { [uid]: street }, junctions: {} });
      localStorage.setItem(`cityloom-city:${area}`, text);
      localStorage.removeItem("cityloom-edits");
      return [`cityloom-city:${area}`, text];
    },
    [DEFAULT_AREA, uid, record.street] as const,
  );
  await page.goto("/map.html");
  await mapReady(page);
  const doc = await edits(page);
  expect(Object.keys(doc?.streets ?? {}), "its edit is not moved").toEqual([]);
  expect(await page.evaluate((k) => localStorage.getItem(k), key), "the old save is left as it was").toBe(legacy);
});

test("a junction near the edge of the area keeps its edit in an area shifted east", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/map.html");
  const shapes = await placeShapes(page);
  const hot = eastmost(shapes, "j");
  const at = shapes[hot][0];
  await page.goto("/" + href(hot));
  expect(await provenanceState(page)).toBe("As imported");
  await page.locator("#t-space").click();
  await page.locator("#turns button.turn").first().click();
  expect(await provenanceState(page)).toBe("Edited");
  await page.goto("/map.html");
  await mapReady(page);
  expect(Object.keys((await edits(page)).junctions)).toHaveLength(1);

  await switchTo(page, EAST_AREA, "East");
  const there = Object.entries(await placeShapes(page)).filter(
    ([h, points]) => h.startsWith("j-") && metres(points[0], at) < 3,
  );
  expect(there, "the junction is in the other area").toHaveLength(1);
  await page.goto("/" + href(there[0][0]));
  expect(await provenanceState(page), "and shows the edit made in the first").toBe("Edited");
});

test("a street cut by the edge of the area keeps its edit in an area shifted east", async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto("/map.html");
  const shapes = await placeShapes(page);
  const hot = eastmost(shapes, "s");
  const { title, after } = await editStreet(page, href(hot));
  const keys = Object.keys((await edits(page)).streets);
  expect(keys, "the street runs to the east edge, where the box cuts it").toEqual([
    expect.stringMatching(/~x~[nesw]$/),
  ]);

  await switchTo(page, EAST_AREA, "East");
  // the street here is made from the same way: it shares positions with the one edited
  const near = Object.entries(await placeShapes(page)).filter(
    ([h, points]) => h.startsWith("s-") && points.some((p) => shapes[hot].some((q) => metres(p, q) < 3)),
  );
  const seen: string[] = [];
  let found: string | null = null;
  for (const [h] of near) {
    await page.goto("/" + href(h));
    if (((await page.locator("#tb-street").textContent()) ?? "").trim() !== title) continue;
    const width = await firstPieceWidth(page);
    seen.push(width);
    if (width === after && (await provenanceState(page)) === "Edited") {
      found = h;
      break;
    }
  }
  expect(
    found,
    `the edited street is in the other area with its edit (${title}, edited to ${after}; saw ${seen})`,
  ).not.toBeNull();
});

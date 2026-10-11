import { expect, mapReady, openStreet, placesOnMap, test } from "./fixtures";
import type { Page } from "@playwright/test";

// What a resident edits is kept in one document, `cityloom-edits`, under the OpenStreetMap-derived key of the
// place (`w<way>~n<end>~n<end>` for a street), so an edit follows its street into any other area of the metro.

const DEFAULT_AREA = "45.5261,-73.5978";
// about 100 m north of the default area: the two overlap almost entirely
// a street well inside both areas (the streets are numbered from one corner of the first)
const MIDDLE = 10;
const NORTH_AREA = "area=45.5270,-73.5978&name=North";

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
const editStreet = async (page: Page, n = 0) => {
  const href = await openStreet(page, n);
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

test("an edit is kept under the street's OSM key", async ({ page }) => {
  await editStreet(page);
  const doc = await edits(page);
  expect(doc.version).toBe(3);
  expect(Object.keys(doc.streets)).toHaveLength(1);
  expect(Object.keys(doc.streets)[0]).toMatch(/^w\d+~n\d+~n\d+$/);
  expect(Object.keys(doc.junctions)).toEqual([]);
  const keys = await page.evaluate(() => Object.keys(localStorage));
  expect(keys.filter((k) => k.startsWith("cityloom-city:"))).toEqual([]);
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

test("a profile that has only the old per-area save keeps its edits", async ({ page }) => {
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
  expect(Object.values<any>(doc.streets)).toEqual([record]);
  expect(await page.evaluate((k) => localStorage.getItem(k), key), "the old save is left as it was").toBe(legacy);
});

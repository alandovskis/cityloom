import { readFile } from "node:fs/promises";

import type { Browser, Page } from "@playwright/test";

import { anotherBrowser, expect, mapReady, openStreet, test } from "./fixtures";

// The design archive (issue #74): Export design and Import design on the Account page. The Account page works on
// this device's storage, so no account server is involved. (`archive.spec.ts` is the area archive's.)

const EDITS = "cityloom-edits";

const edits = (page: Page) => page.evaluate((key) => localStorage.getItem(key), EDITS);

/** Widens the first piece of street `n` `times` times and leaves the page, which keeps the edit. */
async function editStreet(page: Page, times: number) {
  await openStreet(page, 0);
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
  for (let i = 0; i < times; i++) await page.keyboard.press("+");
  await expect(page.locator("#tb-changes")).toHaveText(String(times));
  await page.goto("/map.html");
  await mapReady(page);
}

async function exported(page: Page): Promise<{ name: string; text: string }> {
  await page.goto("/account.html");
  const download = page.waitForEvent("download");
  await page.locator("#archive-export").click();
  const file = await download;
  const path = await file.path();
  return { name: file.suggestedFilename(), text: await readFile(path!, "utf8") };
}

async function importFile(page: Page, name: string, text: string) {
  await page.goto("/account.html");
  const chooser = page.waitForEvent("filechooser");
  await page.locator("#archive-import").click();
  await (await chooser).setFiles({ name, mimeType: "application/json", buffer: Buffer.from(text) });
}

async function second(browser: Browser, baseURL: string | undefined) {
  return anotherBrowser(browser, baseURL!);
}

test("a street edit survives export and import on a fresh device", async ({ page, browser, baseURL }) => {
  await editStreet(page, 1);
  const { name, text } = await exported(page);
  expect(name).toMatch(/^cityloom-\d{4}-\d{2}-\d{2}\.cityloom-design\.json$/);
  expect(JSON.parse(text)).toMatchObject({ kind: "cityloom-design", format_version: 1 });
  await expect(page.locator("#archive-result")).toContainText("Saved");

  const { context, page: other } = await second(browser, baseURL);
  await other.goto("/account.html");
  expect(await edits(other)).toBeNull();
  await importFile(other, name, text);
  await expect(other.locator("#archive-result")).toContainText("Import finished. Added: 1.");
  expect(JSON.parse((await edits(other))!)).toEqual(JSON.parse((await edits(page)) ?? "null"));

  // the same archive again finds everything already here and changes nothing
  const before = await edits(other);
  await importFile(other, name, text);
  await expect(other.locator("#archive-result")).toContainText("Nothing to import");
  expect(await edits(other)).toBe(before);
  await context.close();
});

test("a place that differs is not replaced until the person chooses", async ({ page, browser, baseURL }) => {
  await editStreet(page, 1);
  const { name, text } = await exported(page);

  const { context, page: other } = await second(browser, baseURL);
  await editStreet(other, 2);
  const mine = await edits(other);
  expect(mine).not.toBeNull();
  expect(mine).not.toBe(await edits(page));

  await importFile(other, name, text);
  await expect(other.locator("#archive-confirm")).toBeVisible();
  await expect(other.locator("#archive-keep")).toBeFocused();
  expect(await edits(other)).toBe(mine);

  await other.locator("#archive-cancel").click();
  await expect(other.locator("#archive-result")).toContainText("Nothing on this device was changed");
  expect(await edits(other)).toBe(mine);

  await importFile(other, name, text);
  await other.locator("#archive-keep").click();
  await expect(other.locator("#archive-result")).toContainText("Kept as they were here: 1");
  expect(await edits(other)).toBe(mine);

  await importFile(other, name, text);
  await other.locator("#archive-use").click();
  await expect(other.locator("#archive-result")).toContainText("Replaced: 1");
  expect(JSON.parse((await edits(other))!)).toEqual(JSON.parse((await edits(page))!));
  await context.close();
});

test("a damaged, foreign or future-version file shows why and leaves local work as it was", async ({ page }) => {
  await editStreet(page, 1);
  const { text } = await exported(page);
  const mine = await edits(page);
  const future = JSON.stringify({ ...JSON.parse(text), format_version: 2 });
  const newerEdits = JSON.stringify({ ...JSON.parse(text), edits: { ...JSON.parse(text).edits, version: 99 } });
  const cases: [string, string, RegExp][] = [
    ["garbage.json", "this is not json", /could not be read as JSON/],
    ["truncated.json", text.slice(0, text.length / 2), /could not be read as JSON/],
    ["other.json", '{"streets":{}}', /not a CityLoom design archive/],
    ["future.json", future, /format version 2, made by a newer CityLoom/],
    ["newer-edits.json", newerEdits, /edits in this archive are version 99/],
  ];
  for (const [name, body, message] of cases) {
    await importFile(page, name, body);
    const result = page.locator("#archive-result");
    await expect(result).toContainText(message);
    await expect(result).toContainText("Nothing on this device was changed.");
    await expect(page.locator("#archive-confirm")).toHaveCount(0);
    expect(await edits(page), name).toBe(mine);
  }
});

test("the street page's Share menu offers Export design and saves the archive", async ({ page }) => {
  await editStreet(page, 1);
  await openStreet(page, 0);
  await page.locator("#share-btn").click();
  const download = page.waitForEvent("download");
  await page.locator("#share-export").click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/\.cityloom-design\.json$/);
  expect(JSON.parse(await readFile((await file.path())!, "utf8"))).toMatchObject({
    kind: "cityloom-design",
    format_version: 1,
  });
  await expect(page.locator("#share-menu")).toBeHidden();
  await expect(page.locator("#share-btn")).toBeFocused();
});

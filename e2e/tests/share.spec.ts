// Sharing a design by a link (#221), against the real server: a signed-in person shares a project, and anyone with the
// link (another browser, signed out) opens a copy of it read-only. Links that cannot open say why, in the page's
// language, and keep nothing.

import type { Page } from "@playwright/test";

import { newAccount, requireServer } from "./account-helpers";
import { anotherBrowser, expect, forgive, live, openStreet, placesOnMap, test } from "./fixtures";

test.beforeEach(async ({ request }) => requireServer(request));

const RING = [
  [-73.6, 45.52],
  [-73.59, 45.52],
  [-73.59, 45.53],
  [-73.6, 45.52],
];
const RULES = { version: 1, restrictions: { "r-1": { name: "Shared zone", rule: { kind: "car_free" }, ring: RING } } };
const MADE_UP = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const REFUSED = "This design is open read-only: nothing can be changed.";

/** A street of the first project widened by one step, and an area rule kept beside it. Says the streets' edits. */
async function design(page: Page) {
  await openStreet(page);
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  await expect(page.locator("#tb-changes")).toHaveText("1");
  await page.goto("/map.html");
  await page.evaluate((rules) => localStorage.setItem("cityloom-restrictions@first", JSON.stringify(rules)), RULES);
  return page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits@first")!).streets);
}

/** Shares the first project from its row on the home page; says the link. */
async function share(page: Page) {
  await page.goto("/index.html");
  await page.locator("#project-share-first").click();
  await expect(page).toHaveURL(/\/share\.html\?project=first$/);
  await expect(page.locator("#sharing-heading")).toHaveText("Share First project");
  await expect(page.locator("#sharing-create")).toBeEnabled();
  await page.locator("#sharing-create").click();
  await expect(live(page)).toHaveText(/^Share link created\. It works until \w+ \d+, \d{4}\.$/);
  const field = page.locator("input.share-link").first();
  await expect(field).toBeFocused();
  const link = await field.inputValue();
  expect(link).toMatch(/\/share\.html#[A-Za-z0-9_-]{43}$/);
  return link;
}

/** The page of the share link `hash` in a browser of its own, signed out. */
async function opened(browser: Parameters<typeof anotherBrowser>[0], baseURL: string, hash: string, lang = "en") {
  const other = await anotherBrowser(browser, baseURL);
  await other.context.addInitScript((lang) => localStorage.setItem("cityloom-lang", lang), lang);
  await other.page.goto(`/share.html#${hash}`);
  return other;
}

test("a project shared by a signed-in person opens read-only, rules and all, for anyone with the link", async ({
  page,
  context,
  baseURL,
  browser,
}) => {
  await newAccount(page, context, baseURL!);
  const streets = await design(page);
  const link = await share(page);
  await expect(page.locator(".share-expiry").first()).toHaveText(/^Works until /);

  const other = await anotherBrowser(browser, baseURL!);
  await other.page.goto(link);
  await expect(other.page).toHaveURL(/\/map\.html\?project=s-[0-9a-f]{12}&view=1$/);
  const copy = new URL(other.page.url()).searchParams.get("project")!;
  await expect(other.page.locator("#read-only-tag")).toBeVisible();
  // the edited street and the area rule are there
  expect(
    await other.page.evaluate((id) => JSON.parse(localStorage.getItem(`cityloom-edits@${id}`)!).streets, copy),
  ).toEqual(streets);
  if ((await other.page.locator("#notes-toggle").getAttribute("aria-expanded")) === "false")
    await other.page.locator("#notes-toggle").click();
  await other.page.locator("#map-go-areas").click();
  await expect(other.page.locator("#areas")).toContainText("Shared zone");
  await expect(other.page.locator("#declare-area")).toHaveCount(0);
  // a street opens read-only, and its edits are refused
  const href = (await placesOnMap(other.page)).streets[0];
  await other.page.goto(`/${href}&project=${copy}&view=1`);
  await expect(other.page.locator("#read-only-tag")).toBeVisible();
  await expect(other.page.locator("#tb-changes")).toHaveText("1");
  await other.page.locator("#wrap").focus();
  await other.page.keyboard.press("ArrowRight");
  await other.page.keyboard.press("+");
  await expect(live(other.page)).toHaveText(REFUSED);
  // even without `view=1` in its address the copy is only looked at
  await other.page.goto(`/${href}&project=${copy}`);
  await expect(other.page.locator("#read-only-tag")).toBeVisible();
  // no new project is listed as the person's own
  await other.page.goto("/index.html");
  await expect(other.page.locator(".project-row")).toHaveCount(1);
  await expect(other.page.locator(".project-row")).toContainText("First project");
  await other.context.close();
});

test("a made-up link says no shared design has it, in English and in French, and keeps nothing", async ({
  page,
  errors,
}) => {
  for (const [lang, words] of [
    ["en", "No shared design has this link."],
    ["fr-CA", "Aucun aménagement partagé n’a ce lien."],
  ]) {
    await page.goto("/index.html");
    await page.evaluate((lang) => localStorage.setItem("cityloom-lang", lang), lang);
    const before = await page.evaluate(() => Object.keys(localStorage).sort());
    await page.goto(`/share.html#${MADE_UP}`);
    await expect(page.getByRole("alert")).toContainText(words);
    await expect(page).toHaveURL(new RegExp(`/share\\.html#${MADE_UP}$`));
    expect(await page.evaluate(() => Object.keys(localStorage).sort())).toEqual(before);
  }
  forgive(errors, /status of 404/);
});

test("a link that is not whole is refused without asking the server", async ({ page }) => {
  let asked = false;
  await page.route("**/api/share/**", (route) => {
    asked = true;
    return route.continue();
  });
  await page.goto("/share.html#short");
  await expect(page.getByRole("alert")).toContainText("This is not a whole share link.");
  expect(asked).toBe(false);
});

test("a share its creator deleted says so", async ({ page, context, baseURL, browser }) => {
  await newAccount(page, context, baseURL!);
  const link = await share(page);
  const token = link.split("#")[1];
  await page.locator(`#sharing-delete-${token}`).click();
  await expect(live(page)).toHaveText("Share link deleted. Anyone who opens it is told it was deleted.");
  await expect(page.locator(".share-row")).toHaveCount(0);
  await expect(page.locator("#sharing-create")).toBeFocused();

  const other = await opened(browser, baseURL!, token);
  await expect(other.page.getByRole("alert")).toContainText("The person who shared this design deleted it.");
  await other.context.close();
});

test("an expired share says it has expired, in English and in French", async ({ page, errors }) => {
  await page.route("**/api/share/**", (route) => route.fulfill({ status: 410, json: { error: "expired" } }));
  await page.goto(`/share.html#${MADE_UP}`);
  await expect(page.getByRole("alert")).toContainText(
    "This shared design has expired: a share link works for 90 days. Ask the person who shared it for a new link.",
  );
  await page.locator("#account-btn").click();
  await page.locator('[data-lang="fr-CA"]').click();
  await expect(page.getByRole("alert")).toContainText("Cet aménagement partagé a expiré");
  forgive(errors, /status of 410/);
});

test("signed out, the share page says to sign in and makes no link", async ({ page }) => {
  await page.goto("/share.html?project=first");
  await expect(page.locator("#sharing-blocked")).toHaveText(
    "Sign in to share this project: the server keeps shared copies for your account.",
  );
  await expect(page.locator("#sharing-create")).toBeDisabled();
});

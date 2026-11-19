import { expect, live, mapReady, openJunction, openStreet, test } from "./fixtures";
import type { Page } from "@playwright/test";

// A design opened read-only (#222): an address with `view=1` shows its project's map, streets and junctions to look
// at, says so in the top bar, offers no edit, refuses every edit by key, writes nothing the project keeps, and keeps
// the mode on the links between the project's pages.

const VIEW = "project=first&view=1";
const REFUSED = "This design is open read-only: nothing can be changed.";

/** Everything the browser keeps but the language a test switches: the project's documents, its list and the sync's. */
const kept = (page: Page) =>
  page.evaluate(() =>
    Object.keys(localStorage)
      .filter((k) => k !== "cityloom-lang")
      .sort()
      .map((k) => [k, localStorage.getItem(k)]),
  );

/** Widens the first piece of a street, then goes to the map, which keeps the edit: the project holds a design.
 *  Says the street's address. */
const editStreet = async (page: Page) => {
  const href = await openStreet(page);
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  await expect(page.locator("#tb-changes")).toHaveText("1");
  await page.goto("/map.html");
  await mapReady(page);
  return href;
};

/** The tag a read-only page has in its top bar says so, in short to the eye and in full to a screen reader. */
const saysReadOnly = async (page: Page, short: string, full: string) => {
  const tag = page.locator("#read-only-tag");
  await expect(tag).toBeVisible();
  await expect(tag.locator("span[aria-hidden]")).toHaveText(short);
  await expect(tag.locator(".sr-only")).toHaveText(full);
};

const toFrench = async (page: Page) => {
  await page.locator("#account-btn").click();
  await page.locator('[data-lang="fr-CA"]').click();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
};

test.describe("a design opened read-only", () => {
  test("a street can be looked at and chosen in, and not changed by a control, a key or undo", async ({ page }) => {
    const href = await editStreet(page);
    const before = await kept(page);
    await page.goto(`/${href}&${VIEW}`);
    await saysReadOnly(page, "Read-only", "This design is open read-only: you can look at it, but not change it.");
    // no undo, redo, Start over, Add or Transform
    for (const id of ["#undo", "#redo", "#reset", "#add-btn", "#transform-btn"])
      await expect(page.locator(id)).toHaveCount(0);
    await expect(page.locator("#tb-changes")).toHaveText("1");
    // a piece can be chosen, and its details are shown, disabled
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#piece-remove")).toBeVisible();
    await expect(page.locator("#piece-remove")).toBeDisabled();
    await expect(page.locator("#inspector input").first()).toBeDisabled();
    // the keys that edit refuse, and say why
    await page.locator("#wrap").focus();
    await page.keyboard.press("+");
    await expect(live(page)).toHaveText(REFUSED);
    await page.keyboard.press("Delete");
    await page.keyboard.press("Control+z");
    await expect(live(page)).toHaveText(REFUSED);
    await expect(page.locator("#tb-changes")).toHaveText("1");
    // the tag follows the language
    await toFrench(page);
    await saysReadOnly(
      page,
      "Lecture seule",
      "Cet aménagement est ouvert en lecture seule : vous pouvez le consulter, mais pas le modifier.",
    );
    // leaving keeps nothing either
    await page.goto(`/map.html?${VIEW}`);
    await mapReady(page);
    expect(await kept(page)).toEqual(before);
  });

  test("a junction can be looked at, its streets gone to, and not changed", async ({ page }) => {
    await editStreet(page);
    const href = await openJunction(page);
    await page.goto("/map.html");
    await mapReady(page);
    const before = await kept(page);
    await page.goto(`/${href}&${VIEW}`);
    await saysReadOnly(page, "Read-only", "This design is open read-only: you can look at it, but not change it.");
    for (const id of ["#undo", "#redo", "#reset"]) await expect(page.locator(id)).toHaveCount(0);
    // going to a street of the junction still works; what would change it is disabled
    const go = page.locator("button.jn-go[id^='jn-go-']").first();
    await go.click();
    await expect(page.locator("#jn-path-up-0")).toBeVisible();
    const fields = page.locator("#inspector .stepper input");
    if ((await fields.count()) > 0) await expect(fields.first()).toBeDisabled();
    // its link to the street's cross-section opens that read-only too
    const crossSection = page.locator("#inspector a.btn[href^='street.html']").first();
    await expect(crossSection).toHaveAttribute("href", /&project=first&view=1$/);
    // undo by key refuses, and says why
    await page.locator("#wrap").focus();
    await page.keyboard.press("Control+z");
    await expect(live(page)).toHaveText(REFUSED);
    await toFrench(page);
    await expect(page.locator("#read-only-tag span[aria-hidden]")).toHaveText("Lecture seule");
    await page.goto(`/map.html?${VIEW}`);
    await mapReady(page);
    expect(await kept(page)).toEqual(before);
  });

  test("the map can be searched and its places opened, read-only, and offers no area to declare", async ({ page }) => {
    await editStreet(page);
    const before = await kept(page);
    await page.goto(`/map.html?${VIEW}`);
    await mapReady(page);
    await saysReadOnly(page, "Read-only", "This design is open read-only: you can look at it, but not change it.");
    // the area rules can be read, and none declared
    if ((await page.locator("#notes-toggle").getAttribute("aria-expanded")) === "false") {
      await page.locator("#notes-toggle").click();
    }
    await page.locator("#map-go-areas").click();
    await expect(page.locator("#map-areas")).toBeVisible();
    await expect(page.locator("#declare-area")).toHaveCount(0);
    // the top bar's links to the pages outside the project are as they are
    await expect(page.locator("a.surface", { hasText: "Projects" })).toHaveAttribute("href", "index.html");
    // a place found opens read-only
    await page.locator("#search").fill("e");
    const found = page.locator("#search-results a.place-row").first();
    await expect(found).toHaveAttribute("href", /&project=first&view=1$/);
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\.html\?(street|junction)=\d+&project=first&view=1$/);
    await expect(page.locator("#read-only-tag")).toBeVisible();
    expect(await kept(page)).toEqual(before);
  });

  test("a project opened without view=1 can be edited as ever", async ({ page }) => {
    const href = await editStreet(page);
    await page.goto(`/${href}`);
    await expect(page.locator("#read-only-tag")).toHaveCount(0);
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator("#add-btn")).toBeVisible();
  });
});

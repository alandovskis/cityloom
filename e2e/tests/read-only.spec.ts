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
    // It edits a street, opens the map and a junction twice each, and switches the language: about 28 s alone, past
    // the default 30 s with other workers beside it.
    test.setTimeout(120_000);
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

  test("a junction's signal plan, its pedestrian calls, its actuation and its bicycle detection can be read, and none of them changed", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    // A signal with a crossing on call, made where the project can be edited, then kept by leaving for the map.
    const href = await openJunction(page);
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    const box = page.locator(".call-crossing input[type=checkbox]").first();
    const uid = (await box.getAttribute("id"))!.replace("call-on-", "");
    await box.click();
    await expect(page.locator(`#call-rate-${uid}`)).toBeVisible();
    await page.locator(`#call-detect-${uid}-passive`).check();
    await expect(page.locator(`#call-detect-${uid}-passive`)).toBeChecked();
    // A street made actuated (CLM-12).
    const street = (await page.locator("#signal-actuation .act-mode").first().getAttribute("id"))!.replace(
      "act-mode-",
      "",
    );
    await page.locator(`#act-mode-${street}-actuated`).check();
    await expect(page.locator(`#act-min-${street}`)).toHaveValue("7");
    // A bike movement on detection (CLM-13).
    const bikes = (await page.locator("#signal-bikes .bike-mode").first().getAttribute("id"))!.replace(
      "bike-mode-",
      "",
    );
    await page.locator(`#bike-mode-${bikes}-detection`).check();
    await expect(page.locator(`#bike-rate-${bikes}`)).toHaveValue("30");
    await page.goto("/map.html");
    await mapReady(page);
    const before = await kept(page);
    // Read-only: the plan and its calls are there to read; every box and rate field is disabled.
    await page.goto(`/${href}&${VIEW}`);
    await saysReadOnly(page, "Read-only", "This design is open read-only: you can look at it, but not change it.");
    await page.locator("#jn-go-plan").click();
    await expect(page.locator("#signal-calls")).toBeVisible();
    await expect(page.locator(`#call-on-${uid}`)).toBeChecked();
    const boxes = page.locator("#signal-calls .call-crossing input[type=checkbox]");
    expect(await boxes.count()).toBeGreaterThan(0);
    for (const b of await boxes.all()) await expect(b).toBeDisabled();
    await expect(page.locator(`#call-rate-${uid}`)).toBeDisabled();
    // The detection is shown, and cannot be changed.
    await expect(page.locator(`#call-detect-${uid}-passive`)).toBeChecked();
    await expect(page.locator(`#call-detect-help-${uid}`)).toContainText(
      "one who leaves after waiting 30 s cancels it",
    );
    for (const r of await page.locator(`#call-detect-${uid} input[type=radio]`).all()) await expect(r).toBeDisabled();
    // The actuated street is shown with its settings, and none can be changed.
    await expect(page.locator(`#act-mode-${street}-actuated`)).toBeChecked();
    for (const r of await page.locator(`#act-mode-${street} input[type=radio]`).all()) await expect(r).toBeDisabled();
    await expect(page.locator(`#act-min-${street}`)).toHaveValue("7");
    await expect(page.locator(`#act-min-${street}`)).toBeDisabled();
    await expect(page.locator(`#act-min-${street}-more`)).toBeDisabled();
    await expect(page.locator("#actuation-phases")).toBeVisible();
    // The bike movement on detection is shown with its cyclists, and cannot be changed.
    await expect(page.locator(`#bike-mode-${bikes}-detection`)).toBeChecked();
    for (const r of await page.locator(`#bike-mode-${bikes} input[type=radio]`).all()) await expect(r).toBeDisabled();
    await expect(page.locator(`#bike-rate-${bikes}`)).toHaveValue("30");
    await expect(page.locator(`#bike-rate-${bikes}`)).toBeDisabled();
    await expect(page.locator("#bikes-totals")).toBeVisible();
    await expect(page.locator("#calls-totals")).toBeVisible();
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
    // the row itself is followed: Enter pressed before the list takes it would open nothing
    await found.click();
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

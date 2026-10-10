import type { Page } from "@playwright/test";

import { expect, mapReady, openJunction, openStreet, test } from "./fixtures";

// The title following the language is covered in street.spec.ts ("keeps its own title, named for the street,
// when the language changes").

const selectFirstPiece = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
};

const chooseFrench = async (page: Page) => {
  await page.locator("#account-btn").click();
  await page.locator('[data-lang="fr-CA"]').click();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
};

/** The page in French from the first paint: the choice is stored before any script runs. */
const storedFrench = async (page: Page) => {
  await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
};

/** English phrases the French page must never show. They are the page's own English labels, listed
 *  rather than derived from the English page's words: a derived list is noisy, because French shares
 *  words with English ("Notes", "Menu", "Atlas", the street's own name). */
const ENGLISH = [
  "Add a piece",
  "Street today",
  "Undo",
  "Redo",
  "Allowed",
  "Piece details",
  "Settings",
  "Units",
  "Theme",
  "Region",
  "Checks",
  "Changes",
  "Measures",
  "Your design",
  "Today",
  "too wide",
  "left to use",
  "Resize",
];

/** The visible text of the page, less the street's own name (OpenStreetMap data). */
const visibleText = async (page: Page) => {
  const name = ((await page.locator("#street-name").textContent()) ?? "").trim();
  const text = await page.locator("body").innerText();
  return name ? text.split(name).join("") : text;
};

// Known gap: innerText does not see the values of aria-label and title attributes; the native tests of
// each component cover those.
const expectNoEnglish = async (page: Page, where: string) => {
  const text = await visibleText(page);
  for (const phrase of ENGLISH) expect(text, `"${phrase}" ${where}`).not.toContain(phrase);
  // Lengths take a decimal comma in French: "9,0 m", never "9.0 m".
  expect(text, `a decimal point in a length ${where}`).not.toMatch(/\d\.\d+[\s\u00a0\u202f]?(?:m|ft)\b/);
};

const overflow = (page: Page) =>
  page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

test("the street page can be read in French and the choice is kept", async ({ page }) => {
  await openStreet(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await chooseFrench(page);
  await expect(page.locator("#add-btn")).toHaveText(/Ajouter un élément/);
  await expect(page.locator('[data-lang="fr-CA"]')).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator('[data-lang="en"]')).toHaveAttribute("aria-pressed", "false");
  // The region list is rebuilt in French, not left as it was built.
  await expect(page.locator("#region option").first()).toHaveText(/^Canada \(/);
  await expect(page.locator("#region")).not.toContainText("(right)");
  await expect(page.locator("#region")).toContainText("États-Unis");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("#add-btn")).toHaveText(/Ajouter un élément/);
  await page.locator("#account-btn").click();
  await expect(page.locator('[data-lang="fr-CA"]')).toHaveAttribute("aria-pressed", "true");
});

test("a stored French choice is there from the first paint", async ({ page }) => {
  await storedFrench(page);
  await openStreet(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("#add-btn")).toHaveText(/Ajouter un élément/);
  await expectNoEnglish(page, "on a page opened in French");
});

test("nothing of the page is left in English once it is French, a piece selected and an edit made", async ({
  page,
}) => {
  await storedFrench(page);
  await openStreet(page);
  await selectFirstPiece(page);
  await page.keyboard.press("Enter");
  await expect(page.locator("#add-btn")).toHaveText(/Ajouter/);
  await expect(page.locator("#row-dim")).toHaveText(/^\d+,\d+/);
  await expect(page.locator("#tb-row")).toHaveText(/^\d+,\d+/);
  await expectNoEnglish(page, "with a piece selected");

  // An edit: the status line and the changes' labels are French too.
  await page.keyboard.press("Escape");
  await selectFirstPiece(page);
  await page.keyboard.press("+");
  await expect(page.locator("#fit")).not.toHaveText("");
  await expectNoEnglish(page, "after a nudge");

  for (const tab of ["#t-checks", "#t-changes", "#t-measures"]) {
    await page.locator(tab).click();
    await expect(page.locator(tab)).toHaveAttribute("aria-selected", "true");
    await expectNoEnglish(page, `on the ${tab} tab`);
  }

  // The add menu: its group and kind names and the default widths. It is closed again whatever happens.
  await page.locator("#add-btn").click();
  try {
    await expect(page.locator(".add-item").first()).toBeVisible();
    await expect(page.locator(".dw").first()).toHaveText(/^\d+,\d+/);
    await expectNoEnglish(page, "with the add menu open");
  } finally {
    await page.keyboard.press("Escape");
  }
  await expect(page.locator(".add-item").first()).toBeHidden();

  await page.locator("#account-btn").click();
  await expectNoEnglish(page, "with the settings menu open");
});

/** The page and the settings menu fit the viewport at every width. */
const expectFits = async (page: Page) => {
  for (const width of [1280, 1024, 768, 390]) {
    await page.setViewportSize({ width, height: 800 });
    expect(await overflow(page), `at ${width}px`).toBeLessThanOrEqual(0);
    await page.locator("#account-btn").click();
    const menu = page.locator("#account-menu");
    await expect(menu).toBeVisible();
    const box = (await menu.boundingBox())!;
    expect(box.x, `the menu's left edge at ${width}px`).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, `the menu's right edge at ${width}px`).toBeLessThanOrEqual(width);
    expect(await overflow(page), `with the menu open at ${width}px`).toBeLessThanOrEqual(0);
    await page.keyboard.press("Escape");
  }
};

// English is the baseline: French may not overflow where English does not. English fits at all four widths,
// so none is left out of the French check. Every page is checked, each in both languages.
const OPENERS: { name: string; open: (page: Page) => Promise<unknown>; ready?: (page: Page) => Promise<void> }[] = [
  { name: "home", open: (page) => page.goto("/") },
  { name: "map", open: (page) => page.goto("/map.html"), ready: mapReady },
  { name: "street", open: (page) => openStreet(page) },
  { name: "junction", open: (page) => openJunction(page) },
];

for (const { name, open, ready } of OPENERS) {
  test(`English text does not overflow the ${name} page`, async ({ page }) => {
    await open(page);
    await ready?.(page);
    await expectFits(page);
  });

  test(`French text does not overflow the ${name} page`, async ({ page }) => {
    await storedFrench(page);
    await open(page);
    await ready?.(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expectFits(page);
  });
}

test("a French reader types a decimal comma", async ({ page }) => {
  await storedFrench(page);
  await openStreet(page);
  await selectFirstPiece(page);
  await page.keyboard.press("Enter");
  const width = page.locator("#inspector input").first();
  await expect(width).toBeFocused();
  await expect(width).toHaveValue(/^\d+,\d{2}$/);
  await width.fill("2,5");
  await width.press("Enter");
  await expect(width).toHaveValue("2,50");
  const fit = page.locator("#fit");
  await expect(fit).toContainText(/de trop|inutilisé/);
  await expect(fit).not.toContainText("too wide");
});

test("the map page can be read in French and the choice is kept", async ({ page }) => {
  await page.goto("/map.html");
  await mapReady(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("#h-checks")).toHaveText("Does it work?");
  await chooseFrench(page);
  await expect(page.locator("#h-checks")).toHaveText("Est-ce que ça fonctionne?");
  await expect(page.locator("#places-panel h3").first()).toHaveText("Jonctions");
  await expect(page.locator("#zoom-fit")).toHaveAttribute("title", "Toute la ville");
  await expect(page).toHaveTitle("Carte de la ville CityLoom");
  // The search finds what the page shows, in its language.
  // A junction's small print counts its streets: "4 rues".
  await page.locator("#search").fill("rues");
  await expect(page.locator("#search-results [role=option]").first()).toContainText(/\d rues/);
  await page.reload();
  await mapReady(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("#h-checks")).toHaveText("Est-ce que ça fonctionne?");
  await expect(page.locator("#reset-label")).toHaveText("Recommencer");
});

test("the home page can be read in French and the choice is kept", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("h1")).toHaveText("Redesign the streets of your city.");
  await chooseFrench(page);
  await expect(page.locator("h1")).toHaveText("Réaménagez les rues de votre ville.");
  await expect(page.locator("#search")).toHaveAttribute("placeholder", "Chercher un lieu, comme Kreuzberg, Berlin");
  await expect(page.locator(".search-go")).toHaveText("Chercher");
  await expect(page.locator(".hero-note a")).toHaveText("les contributeurs d’OpenStreetMap");
  await expect(page.locator(".hero-facts")).toContainText("rues");
  await expect(page).toHaveTitle("CityLoom : réaménagez les rues de votre ville");
  await page.reload();
  await expect(page.locator("#hero-facts-slot .hero-facts")).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("h1")).toHaveText("Réaménagez les rues de votre ville.");
});

test("the junction page can be read in French and the choice is kept", async ({ page }) => {
  await openJunction(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("#h-turns")).toHaveText("Turns allowed");
  await expect(page.locator("#inspector")).toContainText("Junction control");
  await chooseFrench(page);
  // The markup, the views and the panel beside the plan follow the switch without a reload.
  await expect(page.locator("#h-turns")).toHaveText("Virages permis");
  await expect(page.locator("#inspector")).toContainText("Signalisation de la jonction");
  await expect(page.locator("#arm-count")).toHaveText(/^\d rues$/);
  await expect(page.locator("#undo")).toHaveText(/Annuler/);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("#h-turns")).toHaveText("Virages permis");
  await expect(page.locator("#inspector")).toContainText("Signalisation de la jonction");
});

test("a junction page opened in French is French from the first paint", async ({ page }) => {
  await storedFrench(page);
  await openJunction(page);
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(page.locator("#arm-count")).toHaveText(/^\d rues$/);
  await expect(page.locator("#inspector")).toContainText(
    "Sélectionnez une rue, un coin ou un passage pour piétons à modifier.",
  );
  await expect(page).toHaveTitle(/ · CityLoom$/);
  const text = await page.locator("body").innerText();
  for (const phrase of ["Junction control", "Select a street", "Turns allowed", "Getting across", "Undo", "Streets"]) {
    expect(text, phrase).not.toContain(phrase);
  }
});

/** Makes every junction of the city undrawable, the way a person could: an edit is made on the street page,
 *  then the stored city is read and every segment widened, each street's row left as it was (a saved street
 *  whose row changed is not taken back). */
const breakJunctions = async (page: Page) => {
  await openStreet(page);
  await selectFirstPiece(page);
  await page.keyboard.press("+");
  await expect(page.locator("#fit")).not.toHaveText("");
  // The edit is kept after a debounce; leaving the page flushes it.
  await page.goto("/map.html");
  await mapReady(page);
  await page.evaluate(() => {
    const key = Object.keys(localStorage).find((k) => k.startsWith("cityloom-city:"))!;
    const city = JSON.parse(localStorage.getItem(key)!);
    for (const street of Object.values<any>(city.streets))
      for (const segment of street.segments) segment.width_mm *= 40;
    localStorage.setItem(key, JSON.stringify(city));
  });
};

test("a junction that cannot be drawn says so, keeps its shell and follows the language", async ({ page }) => {
  await breakJunctions(page);
  const href = await openJunction(page);
  // No redirect to the map: the page of the junction stays, with its name and its panel.
  expect(page.url()).toContain(href);
  const stuck = page.locator(".stuck");
  await expect(stuck.locator(".note-h")).toHaveText("This junction cannot be drawn");
  await expect(page.locator(".tools")).toBeHidden();
  await expect(page.locator(".plan-drawing")).toHaveCount(0);
  const name = ((await page.locator("#street-name").textContent()) ?? "").trim();
  expect(name).not.toBe("");
  expect(await page.title()).toContain(name);
  await expect(stuck.locator("a.back")).toHaveAttribute("href", "map.html");

  // The shell still works: the settings menu opens and the language switch re-words the panel.
  await chooseFrench(page);
  await expect(stuck.locator(".note-h")).toHaveText("Cette jonction ne peut pas être dessinée");
  await expect(page.locator('[data-lang="fr-CA"]')).toHaveAttribute("aria-pressed", "true");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
  await expect(stuck.locator(".note-h")).toHaveText("Cette jonction ne peut pas être dessinée");
  await expect(page.locator(".tools")).toBeHidden();
});

test("a junction that cannot be drawn fits the page in French", async ({ page }) => {
  await breakJunctions(page);
  await storedFrench(page);
  await openJunction(page);
  await expect(page.locator(".stuck")).toBeVisible();
  await expectFits(page);
});

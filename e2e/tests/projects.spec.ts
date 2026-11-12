import type { Browser, Page } from "@playwright/test";

import { expect, live, openStreet, serveWorld, test } from "./fixtures";

// The home page is the list of the person's projects: made, opened, renamed and deleted there. The work kept from
// before projects becomes the first project. Every test's own context already has that first project (fixtures.ts);
// the tests that need a browser with no project, or with the old work only, make their own.

/** A browser that has never seen CityLoom's projects: its storage starts with `seed` only (once, not on every load),
 *  in `lang`. Its page serves the Plateau's tile and fails the test on an error, as the fixtures' own page does. */
async function freshBrowser(browser: Browser, baseURL: string, seed: Record<string, string> = {}, lang = "en") {
  const context = await browser.newContext({ baseURL, locale: "en-CA", viewport: { width: 1440, height: 900 } });
  await context.addInitScript(
    ({ seed, lang }) => {
      try {
        if (sessionStorage.getItem("seeded") === null) {
          sessionStorage.setItem("seeded", "1");
          localStorage.setItem("cityloom-lang", lang);
          for (const [key, value] of Object.entries(seed)) localStorage.setItem(key, value);
        }
      } catch {
        // storage is blocked: nothing to seed
      }
    },
    { seed, lang },
  );
  const page = await context.newPage();
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => {
    if (m.type() !== "error") return;
    if (/^\/api\/(me|providers|work)$/.test(new URL(m.location().url || "http://x/").pathname)) return;
    errors.push(m.text());
  });
  await serveWorld(page);
  return { context, page, errors };
}

const rows = (page: Page) => page.locator("li.project-row");

/** The list is drawn once the page's module has run. */
const listed = (page: Page) => expect(page.locator("#projects-heading")).toBeVisible();

test.describe("the list of projects", () => {
  test("starts empty, makes a project, keeps it over a reload and opens its map", async ({ browser, baseURL }) => {
    const { context, page, errors } = await freshBrowser(browser, baseURL!);
    await page.goto("/");
    await listed(page);
    await expect(page.locator("h1")).toHaveText("Your projects");
    await expect(page.locator(".project-empty")).toHaveText(
      "You have no projects yet. Name your first one below to start.",
    );
    await expect(rows(page)).toHaveCount(0);
    // The area is shown, not chosen: there is one.
    await expect(page.locator(".project-create .project-area")).toContainText("Plateau Mont-Royal, Montréal");

    // An empty name is refused, and the field says why.
    await page.getByRole("button", { name: "Create project" }).click();
    await expect(page.locator("#project-create-error")).toHaveText("Give the project a name.");
    await expect(page.locator("#project-name")).toHaveAttribute("aria-invalid", "true");
    await expect(page.locator("#project-name")).toBeFocused();

    await page.getByLabel("Project name").fill("Plateau study");
    await page.keyboard.press("Enter");
    await expect(rows(page)).toHaveCount(1);
    await expect(page.locator(".project-empty")).toHaveCount(0);
    await expect(page.locator("#project-name")).toHaveValue("");
    await expect(live(page)).toHaveText("Project Plateau study created.");
    const row = page.getByRole("listitem", { name: "Plateau study Plateau Mont-Royal, Montréal" });
    await expect(row).toBeVisible();
    await expect(row.locator(".project-changed")).toHaveText(/^Changed [A-Z][a-z]+ \d{1,2}, \d{4}$/);

    await page.reload();
    await listed(page);
    await expect(rows(page)).toHaveCount(1);
    await expect(page.getByRole("link", { name: "Plateau study" })).toBeVisible();

    await page.getByRole("link", { name: "Plateau study" }).click();
    await expect(page).toHaveURL(/\/map\.html\?project=p-[0-9a-f]{12}$/);
    const id = new URL(page.url()).searchParams.get("project");
    expect(await page.evaluate(() => localStorage.getItem("cityloom-project"))).toBe(id);
    expect(errors).toEqual([]);
    await context.close();
  });

  test("renames a project and deletes one only once asked, and says so", async ({ page }) => {
    await page.goto("/");
    await listed(page);
    await page.getByLabel("Project name").fill("Verdun");
    await page.getByRole("button", { name: "Create project" }).click();
    await expect(rows(page)).toHaveCount(2);

    await page.getByRole("button", { name: "Rename Verdun" }).click();
    const field = page.locator("#project-rename-field");
    await expect(field).toBeFocused();
    await expect(field).toHaveValue("Verdun");
    await field.fill("Verdun south");
    await field.press("Enter");
    await expect(page.getByRole("link", { name: "Verdun south" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Rename Verdun south" })).toBeFocused();

    await page.getByRole("button", { name: "Delete Verdun south" }).click();
    await expect(page.locator("#project-delete-question")).toHaveText(
      "Delete Verdun south and everything in it? This cannot be undone.",
    );
    await expect(page.locator("#project-delete-safe")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.locator("#project-delete-question")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Delete Verdun south" })).toBeFocused();
    await expect(rows(page)).toHaveCount(2);

    await page.getByRole("button", { name: "Delete Verdun south" }).click();
    await page.getByRole("button", { name: "Delete the project" }).click();
    await expect(rows(page)).toHaveCount(1);
    await expect(live(page)).toHaveText("Project Verdun south deleted.");
    await expect(page.locator("#projects-heading")).toBeFocused();
    await page.reload();
    await listed(page);
    await expect(rows(page)).toHaveCount(1);
    await expect(page.getByRole("link", { name: "First project" })).toBeVisible();
  });

  test("makes the work kept from before projects the first project", async ({ page, browser, baseURL }) => {
    // Real work: an edit made on a street of the first project, as an older CityLoom kept it, under the bare key.
    await openStreet(page);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(live(page)).toContainText("Resize");
    await expect.poll(() => page.evaluate(() => localStorage.getItem("cityloom-edits@first"))).not.toBeNull();
    const edits = (await page.evaluate(() => localStorage.getItem("cityloom-edits@first")))!;

    const { context, page: legacy, errors } = await freshBrowser(browser, baseURL!, { "cityloom-edits": edits });
    await legacy.goto("/");
    await listed(legacy);
    await expect(rows(legacy)).toHaveCount(1);
    await expect(legacy.getByRole("link", { name: "First project" })).toHaveAttribute("href", "map.html?project=first");
    const kept = await legacy.evaluate(() => ({
      old: localStorage.getItem("cityloom-edits"),
      moved: localStorage.getItem("cityloom-edits@first"),
      last: localStorage.getItem("cityloom-project"),
    }));
    expect(kept).toEqual({ old: "", moved: edits, last: "first" });
    // The pages open it as they opened the old work.
    await legacy.goto("/map.html");
    await expect(legacy).toHaveURL(/\/map\.html$/);
    await expect(legacy.locator("#changes")).not.toBeEmpty();
    expect(errors).toEqual([]);
    await context.close();
  });

  test("a page sent back from a project that is gone says so", async ({ page }) => {
    await page.goto("/map.html?project=zzz");
    await expect(page).toHaveURL(/\/index\.html\?missing=1$/);
    await expect(page.getByRole("alert")).toHaveText("That project is no longer here.");
    await expect(rows(page)).toHaveCount(1);
  });

  test("is in French for a French reader, the first project too", async ({ browser, baseURL }) => {
    const { context, page, errors } = await freshBrowser(
      browser,
      baseURL!,
      { "cityloom-edits": '{"version":5,"streets":{},"junctions":{}}' },
      "fr-CA",
    );
    await page.goto("/index.html?missing=1");
    await listed(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expect(page.locator("h1")).toHaveText("Vos projets");
    await expect(page.getByRole("alert")).toHaveText("Ce projet n’est plus ici.");
    await expect(page.getByRole("link", { name: "Premier projet" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Renommer Premier projet" })).toBeVisible();
    await page.getByLabel("Nom du projet").fill("Mile End");
    await page.getByRole("button", { name: "Créer le projet" }).click();
    await expect(rows(page)).toHaveCount(2);
    await expect(page.locator(".project-row", { hasText: "Mile End" }).locator(".project-changed")).toHaveText(
      /^Modifié le (1er|\d{1,2}) [a-zéû]+ \d{4}$/,
    );
    await expect(page.locator(".about-heading")).toHaveText("À propos de CityLoom");
    // The switch is followed without a reload.
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="en"]').click();
    await expect(page.locator("h1")).toHaveText("Your projects");
    await expect(page.getByRole("button", { name: "Rename Mile End" })).toBeVisible();
    expect(errors).toEqual([]);
    await context.close();
  });

  test("the first stop is a skip link to the list, and the list comes before the About section", async ({ page }) => {
    await page.goto("/");
    await listed(page);
    const skip = page.locator(".skip-link");
    await expect(skip).toHaveText("Skip to your projects");
    await page.keyboard.press("Tab");
    await expect(skip).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#projects-heading")).toBeFocused();
    // Then the list's own controls: the first project, then its buttons.
    await page.keyboard.press("Tab");
    await expect(page.getByRole("link", { name: "First project" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "Rename First project" })).toBeFocused();
    const list = (await page.locator(".projects-card").boundingBox())!;
    const about = (await page.locator(".hero-card").boundingBox())!;
    expect(about.y).toBeGreaterThanOrEqual(list.y + list.height);
    await expect(page.getByRole("heading", { level: 2, name: "About CityLoom" })).toBeVisible();
  });

  test("every control has a name, every field a label, and each id is the page's only one", async ({ page }) => {
    await page.goto("/");
    await listed(page);
    await page.getByRole("button", { name: "Rename First project" }).click();
    const problems = await page.evaluate(() => {
      const out: string[] = [];
      const ids = new Map<string, number>();
      for (const el of document.querySelectorAll("[id]")) ids.set(el.id, (ids.get(el.id) ?? 0) + 1);
      for (const [id, n] of ids) if (n > 1) out.push(`#${id} ×${n}`);
      for (const el of document.querySelectorAll("#projects-slot input")) {
        const input = el as HTMLInputElement;
        if (!input.labels || input.labels.length === 0) out.push(`input #${input.id} has no label`);
      }
      for (const el of document.querySelectorAll("#projects-slot button, #projects-slot a")) {
        const name = (el.getAttribute("aria-label") ?? el.textContent ?? "").trim();
        if (!name) out.push(`${el.tagName} with no name`);
      }
      for (const el of document.querySelectorAll("[aria-labelledby], [aria-describedby]")) {
        for (const ref of `${el.getAttribute("aria-labelledby") ?? ""} ${el.getAttribute("aria-describedby") ?? ""}`
          .split(/\s+/)
          .filter(Boolean))
          if (!document.getElementById(ref)) out.push(`${el.tagName} refers to #${ref}, which is not there`);
      }
      return out;
    });
    expect(problems).toEqual([]);
  });

  test("fits a phone without scrolling sideways", async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 640 });
    await page.goto("/");
    await listed(page);
    await page.getByRole("button", { name: "Delete First project" }).click();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
});

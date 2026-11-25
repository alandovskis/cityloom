// The Account page. The server's account side is not running in these tests, so the requests are answered here.

import type { Page } from "@playwright/test";

import { expect, live, test } from "./fixtures";

interface Identity {
  id: string;
  provider: string;
  label: string | null;
  created_at: string;
}

interface Account {
  store_on_server: boolean | null;
  identities: Identity[];
}

const PROVIDERS = [
  { slug: "osm", name: "OpenStreetMap" },
  { slug: "google", name: "Google" },
];

/** Answers /api/me and the changes to it from `account`; null is a signed-out visitor. */
async function serveAccount(page: Page, account: Account | null) {
  const out = (route: { fulfill: (o: { status: number }) => Promise<void> }) => route.fulfill({ status: 401 });
  await page.route("**/api/providers", (route) => route.fulfill({ json: { providers: PROVIDERS } }));
  await page.route("**/api/me", (route) => {
    const method = route.request().method();
    if (!account) return out(route);
    if (method === "DELETE") {
      account = null;
      return route.fulfill({ status: 204 });
    }
    return route.fulfill({ json: { id: "u", display_name: "Ada", ...account } });
  });
  await page.route("**/api/me/storage", async (route) => {
    if (!account) return out(route);
    account.store_on_server = (route.request().postDataJSON() as { store_on_server: boolean }).store_on_server;
    await route.fulfill({ status: 204 });
  });
  await page.route("**/api/me/identities/*", async (route) => {
    if (!account) return out(route);
    const id = route.request().url().split("/").pop();
    account.identities = account.identities.filter((i) => i.id !== id);
    await route.fulfill({ status: 204 });
  });
}

const identity = (id: string, provider: string, label: string | null): Identity => ({
  id,
  provider,
  label,
  created_at: "2026-10-01T00:00:00Z",
});

test("a signed-out visitor is offered the providers", async ({ page }) => {
  await serveAccount(page, null);
  await page.goto("/account.html");
  await expect(page.getByText("You are signed out.")).toBeVisible();
  await expect(page.locator("main [data-provider]")).toHaveText(["OpenStreetMap", "Google"]);
});

test("a new account is asked where its work is kept, with nothing chosen for it", async ({ page }) => {
  await serveAccount(page, { store_on_server: null, identities: [identity("a", "google", "ada@example.org")] });
  await page.goto("/account.html");
  await expect(page.locator("#h-storage")).toBeFocused();
  await expect(page.getByRole("radio")).toHaveCount(2);
  await expect(page.getByRole("radio", { checked: true })).toHaveCount(0);
  await page.getByRole("radio", { name: /Store my work on the server/ }).click();
  await expect(live(page)).toHaveText("Your choice is recorded: store your work on the server.");
  await expect(page.getByRole("radio", { name: /Store my work on the server/ })).toBeFocused();
  // The choice does not move from under the person.
  expect(
    await page.locator("#h-storage").evaluate((h) => h.compareDocumentPosition(document.getElementById("h-linked")!)),
  ).toBe(4 /* DOCUMENT_POSITION_FOLLOWING */);
});

test("unlinking a sign-in keeps focus on the page and the last one cannot be unlinked", async ({ page }) => {
  await serveAccount(page, {
    store_on_server: true,
    identities: [identity("a", "google", "ada@example.org"), identity("b", "osm", null)],
  });
  await page.goto("/account.html");
  await expect(page.getByRole("button", { name: "Unlink", exact: false })).toHaveCount(2);
  await page.getByRole("button", { name: "Unlink Google (ada@example.org)" }).click();
  await expect(live(page)).toHaveText("Sign-in unlinked.");
  await expect(page.locator("#h-linked")).toBeFocused();
  await expect(page.getByRole("button", { name: /^Unlink/ })).toHaveCount(0);
  await expect(page.getByText("This is your only sign-in, so it cannot be unlinked.")).toBeVisible();
});

test("deleting is confirmed in the page, with focus on the safe button, and leaves for the home page", async ({
  page,
}) => {
  await serveAccount(page, { store_on_server: false, identities: [identity("a", "google", null)] });
  await page.goto("/account.html");
  await page.getByRole("button", { name: "Delete account…" }).click();
  await expect(page.locator("#delete-no")).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#delete-btn")).toBeFocused();
  await page.getByRole("button", { name: "Delete account…" }).click();
  await page.getByRole("button", { name: "Delete my account" }).click();
  await page.waitForURL("**/index.html");
});

test("a session that ended meanwhile shows the signed-out page", async ({ page }) => {
  test.info().annotations.push({ type: "failing-account-api" });
  const account: Account = {
    store_on_server: true,
    identities: [identity("a", "google", null), identity("b", "osm", null)],
  };
  await serveAccount(page, account);
  await page.goto("/account.html");
  await expect(page.getByRole("button", { name: /^Unlink/ })).toHaveCount(2);
  await page.unroute("**/api/me/identities/*");
  await page.route("**/api/me/identities/*", (route) => route.fulfill({ status: 401 }));
  await page.getByRole("button", { name: "Unlink OpenStreetMap" }).click();
  await expect(page.getByText("You are signed out.")).toBeVisible();
  await expect(page.locator("#account-title")).toBeFocused();
});

test("a storage choice the server refuses leaves nothing chosen and focus on the choice", async ({ page }) => {
  test.info().annotations.push({ type: "failing-account-api" });
  await serveAccount(page, { store_on_server: null, identities: [identity("a", "google", null)] });
  await page.unroute("**/api/me/storage");
  await page.route("**/api/me/storage", (route) => route.fulfill({ status: 500 }));
  await page.goto("/account.html");
  await page.getByRole("radio", { name: /Store my work on the server/ }).click();
  await expect(live(page)).toHaveText("That did not work. Try again.");
  await expect(page.getByRole("radio", { checked: true })).toHaveCount(0);
  await expect(page.locator("#h-storage")).toBeFocused();
});

// ---- the storage choices as one radio group with the editors' (CLM-48) --------------------------------------------------

/** The storage choices the Tab key stops on. */
const tabStops = (page: Page) => page.locator('[role=radio][tabindex="0"]');
const deviceChoice = (page: Page) => page.getByRole("radio", { name: /Keep my work on this device only/ });
const serverChoice = (page: Page) => page.getByRole("radio", { name: /Store my work on the server/ });

test("with nothing chosen only the first storage choice is in the tab order, and an arrow chooses", async ({
  page,
}) => {
  await serveAccount(page, { store_on_server: null, identities: [identity("a", "google", null)] });
  const sent: string[] = [];
  page.on("request", (r) => {
    if (r.url().endsWith("/api/me/storage")) sent.push(r.method());
  });
  await page.goto("/account.html");
  await expect(page.locator("#h-storage")).toBeFocused();
  await expect(tabStops(page)).toHaveCount(1);
  await expect(tabStops(page)).toHaveAccessibleName(/Keep my work on this device only/);
  await page.keyboard.press("Tab");
  await expect(deviceChoice(page)).toBeFocused();
  await expect(page.getByRole("radio", { checked: true })).toHaveCount(0);
  await page.keyboard.press("ArrowDown");
  await expect(serverChoice(page)).toBeFocused();
  await expect(serverChoice(page)).toBeChecked();
  expect(sent).toHaveLength(1);
  // Choosing what is chosen already asks the server nothing.
  await page.keyboard.press("Space");
  await page.waitForTimeout(250);
  await expect(serverChoice(page)).toBeChecked();
  expect(sent).toHaveLength(1);
});

test("Space chooses the storage choice that has the focus", async ({ page }) => {
  await serveAccount(page, { store_on_server: null, identities: [identity("a", "google", null)] });
  await page.goto("/account.html");
  await expect(deviceChoice(page)).toHaveAttribute("tabindex", "0");
  await deviceChoice(page).focus();
  await page.keyboard.press("Space");
  await expect(deviceChoice(page)).toBeChecked();
});

test("an arrow from the server to the device asks first, and Escape leaves the server chosen", async ({ page }) => {
  await serveAccount(page, { store_on_server: true, identities: [identity("a", "google", null)] });
  await page.goto("/account.html");
  await expect(tabStops(page)).toHaveCount(1);
  await expect(tabStops(page)).toHaveAccessibleName(/Store my work on the server/);
  await serverChoice(page).focus();
  await page.keyboard.press("ArrowUp");
  await expect(page.locator("#stop-no")).toBeFocused();
  await expect(serverChoice(page)).toBeChecked();
  await page.keyboard.press("Escape");
  await expect(page.locator("#stop-confirm")).toHaveCount(0);
  await expect(page.locator("#h-storage")).toBeFocused();
  await expect(serverChoice(page)).toBeChecked();
});

test("the storage choices are cards with their help line, at 280 px, in both languages", async ({ page }) => {
  await serveAccount(page, { store_on_server: true, identities: [identity("a", "google", null)] });
  await page.setViewportSize({ width: 280, height: 700 });
  await page.goto("/account.html");
  for (const lang of ["en", "fr-CA"]) {
    await page.evaluate((l) => localStorage.setItem("cityloom-lang", l), lang);
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("lang", lang);
    const cards = page.locator('button[role="radio"]');
    await expect(cards).toHaveCount(2);
    for (const card of await cards.all()) await expect(card.locator(".choice-help")).toBeVisible();
    const bad = await cards.evaluateAll((all) =>
      all.flatMap((card) => {
        const css = getComputedStyle(card);
        const sides = [css.borderTopWidth, css.borderRightWidth, css.borderBottomWidth, css.borderLeftWidth];
        const help = card.querySelector(".choice-help")!;
        const [c, h] = [card.getBoundingClientRect(), help.getBoundingClientRect()];
        const within = h.left >= c.left && h.right <= c.right && h.top >= c.top && h.bottom <= c.bottom;
        const cut = card.scrollWidth > card.clientWidth || help.scrollWidth > help.clientWidth;
        return [
          ...(sides.some((s) => parseFloat(s) < 1) ? [`border ${sides}`] : []),
          ...(within && !cut ? [] : [`the help is cut: ${help.textContent}`]),
        ];
      }),
    );
    expect(bad, lang).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), lang).toBe(true);
  }
});

test("Escape closes the deletion's confirmation and focus goes back to Delete account", async ({ page }) => {
  await serveAccount(page, { store_on_server: true, identities: [identity("a", "google", null)] });
  await page.goto("/account.html");
  await page.getByRole("button", { name: "Delete account…" }).click();
  await expect(page.getByRole("group", { name: "Delete your account?" })).toHaveAccessibleDescription(/Railway/);
  await page.keyboard.press("Escape");
  await expect(page.locator("#delete-confirm")).toHaveCount(0);
  await expect(page.locator("#delete-btn")).toBeFocused();
});

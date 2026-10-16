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
  await page.getByRole("radio", { name: /Store my work on the server/ }).check();
  await expect(live(page)).toHaveText("Your work will be stored on the server.");
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
  await expect(live(page)).toHaveText("Account unlinked.");
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

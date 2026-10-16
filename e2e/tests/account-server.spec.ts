// The real server, not the stubs of account.spec.ts: sign in, the storage choice, linking, unlinking, signing out and deleting, against the server built with the
// fake OpenID provider (`--features test-provider`) and a Postgres.

import type { BrowserContext, Page } from "@playwright/test";

import { anotherBrowser, expect, live, test } from "./fixtures";

type Login = { sub?: string; name?: string; email?: string; deny?: boolean; first_time_user?: unknown };

test.beforeEach(async ({ request }) => {
  if (!process.env.DATABASE_URL) {
    throw new Error(
      "DATABASE_URL is not set: the account specs need a Postgres. Run 'just db', then export the URL it prints.",
    );
  }
  const providers = await request.get("/api/providers");
  expect(
    providers.status(),
    "the server's account side is off: is DATABASE_URL reachable by the server, and was it built with --features test-provider?",
  ).toBe(200);
});

const unique = () => `e2e-${crypto.randomUUID()}`;

/** Who the fake provider signs in next, in this browser. */
async function loginAs(context: BrowserContext, baseURL: string, login: Login) {
  await context.addCookies([{ name: "fake-login", value: encodeURIComponent(JSON.stringify(login)), url: baseURL }]);
}

async function chooseProvider(page: Page, slug: string) {
  await page.locator("#session-btn").click();
  await page.locator(`#session-menu [data-provider="${slug}"]`).click();
}

/** A new account on the fake provider, signed in on the Account page. */
async function newAccount(page: Page, context: BrowserContext, baseURL: string, login: Login = {}) {
  await loginAs(context, baseURL, { sub: unique(), name: "Ada", email: "ada@example.org", ...login });
  await page.goto("/map.html");
  await chooseProvider(page, "fake");
  await expect(page).toHaveURL(/\/account\.html$/);
  await expect(page.locator("#session-btn-label")).toHaveText(login.name ?? "Ada");
}

test.describe("signed out", () => {
  test("every page offers Sign in and the server says nobody is signed in", async ({ page, request }) => {
    expect((await request.get("/api/me")).status()).toBe(401);
    for (const path of ["/index.html", "/map.html", "/account.html"]) {
      await page.goto(path);
      await expect(page.locator("#session-btn")).toHaveText("Sign in");
    }
  });

  test("the menu lists the providers the server has", async ({ page }) => {
    await page.goto("/map.html");
    await page.locator("#session-btn").click();
    await expect(page.locator("#session-menu [data-provider]")).toHaveText(["Fake", "Fake (form post)"]);
    await page.keyboard.press("Escape");
    await expect(page.locator("#session-menu")).toBeHidden();
  });

  test("saying no at the provider comes back with Sign-in cancelled", async ({ page, context, baseURL }) => {
    await loginAs(context, baseURL!, { sub: unique(), deny: true });
    await page.goto("/map.html");
    await chooseProvider(page, "fake");
    await expect(page).toHaveURL(/\/map\.html$/);
    await expect(live(page)).toHaveText("Sign-in cancelled.");
    await expect(page.locator("#session-btn")).toHaveText("Sign in");
  });
});

test.describe("a first sign-in", () => {
  test("goes to the Account page and asks where the work is kept, with nothing chosen for the person", async ({
    page,
    context,
    baseURL,
  }) => {
    await newAccount(page, context, baseURL!);
    await expect(live(page)).toHaveText("Signed in. Choose where your work is kept.");
    await expect(page.getByRole("radio")).toHaveCount(2);
    await expect(page.getByRole("radio", { checked: true })).toHaveCount(0);
    await expect(page.locator("#h-storage")).toBeFocused();
  });

  test("keeps the choice, and the person stays signed in after a reload", async ({ page, context, baseURL }) => {
    await newAccount(page, context, baseURL!);
    await page.getByRole("radio", { name: /Store my work on the server/ }).check();
    await expect(live(page)).toHaveText(
      "Your choice is recorded: store your work on the server. Storing work on the server arrives in a later release; for now, your work stays on this device.",
    );
    await page.reload();
    await expect(page.locator("#session-btn-label")).toHaveText("Ada");
    await expect(page.getByRole("radio", { name: /Store my work on the server/ })).toBeChecked();
    await page.getByRole("radio", { name: /Keep my work on this device only/ }).check();
    await expect(live(page)).toHaveText(
      "Your choice is recorded: keep your work on this device only. For now, your work stays on this device either way.",
    );
    await page.reload();
    await expect(page.getByRole("radio", { name: /Keep my work on this device only/ })).toBeChecked();
  });

  test("a sign-in from Apple's form post takes the name sent that once", async ({ page, context, baseURL }) => {
    await loginAs(context, baseURL!, {
      sub: unique(),
      email: "relay@privaterelay.example",
      first_time_user: { name: { firstName: "Grace", lastName: "Hopper" } },
    });
    await page.goto("/map.html");
    await chooseProvider(page, "fake-apple");
    await expect(page.locator("#session-btn-label")).toHaveText("Grace Hopper");
    await expect(page.locator(".linked")).toContainText("relay@privaterelay.example");
  });
});

test.describe("a returning person", () => {
  test("signs out, signs in again to the same account and lands where they were", async ({
    page,
    context,
    baseURL,
  }) => {
    const sub = unique();
    await newAccount(page, context, baseURL!, { sub });
    await page.getByRole("radio", { name: /Store my work on the server/ }).check();
    await expect(live(page)).toHaveText(
      "Your choice is recorded: store your work on the server. Storing work on the server arrives in a later release; for now, your work stays on this device.",
    );
    await page.locator("#session-btn").click();
    await page.getByRole("button", { name: "Sign out" }).first().click();
    await expect(page).toHaveURL(/\/index\.html$/);
    await expect(page.locator("#session-btn")).toHaveText("Sign in");
    await page.reload();
    await expect(page.locator("#session-btn")).toHaveText("Sign in");
    await page.goto("/map.html");
    await chooseProvider(page, "fake");
    await expect(page).toHaveURL(/\/map\.html$/);
    await expect(live(page)).toHaveText("Signed in.");
    await expect(page.locator("#session-btn-label")).toHaveText("Ada");
    await page.goto("/account.html");
    await expect(page.getByRole("radio", { name: /Store my work on the server/ })).toBeChecked();
  });
});

test.describe("linked sign-ins", () => {
  test("a second provider is linked, either signs in afterwards, and one can be unlinked but not the last", async ({
    page,
    context,
    baseURL,
    browser,
  }) => {
    const first = unique();
    const second = unique();
    await newAccount(page, context, baseURL!, { sub: first });
    await expect(page.getByRole("button", { name: "Unlink" })).toHaveCount(0);
    await expect(page.getByText("This is your only sign-in, so it cannot be unlinked.")).toBeVisible();
    await loginAs(context, baseURL!, { sub: second, name: "Ada", email: "second@example.org" });
    await page.getByRole("button", { name: "Link Fake (form post)" }).click();
    await expect(page).toHaveURL(/\/account\.html$/);
    await expect(live(page)).toHaveText("Account linked.");
    await expect(page.locator(".linked li")).toHaveCount(2);
    // Signing in with the second identity, from a fresh browser context, is the same account.
    const { context: other, page: tab } = await anotherBrowser(browser, baseURL!);
    try {
      await loginAs(other, baseURL!, { sub: second, name: "Ada", email: "second@example.org" });
      await tab.goto("/map.html");
      await chooseProvider(tab, "fake-apple");
      await expect(tab.locator("#session-btn-label")).toHaveText("Ada");
      await tab.goto("/account.html");
      await expect(tab.locator(".linked li")).toHaveCount(2);
    } finally {
      await other.close();
    }
    // Unlink the first: one is left, and the last cannot be unlinked.
    await page
      .getByRole("button", { name: /^Unlink Fake \(/ })
      .first()
      .click();
    await expect(live(page)).toHaveText("Sign-in unlinked.");
    await expect(page.locator(".linked li")).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Unlink" })).toHaveCount(0);
  });

  test("an identity that belongs to another account is refused and nothing changes", async ({
    page,
    context,
    baseURL,
    browser,
  }) => {
    const theirs = unique();
    const { context: other, page: tab } = await anotherBrowser(browser, baseURL!);
    try {
      await newAccount(tab, other, baseURL!, { sub: theirs, name: "Bob" });
      await newAccount(page, context, baseURL!);
      await loginAs(context, baseURL!, { sub: theirs });
      // Bob signed in with the provider "fake", so a conflict needs the same provider: every enabled provider is
      // offered for linking, the one already linked included. (`exact`: "Link Fake" is also a prefix of the other.)
      await page.getByRole("button", { name: "Link Fake", exact: true }).click();
      await expect(page).toHaveURL(/\/account\.html$/);
      await expect(live(page)).toContainText("is already linked to another CityLoom account.");
      await expect(page.locator(".linked li")).toHaveCount(1);
    } finally {
      await other.close();
    }
  });
});

test.describe("the header", () => {
  test("shows the person's name in French too", async ({ page, context, baseURL }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr"));
    await loginAs(context, baseURL!, { sub: unique(), name: "Zoë", email: "zoe@example.org" });
    await page.goto("/map.html");
    await chooseProvider(page, "fake");
    await expect(page).toHaveURL(/\/account\.html$/);
    await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
    await expect(page.locator("#session-btn-label")).toHaveText("Zoë");
  });

  test("signing out from the menu puts focus back on the menu button", async ({ page, context, baseURL }) => {
    await loginAs(context, baseURL!, { sub: unique(), name: "Ada", email: "ada@example.org" });
    await page.goto("/map.html");
    await chooseProvider(page, "fake");
    await expect(page).toHaveURL(/\/account\.html$/);
    await expect(page.locator("#session-btn-label")).toHaveText("Ada");
    await page.goto("/map.html");
    await expect(page.locator("#session-btn-label")).toHaveText("Ada");
    await page.locator("#session-btn").click();
    await page.getByRole("button", { name: "Sign out" }).first().click();
    await expect(page.locator("#session-btn")).toHaveText("Sign in");
    await expect(page.locator("#session-btn")).toBeFocused();
  });
});

test.describe("deleting the account", () => {
  test("asks in the page, never in a dialog, then leaves no account behind", async ({ page, context, baseURL }) => {
    const dialogs: string[] = [];
    page.on("dialog", (d) => {
      dialogs.push(d.message());
      void d.dismiss();
    });
    const sub = unique();
    await newAccount(page, context, baseURL!, { sub });
    await page.getByRole("button", { name: "Delete account…" }).click();
    await expect(page.getByText(/Railway/)).toBeVisible();
    await expect(page.locator("#delete-no")).toBeFocused();
    await page.locator("#delete-no").click();
    await expect(page.locator("#delete-btn")).toBeFocused();
    await expect(page.locator("#session-btn-label")).toHaveText("Ada");
    await page.getByRole("button", { name: "Delete account…" }).click();
    await page.getByRole("button", { name: "Delete my account" }).click();
    await expect(page).toHaveURL(/\/index\.html$/);
    await expect(page.locator("#session-btn")).toHaveText("Sign in");
    // The same identity now makes a brand new account, which asks where to keep the work again.
    await page.goto("/map.html");
    await chooseProvider(page, "fake");
    await expect(page).toHaveURL(/\/account\.html$/);
    await expect(page.getByRole("radio", { checked: true })).toHaveCount(0);
    expect(dialogs).toEqual([]);
  });
});

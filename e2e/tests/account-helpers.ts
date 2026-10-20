import type { APIRequestContext, BrowserContext, Page } from "@playwright/test";

import { expect } from "./fixtures";

export type Login = { sub?: string; name?: string; email?: string; deny?: boolean; first_time_user?: unknown };

/** The specs need the real server with a Postgres and the fake provider. */
export async function requireServer(request: APIRequestContext) {
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
}

export const unique = () => `e2e-${crypto.randomUUID()}`;

/** Who the fake provider signs in next, in this browser. */
export async function loginAs(context: BrowserContext, baseURL: string, login: Login) {
  await context.addCookies([{ name: "fake-login", value: encodeURIComponent(JSON.stringify(login)), url: baseURL }]);
}

export async function chooseProvider(page: Page, slug: string) {
  await page.locator("#session-btn").click();
  await page.locator(`#session-menu [data-provider="${slug}"]`).click();
}

/** A new account on the fake provider, signed in on the Account page. */
export async function newAccount(page: Page, context: BrowserContext, baseURL: string, login: Login = {}) {
  await loginAs(context, baseURL, { sub: unique(), name: "Ada", email: "ada@example.org", ...login });
  await page.goto("/map.html");
  await chooseProvider(page, "fake");
  await expect(page).toHaveURL(/\/account\.html$/);
  await expect(page.locator("#session-btn-label")).toHaveText(login.name ?? "Ada");
}

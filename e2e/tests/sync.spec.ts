// Cloud edits against the real server: two browser contexts are two devices of one account; a context's `setOffline` is
// a device without a network. The fake provider signs a context in as whoever its `fake-login` cookie says, so the same
// `sub` in two contexts is one account.

import type { BrowserContext, Page } from "@playwright/test";

import { chooseProvider, loginAs, newAccount, requireServer, unique } from "./account-helpers";
import { anotherBrowser, expect, live, openStreet, test } from "./fixtures";

test.beforeEach(async ({ request }) => requireServer(request));

const SERVER = /Store my work on the server/;

/** What the server holds for the person signed in on this context. */
const serverWork = async (context: BrowserContext) => {
  // `context.request` does not send the Secure `__Host-` session cookie over http, so the header is set by hand.
  const cookie = (await context.cookies()).map((c) => `${c.name}=${c.value}`).join("; ");
  const response = await context.request.get("/api/work?since=0", { headers: { cookie } });
  return { status: response.status(), body: response.status() === 200 ? await response.json() : null };
};

/** The streets this browser holds edits for. */
const editedStreets = (page: Page) =>
  page.evaluate(() => Object.keys(JSON.parse(localStorage.getItem("cityloom-edits") ?? '{"streets":{}}').streets));

/** What this browser holds as the streets' edits, whole. */
const streetEdits = (page: Page) =>
  page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits") ?? '{"streets":{}}').streets);

/** Changes the first piece of the street on the page by one step: wider (`+`) or narrower (`-`). */
async function resize(page: Page, key: "+" | "-") {
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press(key);
  await expect(page.locator("#tb-changes")).not.toHaveText("0");
}

const widen = (page: Page) => resize(page, "+");
const narrow = (page: Page) => resize(page, "-");

/** A person with an account that stores its work on the server, signed in on this device (the Account page). */
async function storingAccount(page: Page, context: BrowserContext, baseURL: string) {
  const sub = unique();
  await newAccount(page, context, baseURL, { sub });
  await page.getByRole("radio", { name: SERVER }).check();
  await expect(live(page)).toHaveText("Your work was merged with your account: 0 places sent, 0 places received.");
  return sub;
}

/** The same person on a second device, signed in and on the map. */
async function secondDevice(browser: Parameters<typeof anotherBrowser>[0], baseURL: string, sub: string) {
  const device = await anotherBrowser(browser, baseURL);
  await loginAs(device.context, baseURL, { sub, name: "Ada", email: "ada@example.org" });
  await device.page.goto("/map.html");
  await chooseProvider(device.page, "fake");
  await expect(device.page.locator("#session-btn-label")).toHaveText("Ada");
  return device;
}

/** The tab shows again. */
const shown = (page: Page) => page.evaluate(() => document.dispatchEvent(new Event("visibilitychange")));

test("an edit on one device appears on another when its tab is shown again", async ({
  page,
  context,
  browser,
  baseURL,
}) => {
  test.slow(); // two browser contexts on a shared server: 18-25 s under parallel load, against a 30 s timeout
  const sub = await storingAccount(page, context, baseURL!);
  const phone = await secondDevice(browser, baseURL!, sub);
  const href = await openStreet(page, 0);
  await phone.page.goto("/" + href);
  await widen(page);
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  expect(await editedStreets(phone.page)).toEqual([]);
  await shown(phone.page);
  await expect(live(phone.page)).toHaveText("Updated from another device.");
  expect(await editedStreets(phone.page)).toEqual(await editedStreets(page));
  await phone.context.close();
});

test("an edit made offline is sent when the network returns, and the menu says so", async ({
  page,
  context,
  baseURL,
}) => {
  await storingAccount(page, context, baseURL!);
  await openStreet(page, 0);
  await expect(page.locator("#sync-status")).toHaveText("Synced"); // the page knows the person before the network goes
  await context.setOffline(true);
  await widen(page);
  await expect(page.locator("#sync-status")).toHaveText("Not synced yet");
  // the marked record is the street that was edited, and it waits to be sent
  await expect.poll(async () => (await editedStreets(page)).length).toBe(1);
  const [edited] = await editedStreets(page);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-sync") ?? "{}").records)).toMatchObject({
    [`street/${edited}`]: { pending: true, deleted: false },
  });
  await context.setOffline(false);
  await page.evaluate(() => window.dispatchEvent(new Event("online")));
  await expect(page.locator("#sync-status")).toHaveText("Synced");
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-sync") ?? "{}").records)).toMatchObject({
    [`street/${edited}`]: { pending: false },
  });
});

test("the last edit is sent when the page is left, with nothing else to send it", async ({
  page,
  context,
  baseURL,
}) => {
  await storingAccount(page, context, baseURL!);
  // Which writes to /api/work asked for `keepalive`, noted in storage because the page is gone when it matters.
  await page.addInitScript(() => {
    const real = window.fetch;
    window.fetch = function (input: any, init?: any) {
      // the app passes a Request, whose own `keepalive` is what the browser honours
      const method = String(input?.method ?? init?.method ?? "GET");
      if (String(input?.url ?? input).includes("/api/work") && method !== "GET") {
        const noted = JSON.parse(localStorage.getItem("e2e-work-writes") ?? "[]");
        noted.push(!!(input?.keepalive ?? init?.keepalive));
        localStorage.setItem("e2e-work-writes", JSON.stringify(noted));
      }
      return real.call(this, input, init);
    } as typeof fetch;
  });
  await openStreet(page, 0);
  await widen(page);
  // Nothing else can send it: no other page of the app opens to find it marked. Whether the 250 ms debounce had already
  // sent it first is told by the recorded fetches: the test needs a keepalive one.
  await page.goto("about:blank");
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  const reader = await context.newPage();
  await reader.goto("/map.html");
  expect(
    await reader.evaluate(() => JSON.parse(localStorage.getItem("e2e-work-writes") ?? "[]")),
    "the writes to /api/work, whether each asked for keepalive",
  ).toContain(true);
});

test("when two devices changed the same place the later edit is kept on both and the other device says so", async ({
  page,
  context,
  browser,
  baseURL,
}) => {
  test.slow(); // two browser contexts on a shared server: 18-25 s under parallel load, against a 30 s timeout
  const sub = await storingAccount(page, context, baseURL!);
  const phone = await secondDevice(browser, baseURL!, sub);
  const href = await openStreet(page, 0);
  await phone.page.goto("/" + href);
  await expect(phone.page.locator("#sync-status")).toHaveText("Synced"); // the page knows the person before the network goes
  await phone.context.setOffline(true);
  await widen(phone.page); // the phone's change comes first ...
  await narrow(page); // ... the laptop's later, and a different one: two equal bodies are no conflict
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  await phone.context.setOffline(false);
  await shown(phone.page);
  await expect(live(phone.page)).toHaveText("Your change to this place was replaced by a later one.");
  expect(await streetEdits(phone.page)).toEqual(await streetEdits(page)); // the later edit is kept on both
  await phone.context.close();
});

test("turning storage off asks first, deletes the server copy, keeps this browser's work and stops the other device", async ({
  page,
  context,
  browser,
  baseURL,
}) => {
  test.slow(); // two browser contexts on a shared server: 18-25 s under parallel load, against a 30 s timeout
  const sub = await storingAccount(page, context, baseURL!);
  const phone = await secondDevice(browser, baseURL!, sub);
  await openStreet(page, 0);
  await widen(page);
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  await shown(phone.page); // the phone gets the street while the copy is still on the server
  await expect.poll(async () => (await editedStreets(phone.page)).length).toBe(1);
  await page.goto("/account.html");
  await expect(page.locator("#h-privacy")).toBeVisible();
  await page.getByRole("radio", { name: /Keep my work on this device only/ }).click();
  await expect(page.locator("#stop-confirm")).toContainText(
    "This deletes your work from the server and keeps the copy in this browser.",
  );
  await page.locator("#stop-no").click();
  await expect(page.getByRole("radio", { name: SERVER })).toBeChecked();
  expect((await serverWork(context)).body.records.length).toBe(1);
  await page.getByRole("radio", { name: /Keep my work on this device only/ }).click();
  await page.locator("#stop-yes").click();
  await expect(live(page)).toHaveText(
    "Your choice is recorded: keep your work on this device only. The copy on the server was deleted.",
  );
  expect((await serverWork(context)).status).toBe(409);
  expect(await editedStreets(page)).toHaveLength(1);
  // the other device learns it the next time it syncs, stops syncing and keeps its own copy
  const phoneEdits = await streetEdits(phone.page);
  // the phone's own page asks the server who it is, and is told that storage is off: the sign it has been told
  const told = phone.page.waitForResponse(
    async (r) => r.url().endsWith("/api/me") && (await r.json()).store_on_server === false,
  );
  await phone.page.goto("/map.html");
  await phone.page.evaluate(() => window.dispatchEvent(new Event("online")));
  await told;
  // the positive sign that the phone has learned it: the status goes away and the server says storage is off
  await expect(phone.page.locator("#sync-status")).toHaveCount(0);
  expect(await phone.page.evaluate(async () => (await (await fetch("/api/me")).json()).store_on_server)).toBe(false);
  // and then its work is as it was, and the server has nothing of it
  await expect.poll(async () => streetEdits(phone.page)).toEqual(phoneEdits);
  expect(phoneEdits).toBeTruthy();
  expect(Object.keys(phoneEdits)).toHaveLength(1);
  expect((await serverWork(context)).status).toBe(409);
  await phone.context.close();
});

test("turning storage on merges the work made while signed out and says how many places were sent", async ({
  page,
  context,
  baseURL,
}) => {
  await openStreet(page, 0);
  await widen(page); // signed out: marked, not sent
  await page.goto("/map.html");
  await newAccount(page, context, baseURL!);
  await page.getByRole("radio", { name: SERVER }).check();
  await expect(live(page)).toHaveText("Your work was merged with your account: 1 place sent, 0 places received.");
  await expect(page.locator("#merge-report")).toHaveText(
    "Your work was merged with your account: 1 place sent, 0 places received.",
  );
  expect((await serverWork(context)).body.records.length).toBe(1);
});

test("a person who never chose server storage sends nothing and the menu shows no status", async ({
  page,
  context,
  baseURL,
}) => {
  await newAccount(page, context, baseURL!);
  await page.getByRole("radio", { name: /Keep my work on this device only/ }).check();
  await openStreet(page, 0);
  await widen(page);
  await page.locator("#session-btn").click();
  await expect(page.locator("#sync-status")).toHaveCount(0);
  expect((await serverWork(context)).status).toBe(409);
});

test("deleting the account deletes the server copy and keeps the browser's work", async ({
  page,
  context,
  baseURL,
}) => {
  await storingAccount(page, context, baseURL!);
  await openStreet(page, 0);
  await widen(page);
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  await page.goto("/account.html");
  await expect(page.getByText("including the work you stored there")).toHaveCount(0);
  await page.locator("#delete-btn").click();
  await expect(page.locator("#delete-warning")).toContainText("including the work you stored there");
  await page.getByRole("button", { name: "Delete my account" }).click();
  await expect(page).toHaveURL(/index\.html$/);
  expect((await serverWork(context)).status).toBe(401);
  expect(await editedStreets(page)).toHaveLength(1);
});

// A "received" count above zero cannot be reached by opting in: stopping deletes the server copy, so an account that is
// not storing on the server holds nothing there. The same merge runs, silently, when a device signs in to an account
// that already stores its work: the device receives what the server holds and sends what only it has.
test("a device that signs in to a storing account receives its places and sends its own", async ({
  page,
  context,
  browser,
  baseURL,
}) => {
  test.slow(); // two browser contexts on a shared server: 18-25 s under parallel load, against a 30 s timeout
  const sub = await storingAccount(page, context, baseURL!);
  await openStreet(page, 0);
  await widen(page);
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(1);
  const [onLaptop] = await editedStreets(page);
  const phone = await anotherBrowser(browser, baseURL!);
  await openStreet(phone.page, 1);
  await widen(phone.page); // signed out: marked, not sent
  await expect.poll(async () => (await editedStreets(phone.page)).length).toBe(1); // kept after the debounce
  const [onPhone] = await editedStreets(phone.page);
  expect(onPhone).not.toBe(onLaptop);
  await loginAs(phone.context, baseURL!, { sub, name: "Ada", email: "ada@example.org" });
  await phone.page.goto("/map.html");
  await chooseProvider(phone.page, "fake");
  await expect(phone.page.locator("#session-btn-label")).toHaveText("Ada");
  await expect.poll(async () => (await editedStreets(phone.page)).sort()).toEqual([onLaptop, onPhone].sort()); // received
  await expect.poll(async () => (await serverWork(context)).body?.records.length).toBe(2); // sent
  await phone.context.close();
});

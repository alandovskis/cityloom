// Evidence capture for #60 (not part of the PR): drives the pedestrian calls step by step and keeps a numbered
// screenshot of the plan's calls section, and the live region's words, at each step.
import { expect, live, openJunction, test } from "/Users/alex/src/cityloom/main-issue-60/e2e/tests/fixtures.ts";
import { appendFileSync, writeFileSync } from "node:fs";
import type { Page } from "/Users/alex/src/cityloom/main-issue-60/e2e/node_modules/@playwright/test/index.js";

const OUT = "/private/tmp/claude-501/-Users-alex-src-cityloom-main/9e65ae91-6924-4fa7-b704-f4926a0b84f7/scratchpad/ev/pub";
const LOG = `${OUT}/steps.txt`;

async function shot(page: Page, file: string, caption: string) {
  await page.locator("#signal-calls").screenshot({ path: `${OUT}/${file}` });
  const said = (await live(page).innerText()).trim();
  appendFileSync(LOG, `${file}\n  ${caption}\n  live region: ${said || "(empty)"}\n`);
}

async function signalPlan(page: Page, n: number) {
  await openJunction(page, n);
  await expect(page.locator("#drawing")).toBeVisible();
  await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
  await page.locator("#jn-go-plan").click();
  await expect(page.locator("#signal-plan-state")).toBeVisible();
}

test.use({ viewport: { width: 1440, height: 6000 } });

test("pedestrian calls, step by step (junction 0, English then French)", async ({ page }) => {
  writeFileSync(LOG, "");
  await signalPlan(page, 0);
  const section = page.locator("#signal-calls");
  await expect(section).toContainText("Pedestrian calls");
  await shot(page, "01-recall.png", "Signal plan opened: every crossing on recall, the placeholder figures shown");

  const boxes = section.locator(".call-crossing input[type=checkbox]");
  const uid = async (n: number) => (await boxes.nth(n).getAttribute("id"))!.replace("call-on-", "");
  const [first, second] = [await uid(0), await uid(1)];
  const walks = page.locator(`#inspector input[id^="sig-walks-"][id$="-${first}"]`);
  for (let i = 0; i < (await walks.count()); i++) if (await walks.nth(i).isChecked()) await walks.nth(i).uncheck();
  await page.locator(`#call-on-${first}`).click();
  await expect(page.locator(`#call-on-${first}`)).not.toBeChecked();
  await expect(live(page)).toContainText("cannot be on call");
  await shot(page, "02-refused.png", "Walk taken out of every phase for the first crossing, then On call pressed: refused, with the reason");

  await page.locator(`#call-on-${second}`).click();
  await expect(page.locator(`#call-on-${second}`)).toBeChecked();
  const rate = page.locator(`#call-rate-${second}`);
  await rate.fill("30");
  await rate.press("Enter");
  await expect(rate).toHaveValue("30");
  await expect(live(page)).toContainText("30 calls an hour");
  await shot(page, "03-on-call-30.png", "Second crossing put on call, 30 calls an hour: its walk + clearance, and the totals for the hour");

  await rate.fill("61");
  await rate.press("Enter");
  await expect(rate).toHaveValue("30");
  await shot(page, "04-rate-refused.png", "61 calls an hour typed: refused as outside the placeholder bound, the rate stays 30");

  await page.reload();
  await expect(page.locator("#drawing")).toBeVisible();
  await page.locator("#jn-go-plan").click();
  await expect(page.locator("#signal-plan-state")).toBeVisible();
  await expect(page.locator(`#call-on-${second}`)).toBeChecked();
  await expect(page.locator(`#call-rate-${second}`)).toHaveValue("30");
  await shot(page, "04b-after-reload.png", "Page reloaded and the plan reopened: the crossing is still on call at 30 calls an hour (saved with the junction)");

  const region = page.locator("#calls-totals").locator("xpath=ancestor::*[@role='region'][1]");
  await region.focus();
  await page.keyboard.press("End");
  await region.evaluate((el) => (el.scrollLeft = el.scrollWidth));
  await shot(page, "05-totals-scrolled.png", "Totals table focused (a labelled region) and scrolled to its last column, Longest wait");

  await page.locator("#calls-rows summary").click();
  await expect(page.locator("#calls-rows tbody tr")).toHaveCount(30);
  await shot(page, "06-every-call.png", "Every call, one row each (30): arrival time and outcome (served after / delayed)");

  await page.locator("#account-btn").click();
  await page.getByRole("button", { name: "Français" }).click();
  await expect(section).toContainText("Appels piétons");
  await page.keyboard.press("Escape");
  await shot(page, "07-french.png", "Switched to fr-CA: the same section in French, with OQLF spacing");
});

test("a crossing already on call keeps its call when an edit takes its walk away, as a finding (junction 0)", async ({ page }) => {
  await signalPlan(page, 0);
  const section = page.locator("#signal-calls");
  const boxes = section.locator(".call-crossing input[type=checkbox]");
  const uid = (await boxes.nth(1).getAttribute("id"))!.replace("call-on-", "");
  await page.locator(`#call-on-${uid}`).click();
  await expect(page.locator(`#call-on-${uid}`)).toBeChecked();
  await expect(page.locator("#calls-findings")).toHaveCount(0);
  await shot(page, "09a-on-call-walked.png", "A crossing put on call while a phase walks it: no findings");
  const walks = page.locator(`#inspector input[id^="sig-walks-"][id$="-${uid}"]`);
  for (let i = 0; i < (await walks.count()); i++) if (await walks.nth(i).isChecked()) await walks.nth(i).uncheck();
  await expect(page.locator(`#call-on-${uid}`)).toBeChecked();
  await expect(page.locator("#calls-findings")).toBeVisible();
  appendFileSync(LOG, `09b findings text: ${(await page.locator("#calls-findings").innerText()).replace(/\s+/g, " ")}\n09b crossing status: ${(await page.locator(`#call-status-${uid}`).innerText()).trim()}\n`);
  await shot(page, "09b-walk-taken-away.png", "Then its walk unticked in every phase: it stays on call (box still ticked), and the findings say no phase walks it; the edit's announcement says so too");
  await page.locator("#calls-findings").screenshot({ path: `${OUT}/09c-finding-closeup.png` });
});

test("a crossing too long for any phase's green is a finding (junction 3)", async ({ page }) => {
  await signalPlan(page, 3);
  for (const box of await page.locator(".call-crossing input[type=checkbox]").all()) await box.click();
  const phases = await page.locator("#inspector .signal-list > li").count();
  for (let i = 0; i < phases; i++) {
    await page.locator("#inspector .signal-list > li > button").nth(i).click();
    const green = page.locator('#inspector input[aria-labelledby="i-h-green"]');
    await green.fill("5");
    await green.press("Enter");
    await expect(green).toHaveValue("5");
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.locator("#jn-go-plan").click();
    await expect(page.locator("#signal-plan-state")).toBeVisible();
  }
  await expect(page.locator("#calls-findings")).toContainText("cannot be served safely");
  const region = page.locator("#calls-totals").locator("xpath=ancestor::*[@role='region'][1]");
  await region.evaluate((el) => (el.scrollLeft = el.scrollWidth));
  await page.locator("#calls-rows summary").click();
  const extended = page.locator("#calls-rows tbody tr").filter({ hasText: /xtended by \d+ s/ });
  await expect(extended.first()).toBeVisible();
  const totals = (await page.locator("#calls-totals").innerText()).replace(/\s+/g, " ");
  appendFileSync(LOG, `08 totals table text: ${totals}\n08 rows with an extension: ${await extended.count()}, first: ${(await extended.first().innerText()).replace(/\s+/g, " ")}\n`);
  await page.locator("#calls-totals").locator("xpath=ancestor::*[@role='region'][1]").screenshot({ path: `${OUT}/08a-totals-closeup.png` });
  await page.locator("#calls-findings").screenshot({ path: `${OUT}/08b-finding-closeup.png` });
  await shot(page, "08-finding.png", "Junction 3, every crossing on call and every green at 5 s: greens extended for the shorter crossings (totals scrolled to Green extended / Longest wait, rows open), and the 21 s crossing cannot be served safely (at most 20 s)");
});

import { appendFileSync, writeFileSync } from "node:fs";
import { expect, live, openJunction, test } from "./fixtures";
import type { Page } from "@playwright/test";

const OUT = "/private/tmp/claude-501/-Users-alex-src-cityloom/4cb51d1a-df55-4d28-90b1-a94549351273/scratchpad/ev115";
let n = 0;
const log = (s: string) => appendFileSync(`${OUT}/log.txt`, s + "\n");
const polled = async (page: Page, ms = 1500) => {
  const seen: string[] = [];
  for (let t = 0; t < ms; t += 100) {
    const v = ((await live(page).textContent()) ?? "").trim();
    if (v && !seen.includes(v)) seen.push(v);
    await page.waitForTimeout(100);
  }
  return seen;
};
const state = async (page: Page, tag: string) => {
  const card = page.locator("#inspector .signal-phases .phase-card");
  const out: string[] = [];
  for (let i = 0; i < (await card.count()); i++) {
    const c = card.nth(i);
    const moves = await c.locator("svg.phase-svg .ph-move").evaluateAll((els) =>
      els.map((e) => `${e.getAttribute("class")}|dash=${e.querySelector(".ph-line")?.getAttribute("stroke-dasharray")}`),
    );
    const wl = page.locator("#inspector .phase-conflicts").nth(i);
    const lists = await page.locator("#inspector .phase-conflicts").count();
    const items = i < lists ? await wl.locator("li").allInnerTexts() : [];
    out.push(`   card ${i + 1}: warn-moves=${moves.filter((m) => /\bwarn\b/.test(m)).length} lists-in-inspector=${lists} list-items=${items.length} items=${JSON.stringify(items)}\n     moves=${JSON.stringify(moves)}`);
  }
  log(`   [${tag}]\n${out.join("\n")}`);
};
const shot = async (page: Page, name: string, target?: string) => {
  n++;
  const f = `${OUT}/s${String(n).padStart(2, "0")}-${name}.png`;
  if (target) await page.locator(target).first().screenshot({ path: f });
  else await page.screenshot({ path: f });
  log(`   screenshot: ${f.split("/").pop()}`);
};
const checksText = async (page: Page, hasText: RegExp | string) => {
  await page.locator("#t-checks").click();
  const li = page.locator("#p-checks li", { hasText });
  const r = { cls: await li.getAttribute("class"), text: (await li.innerText()).replace(/\s+/g, " "), items: await li.locator("li, .detail li").count() };
  return r;
};

const full = async (page: Page, sel: string, file: string) => {
  const vp = page.viewportSize()!;
  await page.setViewportSize({ width: vp.width, height: 3200 });
  await page.waitForTimeout(400);
  const loc = page.locator(sel).first();
  await loc.scrollIntoViewIfNeeded();
  const m = await loc.evaluate((e) => ({ client: e.clientHeight, scroll: e.scrollHeight }));
  await loc.screenshot({ path: `${OUT}/${file}` });
  log(`   screenshot: ${file} (${sel} on a ${vp.width}x3200 window; clientHeight=${m.client} scrollHeight=${m.scroll})`);
  await page.setViewportSize(vp);
  await page.waitForTimeout(200);
};
const checkShots = async (page: Page, row: RegExp | string, tag: string) => {
  await page.locator("#t-checks").click();
  const li = page.locator("#p-checks li", { hasText: row }).first();
  await li.scrollIntoViewIfNeeded();
  await li.screenshot({ path: `${OUT}/${tag}-row.png` });
  log(`   screenshot: ${tag}-row.png (the "${typeof row === "string" ? row : row.source}" check row itself)`);
  await full(page, "#p-checks", `${tag}-panel.png`);
};
const toInspector = async (page: Page) => {
  if (await t.count()) await t.click();
};

test.setTimeout(240000);
test("evidence 115", async ({ page }) => {
  writeFileSync(`${OUT}/log.txt`, "Evidence for #115, HEAD e6b6342\n\n");
  await openJunction(page);
  await expect(page.locator("#drawing")).toBeVisible();
  await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
  const cards = page.locator("#inspector .signal-phases .phase-card");
  await expect(cards).toHaveCount(2);
  const walks = (k: number) => page.getByRole("group", { name: new RegExp(`^People walking in phase ${k}, `) });
  const meet = "Movements that meet";

  log("1. Suggested plan (English)");
  log(`   plan state: ${await page.locator("#signal-plan-state").innerText()}`);
  await state(page, "initial");
  await shot(page, "suggested-cards", "#inspector .signal-phases");
  const c1 = await checksText(page, meet);
  log(`   check class="${c1.cls}" text="${c1.text}"`);
  await checkShots(page, meet, "checks-1-suggested-pass");

  log("\n2. Switch on crossing of phase 1's own street");
  const off = walks(1).locator("input:not(:checked)").first();
  const id = await off.getAttribute("id");
  log(`   checkbox id=${id} name="${await off.evaluate((e: HTMLInputElement) => e.labels?.[0]?.innerText ?? e.getAttribute("aria-label"))}"`);
  await off.focus();
  await page.keyboard.press("Space");
  log(`   checked after toggle: ${await page.locator(`#${id}`).isChecked()}  (7: the edit was not refused)`);
  log(`   live (polled 1.5 s): ${JSON.stringify(await polled(page))}`);
  await state(page, "after switch on");
  log(`   list aria name visible: ${await page.getByRole("list", { name: "Warnings for phase 1" }).isVisible()}`);
  await shot(page, "warn-cards", "#inspector .signal-phases");
  await full(page, "#inspector .phase-conflicts", "list-en-phase1-full.png");
  await shot(page, "warn-card1-closeup", "#inspector .signal-phases .phase-card");
  const c2 = await checksText(page, meet);
  log(`   check class="${c2.cls}" text="${c2.text}"`);
  await checkShots(page, meet, "checks-2-one-crossing-fail");

  log("\n3. Non-colour cue: dashed warn movement vs lit non-warn movement");
  const c1svg = cards.first().locator("svg.phase-svg");
  const lit = await cards.nth(1).locator("svg.phase-svg").locator(".ph-move.lit:not(.warn) .ph-line").evaluateAll((els) => els.map((e) => `stroke-dasharray=${e.getAttribute("stroke-dasharray")} stroke=${getComputedStyle(e).stroke}`));
  const warn = await c1svg.locator(".ph-move.warn .ph-line").evaluateAll((els) => els.map((e) => `stroke-dasharray=${e.getAttribute("stroke-dasharray")} stroke=${getComputedStyle(e).stroke}`));
  log(`   lit non-warn lines (card 2): ${JSON.stringify(lit)}`);
  log(`   warn lines: ${JSON.stringify(warn)}`);
  await c1svg.screenshot({ path: `${OUT}/s${String(++n).padStart(2, "0")}-dashed-vs-lit-closeup.png`, scale: "device" });
  log(`   screenshot: s${String(n).padStart(2, "0")}-dashed-vs-lit-closeup.png`);
  await page.evaluate(() => {
    const s = document.querySelector("#inspector .phase-card svg.phase-svg") as SVGElement;
    s.style.transform = "scale(3)"; s.style.transformOrigin = "top left"; s.style.background = "";
    (s.parentElement as HTMLElement).style.overflow = "visible"; (s.parentElement as HTMLElement).style.minHeight = "460px";
  });
  await shot(page, "dashed-vs-lit-zoom3x", "#inspector .phase-card");
  await page.evaluate(() => { const s = document.querySelector("#inspector .phase-card svg.phase-svg") as SVGElement; s.style.transform = ""; (s.parentElement as HTMLElement).style.minHeight = ""; });

  log("\n4. Undo, redo, unrelated edit");
  const clear = () => page.evaluate(() => { document.querySelector("#live")!.textContent = ""; });
  await clear();
  await page.locator("#undo").click();
  log(`   after undo: live=${JSON.stringify(await polled(page))}  checked=${await page.locator(`#${id}`).isChecked()}`);
  await state(page, "after undo");
  await shot(page, "after-undo", "#inspector .signal-phases");
  await clear();
  await page.locator("#redo").click();
  log(`   after redo: live=${JSON.stringify(await polled(page))}  checked=${await page.locator(`#${id}`).isChecked()}`);
  await state(page, "after redo");
  await shot(page, "after-redo", "#inspector .signal-phases");
  await clear();
  const other = walks(2).locator("input:checked").first();
  await other.focus();
  await page.keyboard.press("Space");
  log(`   unrelated edit (phase 2 crossing off): live=${JSON.stringify(await polled(page))}`);
  await state(page, "after unrelated edit");
  await shot(page, "after-unrelated", "#inspector .signal-phases");
  await clear();
  await page.keyboard.press("Space"); // back on
  log(`   unrelated edit reverted (phase 2 crossing on): live=${JSON.stringify(await polled(page))}`);
  await clear();
  // Also: a second warning-making edit while one persists
  const off2 = walks(1).locator("input:not(:checked)").first();
  const id2 = await off2.getAttribute("id");
  await off2.focus();
  await page.keyboard.press("Space");
  log(`   second crossing on in phase 1 (${id2}): live=${JSON.stringify(await polled(page))}`);
  await state(page, "second crossing");
  await shot(page, "second-crossing", "#inspector .signal-phases");

  log("\n6. Many conflicts: switch on every unchecked crossing and bike box in phases 1 and 2");
  for (const k of [1, 2]) {
    for (let g = 0; g < 8; g++) {
      const o = walks(k).locator("input:not(:checked)").first();
      if (!(await o.count())) break;
      if (await o.isDisabled()) break;
      await o.focus(); await page.keyboard.press("Space");
    }
    const bikes = page.getByRole("group", { name: new RegExp(`^Bikes.* phase ${k}, `) });
    for (let g = 0; g < 8; g++) {
      const o = bikes.locator("input:not(:checked)").first();
      if (!(await o.count())) break;
      await o.focus(); await page.keyboard.press("Space");
    }
  }
  await state(page, "many");
  await shot(page, "many-cards", "#inspector .signal-phases");
  await full(page, "#inspector .phase-conflicts", "list-en-many-phase1-full.png");
  const c6 = await checksText(page, meet);
  log(`   check class="${c6.cls}"\n   check text="${c6.text}"`);
  const sentences = await page.locator("#p-checks li", { hasText: meet }).evaluate((e) => (e.textContent ?? "").split(/(?<=\.)\s+/).length);
  log(`   check sentence-ish segments: ${sentences}; "And N more" present: ${/And \d+ more/.test(c6.text)}`);
  await checkShots(page, meet, "checks-3-nine-conflicts-fail");

  log("\n5. Switch the crossings back off");
  for (let pass = 0; pass < 3; pass++)
    for (const k of [1, 2]) {
      const all = walks(k).locator("input:checked");
      // turn off checked boxes in phase 1 that are not the suggested ones is hard; use Restore instead
    }
  await page.locator("#signal-restore").click();
  await page.waitForTimeout(500);
  await state(page, "after Restore (suggested)");
  const c5 = await checksText(page, meet);
  log(`   check class="${c5.cls}" text="${c5.text}"`);
  await checkShots(page, meet, "checks-4-restored-pass");
  await shot(page, "restored-cards", "#inspector .signal-phases");

  log("\n5b. Switch the single crossing off again (undo path was step 4; here toggling manually)");
  const off3 = walks(1).locator("input:not(:checked)").first();
  const id3 = await off3.getAttribute("id");
  await off3.focus(); await page.keyboard.press("Space");
  await expect(page.locator(`#${id3}`)).toBeChecked();
  await state(page, "on");
  await clear();
  await page.locator(`#${id3}`).focus(); await page.keyboard.press("Space");
  log(`   switched off: checked=${await page.locator(`#${id3}`).isChecked()} live=${JSON.stringify(await polled(page))}`);
  await state(page, "off again");
  await shot(page, "off-again-cards", "#inspector .signal-phases");
  const c5b = await checksText(page, meet);
  log(`   check class="${c5b.cls}" text="${c5b.text}"`);
  await checkShots(page, meet, "checks-5-off-again-pass");

  log("\n8. French (fr-CA)");
  await page.evaluate(() => localStorage.setItem("cityloom-lang", "fr-CA"));
  await page.reload();
  await expect(page.locator("#drawing")).toBeVisible();
  const fwalks = page.getByRole("group", { name: /^Piétons en marche à la phase 1, / });
  await clear();
  const f = fwalks.locator("input:not(:checked)").first();
  await f.focus(); await page.keyboard.press("Space");
  log(`   live fr: ${JSON.stringify(await polled(page))}`);
  await state(page, "fr");
  await shot(page, "fr-cards", "#inspector .signal-phases");
  await full(page, "#inspector .phase-conflicts", "list-fr-phase1-full.png");
  const c8 = await checksText(page, "Mouvements qui se rencontrent");
  log(`   fr check class="${c8.cls}" text="${c8.text}"`);
  await checkShots(page, "Mouvements qui se rencontrent", "checks-6-fr-fail");
});

import { expect, test as base, type Page } from "@playwright/test";

/** Every test fails if its page throws or logs a console error. Each test gets a
 *  fresh browser context, so the page starts with empty storage. */
export const test = base.extend<{ errors: string[] }>({
  errors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(e.message));
      page.on("console", (m) => {
        if (m.type() === "error") errors.push(m.text());
      });
      await use(errors);
      expect(errors).toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };

/** What a screen reader was last told. */
export const live = (page: Page) => page.locator("#live");

/** The sample city, as the map lists it: the junction and street the tests open. */
export const JUNCTION = 3; // "Junction 1"
export const STREET = 1; // "Sample Avenue 2", between the edge of the map and Junction 4

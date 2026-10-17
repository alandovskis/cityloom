import { expect, forgive, mapReady, placesOnMap, test } from "./fixtures";

const PAGES = ["/", "/map.html", "street", "junction", "/account.html"];
const IMMUTABLE = "public, max-age=31536000, immutable";

test.describe("the delivery of the site's code", () => {
  test("every page names its code by versioned addresses, all of them answered and kept for a year", async ({
    page,
  }) => {
    for (const path of PAGES) {
      const code: { url: string; status: number; cache: string | undefined }[] = [];
      page.on("response", (res) => {
        const url = new URL(res.url());
        if (/\.(js|mjs|css|wasm|svg)$/.test(url.pathname) && !/favicon|logo-|\/boot\.js$/.test(url.pathname))
          code.push({ url: url.pathname, status: res.status(), cache: res.headers()["cache-control"] });
      });
      if (path === "street" || path === "junction") {
        await page.goto("/map.html");
        const places = await placesOnMap(page);
        code.length = 0;
        await page.goto("/" + (path === "street" ? places.streets[0] : places.junctions[0]));
      } else await page.goto(path);
      // the shell is bound, and fills in the version, when the module has started
      await expect(page.locator("[data-version]")).not.toBeEmpty();
      page.removeAllListeners("response");
      const build = new Set(code.map((c) => /^\/v\/([0-9a-f]{16})\//.exec(c.url)?.[1]));
      expect(code.length, path).toBeGreaterThan(3);
      expect(
        code.filter((c) => !c.url.startsWith("/v/")).map((c) => c.url),
        `${path}: unversioned code`,
      ).toEqual([]);
      expect(build.size, `${path}: one build`).toBe(1);
      expect(
        code.filter((c) => c.status !== 200 || c.cache !== IMMUTABLE),
        path,
      ).toEqual([]);
      expect(
        code.map((c) => c.url).some((u) => u.endsWith("/pkg/cityloom_editor_bg.wasm")),
        path,
      ).toBe(true);
    }
  });

  test("the pages themselves are revalidated, with a tag that changes with the build", async ({ request }) => {
    const page = await request.get("/map.html");
    expect(page.headers()["cache-control"]).toMatch(/^no-(cache|store)$/);
    expect(page.headers()["etag"]).toBeTruthy();
    expect((await request.get("/map.html", { headers: { "if-none-match": page.headers()["etag"] } })).status()).toBe(
      304,
    );
  });

  test("the module is sent in Brotli, or gzip to a client that takes no Brotli, and it is smaller than the file", async ({
    request,
  }) => {
    const wasm = "/pkg/cityloom_editor_bg.wasm";
    const plain = (await request.get(wasm, { headers: { "accept-encoding": "identity" } })).headers();
    const br = await request.get(wasm, { headers: { "accept-encoding": "br, gzip" } });
    const gz = await request.get(wasm, { headers: { "accept-encoding": "gzip" } });
    expect(plain["content-encoding"]).toBeUndefined();
    expect(br.headers()["content-encoding"]).toBe("br");
    expect(gz.headers()["content-encoding"]).toBe("gzip");
    expect(Number(plain["content-length"])).toBeGreaterThan(2_000_000);
    expect(Number(br.headers()["content-length"])).toBeLessThan(Number(gz.headers()["content-length"]));
    expect(Number(gz.headers()["content-length"])).toBeLessThan(Number(plain["content-length"]) / 2);
    // and what comes out is the module
    expect((await br.body()).subarray(0, 4)).toEqual(Buffer.from([0, 0x61, 0x73, 0x6d]));
  });

  test("a build that is not the current one is not served", async ({ request }) => {
    const res = await request.get("/v/0000000000000000/pkg/cityloom_editor.js");
    expect(res.status()).toBe(404);
    expect(res.headers()["cache-control"]).toBe("no-store");
  });

  test("the glyphs and the tile index are kept for a while, the tiles are always revalidated", async ({ request }) => {
    // (the tile index is a file only after `just prepare`, and the fixture serves it by a route: the server's own
    // tests pin its header; the glyphs are in the repository)
    const glyph = await request.get("/glyphs/Noto%20Sans%20Regular/0-255.pbf");
    expect(glyph.status()).toBe(200);
    expect(glyph.headers()["cache-control"]).toContain("max-age=86400");
  });
});

test.describe("a module that does not start", () => {
  const WASM = "**/pkg/cityloom_editor_bg.wasm";

  for (const [name, answer] of [
    ["is not found", (route: any) => route.fulfill({ status: 404, body: "no" })],
    [
      "is not a module",
      (route: any) => route.fulfill({ status: 200, contentType: "application/wasm", body: Buffer.from("nope") }),
    ],
  ] as const) {
    test(`says so in English when the module ${name}`, async ({ page, errors }) => {
      await page.route(WASM, answer);
      await page.goto("/map.html");
      const banner = page.locator("#boot-failed");
      await expect(banner).toBeVisible();
      await expect(banner).toHaveAttribute("role", "alert");
      await expect(banner).toContainText(
        "CityLoom could not load. Reload the page; if that does not help, check your connection.",
      );
      await expect(banner.getByRole("link", { name: "Reload" })).toBeVisible();
      await expect(banner.locator('[lang="fr-CA"]')).toBeHidden();
      forgive(errors, /./);
      errors.length = 0;
    });
  }

  test("says so in French when the person chose French", async ({ page, errors }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.route(WASM, (route) => route.fulfill({ status: 404, body: "no" }));
    await page.goto("/street.html?street=1");
    const banner = page.locator("#boot-failed");
    await expect(banner).toBeVisible();
    await expect(banner).toContainText(
      "CityLoom n’a pas pu se charger. Rechargez la page; si cela ne règle rien, vérifiez votre connexion.",
    );
    await expect(banner.getByRole("link", { name: "Recharger" })).toBeVisible();
    await expect(banner.locator('[lang="en"]')).toBeHidden();
    forgive(errors, /./);
    errors.length = 0;
  });

  test("follows the browser's language when none was chosen", async ({ browser, baseURL }) => {
    const context = await browser.newContext({ baseURL, locale: "fr-CA" });
    const page = await context.newPage();
    await page.route(WASM, (route) => route.fulfill({ status: 404, body: "no" }));
    await page.goto("/");
    await expect(page.locator("#boot-failed")).toContainText("n’a pas pu se charger");
    await context.close();
  });

  // A deploy retires the addresses of the build an open page belongs to. Which request meets the new server first
  // is a matter of chance, so each file the page's start depends on is failed in turn.
  for (const [name, glob] of [
    ["entry script", "**/v/*/map.js"],
    ["script it imports", "**/v/*/city.js"],
    ["glue", "**/v/*/pkg/cityloom_editor.js"],
  ] as const) {
    test(`says so when the ${name} is not found under its build`, async ({ page, errors }) => {
      await page.route(glob, (route) => route.fulfill({ status: 404, body: "this page was made for another build" }));
      await page.goto("/map.html");
      const banner = page.locator("#boot-failed");
      await expect(banner).toBeVisible();
      await expect(banner).toContainText("CityLoom could not load.");
      await expect(banner.getByRole("link", { name: "Reload" })).toBeVisible();
      forgive(errors, /./);
      errors.length = 0;
    });
  }

  test("says so in French when the entry script is not found", async ({ page, errors }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.route("**/v/*/home.js", (route) => route.fulfill({ status: 404, body: "no" }));
    await page.goto("/");
    await expect(page.locator("#boot-failed")).toContainText("n’a pas pu se charger");
    forgive(errors, /./);
    errors.length = 0;
  });

  test("shows nothing when the module starts", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#search")).toBeVisible();
    await expect(page.locator("#boot-failed")).toBeHidden();
  });
});

test.describe("the reader of the roads after a deploy", () => {
  test("is told as the page is, and is tried again by the next call", async ({ page, errors }) => {
    // The first visit reads the roads and keeps them; the second does not need the reader, so it is not loaded yet.
    await page.goto("/map.html");
    await mapReady(page);
    await page.reload();
    await mapReady(page);
    await expect(page.locator("#boot-failed")).toBeHidden();
    // A new build went live while the page was open: its reader's address is gone.
    await page.route("**/v/*/pkg/osm_import.js", (route) => route.fulfill({ status: 404, body: "gone" }));
    const call = () =>
      page.evaluate(async () => {
        try {
          await (window as any).cityloomImportOsm(new Uint8Array(), "{}");
          return "read";
        } catch (error) {
          return String(error);
        }
      });
    const first = await call();
    expect(first).toMatch(/osm_import|module|fetch|import/i);
    await expect(page.locator("#boot-failed")).toBeVisible();
    await expect(page.locator("#boot-failed")).toContainText("CityLoom could not load.");
    // The failure is not kept: once the address answers, the reader loads (and then reads nothing from no bytes).
    await page.unroute("**/v/*/pkg/osm_import.js");
    const requested = page.waitForRequest(/\/pkg\/osm_import\.js\?retry=2$/);
    const second = await call();
    await requested;
    expect(second).not.toBe(first);
    forgive(errors, /./);
    errors.length = 0;
  });
});

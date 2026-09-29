import { mkdir } from "node:fs/promises";
import { chromium } from "playwright";

const baseUrl = process.env.VISUAL_BASE_URL ?? "http://127.0.0.1:4173";
const outputDir = process.env.VISUAL_OUTPUT_DIR ?? "visual-evidence";

const cases = [
  { name: "setup-1024x700", width: 1024, height: 700, path: "/?visual=setup", active: "Overview", locale: "en" },
  { name: "setup-1440x900", width: 1440, height: 900, path: "/?visual=setup", active: "Overview", locale: "en" },
  { name: "setup-fr-1024x700", width: 1024, height: 700, path: "/?visual=setup", active: "Vue d’ensemble", locale: "fr" },
  { name: "setup-fr-1440x900", width: 1440, height: 900, path: "/?visual=setup", active: "Vue d’ensemble", locale: "fr" },
  { name: "overview-1024x700", width: 1024, height: 700, path: "/?visual=overview", active: "Overview", locale: "en" },
  { name: "overview-1440x900", width: 1440, height: 900, path: "/?visual=overview", active: "Overview", locale: "en" },
  { name: "overview-fr-1024x700", width: 1024, height: 700, path: "/?visual=overview", active: "Vue d’ensemble", locale: "fr" },
  { name: "overview-fr-1440x900", width: 1440, height: 900, path: "/?visual=overview", active: "Vue d’ensemble", locale: "fr" },
  { name: "overview-partial-1024x700", width: 1024, height: 700, path: "/?visual=overview-partial", active: "Overview", locale: "en" },
  { name: "overview-partial-1440x900", width: 1440, height: 900, path: "/?visual=overview-partial", active: "Overview", locale: "en" },
  { name: "overview-partial-fr-1024x700", width: 1024, height: 700, path: "/?visual=overview-partial", active: "Vue d’ensemble", locale: "fr" },
  { name: "overview-partial-fr-1440x900", width: 1440, height: 900, path: "/?visual=overview-partial", active: "Vue d’ensemble", locale: "fr" },
  { name: "library-1024x700", width: 1024, height: 700, path: "/?surface=library&visual=library", active: "Library" },
  { name: "library-1440x900", width: 1440, height: 900, path: "/?surface=library&visual=library", active: "Library" },
  { name: "detail-1024x700", width: 1024, height: 700, path: "/?surface=detail&mod=rpo&visual=detail", active: "Library" },
  { name: "detail-1440x900", width: 1440, height: 900, path: "/?surface=detail&mod=rpo&visual=detail", active: "Library" },
  {
    name: "library-offline-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=library&state=offline&visual=library",
    active: "Library"
  },
  {
    name: "health-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=health&tab=updates&visual=health",
    active: "Health"
  },
  {
    name: "health-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=health&tab=updates&visual=health",
    active: "Health"
  },
  {
    name: "health-pack-detail-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=health&tab=updates&visual=health-pack",
    active: "Health"
  },
  {
    name: "health-pack-detail-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=health&tab=updates&visual=health-pack",
    active: "Health"
  },
  {
    name: "discover-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=discover&visual=discover",
    active: "Discover"
  },
  {
    name: "discover-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=discover&visual=discover",
    active: "Discover"
  },
  {
    name: "discover-blocked-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=discover&visual=discover-blocked",
    active: "Discover",
    locale: "en"
  },
  {
    name: "discover-blocked-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=discover&visual=discover-blocked",
    active: "Discover",
    locale: "en"
  },
  {
    name: "discover-blocked-fr-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=discover&visual=discover-blocked",
    active: "Découvrir",
    locale: "fr"
  },
  {
    name: "discover-blocked-fr-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=discover&visual=discover-blocked",
    active: "Découvrir",
    locale: "fr"
  },
  {
    name: "settings-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=settings",
    active: "Settings"
  },
  {
    name: "settings-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=settings",
    active: "Settings"
  }
];

await mkdir(outputDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage();

try {
  for (const testCase of cases) {
    await page.setViewportSize({ width: testCase.width, height: testCase.height });
    await page.goto(new URL("/", baseUrl).toString(), { waitUntil: "domcontentloaded" });
    await page.evaluate(
      ({ locale }) => localStorage.setItem("sims-mod-health.locale", locale),
      { locale: testCase.locale ?? "en" }
    );
    await page.goto(new URL(testCase.path, baseUrl).toString(), { waitUntil: "networkidle" });

    if (testCase.path.includes("visual=overview")) {
      await page.locator("#overview-title").waitFor();
    }
    if (testCase.path.includes("visual=setup")) {
      await page.locator("#setup-title").waitFor();
    }
    if (testCase.path.includes("surface=health")) {
      await page.locator("#health-title").waitFor();
    }
    if (testCase.path.includes("surface=discover")) {
      await page.locator("#discover-title").waitFor();
    }
    if (testCase.path.includes("surface=settings")) {
      await page.locator("#settings-title").waitFor();
    }

    const layout = await page.evaluate(() => ({
      viewportWidth: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      activeNavigation: document.querySelector('[aria-current="page"]')?.textContent?.trim()
    }));

    if ((testCase.locale ?? "en") === "fr") {
      const frenchToggle = page.getByRole("button", { name: "Français" });
      if ((await frenchToggle.getAttribute("aria-pressed")) !== "true") {
        throw new Error(testCase.name + " did not persist the French locale");
      }
    }

    if (layout.documentWidth > layout.viewportWidth) {
      throw new Error(
        testCase.name + " has horizontal overflow: " +
          layout.documentWidth + "px document in " +
          layout.viewportWidth + "px viewport"
      );
    }

    if (!layout.activeNavigation?.includes(testCase.active)) {
      throw new Error(
        testCase.name + " does not expose " + testCase.active +
          " as the active navigation destination"
      );
    }

    if (testCase.path.includes("surface=library")) {
      const search = page.getByLabel("Search local filename");
      await search.focus();
      const focus = await search.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible keyboard focus state");
      }
    }

    if (testCase.path.includes("visual=health-pack")) {
      await page.getByRole("dialog", { name: "EP17" }).waitFor();
    }

    if (testCase.path.includes("surface=health")) {
      const updates = page.getByRole("button", { name: /Updates/ }).first();
      await updates.focus();
      const focus = await updates.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible Health focus state");
      }
    }

    if (testCase.path.includes("surface=discover")) {
      const search = page.getByLabel("Search recommendations");
      await search.focus();
      const focus = await search.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible Discover focus state");
      }
    }

    if (testCase.path.includes("surface=settings")) {
      const paths = page.getByRole("button", { name: "Game & folders" });
      await paths.focus();
      const focus = await paths.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible Settings focus state");
      }
    }

    if (testCase.path.includes("surface=detail")) {
      const back = page.getByRole("button", { name: "Back to Library" });
      await back.focus();
      const focus = await back.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible detail focus state");
      }
    }

    await page.screenshot({
      path: outputDir + "/" + testCase.name + ".png",
      fullPage: true
    });
  }
} finally {
  await browser.close();
}

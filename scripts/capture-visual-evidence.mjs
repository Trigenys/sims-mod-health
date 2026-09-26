import { mkdir } from "node:fs/promises";
import { chromium } from "playwright";

const baseUrl = process.env.VISUAL_BASE_URL ?? "http://127.0.0.1:4173";
const outputDir = process.env.VISUAL_OUTPUT_DIR ?? "visual-evidence";

const cases = [
  { name: "overview-1024x700", width: 1024, height: 700, path: "/?visual=overview", active: "Overview" },
  { name: "overview-1440x900", width: 1440, height: 900, path: "/?visual=overview", active: "Overview" },
  { name: "library-1024x700", width: 1024, height: 700, path: "/?surface=library", active: "Library" },
  { name: "library-1440x900", width: 1440, height: 900, path: "/?surface=library", active: "Library" },
  { name: "detail-1024x700", width: 1024, height: 700, path: "/?surface=detail&mod=rpo", active: "Library" },
  { name: "detail-1440x900", width: 1440, height: 900, path: "/?surface=detail&mod=rpo", active: "Library" },
  {
    name: "library-offline-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=library&state=offline",
    active: "Library"
  },
  {
    name: "diagnostics-1024x700",
    width: 1024,
    height: 700,
    path: "/?surface=diagnostics&visual=diagnostics",
    active: "Diagnostics"
  },
  {
    name: "diagnostics-1440x900",
    width: 1440,
    height: 900,
    path: "/?surface=diagnostics&visual=diagnostics",
    active: "Diagnostics"
  }
];

await mkdir(outputDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage();

try {
  for (const testCase of cases) {
    await page.setViewportSize({ width: testCase.width, height: testCase.height });
    await page.goto(new URL(testCase.path, baseUrl).toString(), { waitUntil: "networkidle" });

    if (testCase.path.includes("visual=overview")) {
      await page.locator("#overview-title").waitFor();
    }

    const layout = await page.evaluate(() => ({
      viewportWidth: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      activeNavigation: document.querySelector('[aria-current="page"]')?.textContent?.trim()
    }));

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
      const search = page.getByLabel("Search canonical name or filename");
      await search.focus();
      const focus = await search.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible keyboard focus state");
      }
    }

    if (testCase.path.includes("surface=diagnostics")) {
      await page.locator("#diagnostics-title").waitFor();
      const analyze = page.getByRole("button", { name: "Analyze reports" });
      await analyze.focus();
      const focus = await analyze.evaluate((element) => ({
        active: document.activeElement === element,
        ring: getComputedStyle(element).boxShadow
      }));
      if (!focus.active || focus.ring === "none") {
        throw new Error(testCase.name + " does not expose a visible Diagnostics focus state");
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

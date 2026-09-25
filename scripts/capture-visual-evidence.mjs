import { mkdir } from "node:fs/promises";
import { chromium } from "playwright";

const baseUrl = process.env.VISUAL_BASE_URL ?? "http://127.0.0.1:4173";
const outputDir = process.env.VISUAL_OUTPUT_DIR ?? "visual-evidence";

const cases = [
  { name: "overview-1024x700", width: 1024, height: 700 },
  { name: "overview-1440x900", width: 1440, height: 900 }
];

await mkdir(outputDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage();

try {
  for (const testCase of cases) {
    await page.setViewportSize({ width: testCase.width, height: testCase.height });
    await page.goto(baseUrl, { waitUntil: "networkidle" });

    const layout = await page.evaluate(() => ({
      viewportWidth: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      activeNavigation: document.querySelector('[aria-current="page"]')?.textContent?.trim()
    }));

    if (layout.documentWidth > layout.viewportWidth) {
      throw new Error(
        `${testCase.name} has horizontal overflow: ${layout.documentWidth}px document in ${layout.viewportWidth}px viewport`
      );
    }

    if (!layout.activeNavigation?.includes("Overview")) {
      throw new Error(`${testCase.name} does not expose Overview as the active navigation destination`);
    }

    await page.screenshot({
      path: `${outputDir}/${testCase.name}.png`,
      fullPage: true
    });
  }
} finally {
  await browser.close();
}

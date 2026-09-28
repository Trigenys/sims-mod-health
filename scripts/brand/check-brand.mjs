import { readFile } from "node:fs/promises";

const landing = await readFile(new URL("../../apps/landing/logo.svg", import.meta.url), "utf8");
const desktop = await readFile(new URL("../../src/assets/brand-logo.svg", import.meta.url), "utf8");

if (landing.trim() !== desktop.trim()) {
  console.error("Desktop brand logo diverged from apps/landing/logo.svg.");
  console.error("Run the brand sync/update so both product surfaces use the canonical landing asset.");
  process.exit(1);
}

console.log("Brand contract: landing and desktop use the same canonical SVG.");

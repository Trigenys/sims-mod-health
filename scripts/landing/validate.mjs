import { access, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const site = resolve(process.cwd(), "_site");
const requiredFiles = ["index.html", "release.js", "logo.svg", ".nojekyll"];

for (const file of requiredFiles) {
  await access(join(site, file));
}

const [html, js] = await Promise.all([
  readFile(join(site, "index.html"), "utf8"),
  readFile(join(site, "release.js"), "utf8")
]);

const requiredHtml = [
  "Keep your Sims 4 Mods healthy",
  'id="features"',
  'id="how-it-works"',
  'id="privacy-and-architecture"',
  'id="principles"',
  'id="download"',
  "Download Windows Beta 1",
  "data-download-link",
  "data-release-version",
  "data-download-size",
  "data-checksum-link",
  "./logo.svg",
  "./release.js",
  "cdn.tailwindcss.com",
  "Plus+Jakarta+Sans"
];

for (const token of requiredHtml) {
  if (!html.includes(token)) {
    throw new Error(`Approved Stitch landing is missing required marker: ${token}`);
  }
}

const fallback =
  "https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi";

if (!html.includes(fallback) || !js.includes(fallback)) {
  throw new Error("Beta 1 MSI fallback is missing from the built page or release wiring.");
}

for (const forbidden of [
  "v0.9.4",
  ".EXE",
  ".msix",
  "18.4 MB",
  "Digital Signature: Verified",
  "Zero Telemetry Guarantee",
  "Know what your Sims 4 Mods folder is doing."
]) {
  if (html.includes(forbidden)) {
    throw new Error(`Stale or misleading landing copy remains: ${forbidden}`);
  }
}

if (!js.includes("/releases?per_page=10")) {
  throw new Error("Runtime GitHub release lookup is missing.");
}

if (!js.includes('asset.name.toLowerCase().endsWith(".msi")')) {
  throw new Error("Release wiring must choose an MSI asset explicitly.");
}

for (const forbiddenAnalytics of [
  "googletagmanager",
  "google-analytics",
  "segment.com",
  "mixpanel",
  "hotjar"
]) {
  if (
    html.toLowerCase().includes(forbiddenAnalytics) ||
    js.toLowerCase().includes(forbiddenAnalytics)
  ) {
    throw new Error(`Unexpected analytics dependency: ${forbiddenAnalytics}`);
  }
}

console.log("Approved Stitch landing contract passed.");

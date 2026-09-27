import { access, readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const landing = join(root, "apps", "landing");

const files = ["index.html", "styles.css", "app.js", "favicon.svg", "vercel.json"];

for (const file of files) {
  await access(join(landing, file));
}

const [html, css, js] = await Promise.all([
  readFile(join(landing, "index.html"), "utf8"),
  readFile(join(landing, "styles.css"), "utf8"),
  readFile(join(landing, "app.js"), "utf8")
]);

const requiredHtml = [
  'id="features"',
  'id="privacy"',
  'id="beta"',
  "Download for Windows",
  "Raw mods stay local",
  "data-download-link",
  "data-checksum-link",
  "./styles.css",
  "./app.js"
];

for (const token of requiredHtml) {
  if (!html.includes(token)) {
    throw new Error(`Landing is missing required markup: ${token}`);
  }
}

const fallbackAsset =
  "https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi";

if (!html.includes(fallbackAsset) || !js.includes(fallbackAsset)) {
  throw new Error("The current Beta 1 MSI fallback must exist in both markup and runtime metadata.");
}

if (!js.includes("/releases?per_page=10")) {
  throw new Error("Runtime release discovery is missing.");
}

if (!js.includes("asset.name.toLowerCase().endsWith(\".msi\")")) {
  throw new Error("Runtime release discovery must select an MSI asset explicitly.");
}

const externalScripts = [
  ...html.matchAll(/<script\s+[^>]*src=["']([^"']+)["']/gi)
].map((match) => match[1]).filter((src) => /^https?:\/\//i.test(src));

const externalStyles = [
  ...html.matchAll(/<link\s+[^>]*rel=["']stylesheet["'][^>]*href=["']([^"']+)["']/gi)
].map((match) => match[1]).filter((href) => /^https?:\/\//i.test(href));

if (externalScripts.length || externalStyles.length) {
  throw new Error(
    `Landing must render without third-party script/style dependencies. Scripts: ${externalScripts.join(", ")} Styles: ${externalStyles.join(", ")}`
  );
}

for (const forbidden of ["googletagmanager", "google-analytics", "segment.com", "mixpanel", "hotjar"]) {
  if (html.toLowerCase().includes(forbidden) || js.toLowerCase().includes(forbidden)) {
    throw new Error(`Unexpected analytics dependency: ${forbidden}`);
  }
}

if (!css.includes("@media (max-width: 700px)")) {
  throw new Error("Mobile layout breakpoint is missing.");
}

if (!css.includes("prefers-reduced-motion")) {
  throw new Error("Reduced-motion handling is missing.");
}

console.log("Landing static contract passed.");

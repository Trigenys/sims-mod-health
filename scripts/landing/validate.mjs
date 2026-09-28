import { access, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const site = resolve(process.cwd(), "_site");
const requiredFiles = [
  "index.html",
  "release.js",
  "i18n.js",
  "logo.svg",
  ".nojekyll",
  "download/index.html",
  "release-notes/index.html",
  "verify/index.html",
  "architecture/index.html",
  "security/index.html",
  "docs/index.html",
  "support/index.html",
  "source/index.html",
  "404.html"
];

for (const file of requiredFiles) {
  await access(join(site, file));
}

const [html, js, i18n] = await Promise.all([
  readFile(join(site, "index.html"), "utf8"),
  readFile(join(site, "release.js"), "utf8"),
  readFile(join(site, "i18n.js"), "utf8")
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
  'rel="icon" type="image/svg+xml" href="/logo.svg"',
  "./release.js",
  "cdn.tailwindcss.com",
  "Plus+Jakarta+Sans",
  "max-w-[1600px]",
  "max-w-[1480px]",
  "data-lang-switch",
  "data-product-header",
  'aria-current="page"',
  'data-copy-fr="Accueil"',
  'data-copy-fr="Télécharger"',
  'data-copy-fr="Documentation"',
  'data-copy-fr="Sécurité"',
  'data-copy-fr="Assistance"',
  'data-lang="en"',
  'data-lang="fr"',
  "./i18n.js",
  'href="/download/"',
  'href="/release-notes/"',
  'href="/verify/"',
  'href="/architecture/"',
  'href="/security/"',
  'href="/docs/"',
  'href="/source/"'
];

for (const token of requiredHtml) {
  if (!html.includes(token)) {
    throw new Error(`Approved Stitch landing is missing required marker: ${token}`);
  }
}

const fallback =
  "https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi";

const downloadPageForFallback = await readFile(
  join(site, "download", "index.html"),
  "utf8"
);

if (!downloadPageForFallback.includes(fallback) || !js.includes(fallback)) {
  throw new Error(
    "Beta 1 MSI fallback must exist on the branded download page and in release wiring."
  );
}

for (const forbidden of [
  "max-w-7xl",
  "max-w-6xl",
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

if (/href="https:\/\/github\.com\/Trigenys\/sims-mod-health/i.test(html)) {
  throw new Error("Primary landing links must stay on the branded site before GitHub.");
}

if (!js.includes("/releases?per_page=10")) {
  throw new Error("Runtime GitHub release lookup is missing.");
}

if (!js.includes('asset.name.toLowerCase().endsWith(".msi")')) {
  throw new Error("Release wiring must choose an MSI asset explicitly.");
}

for (const marker of [
  "[data-binary-download-link]",
  "[data-external-release-link]",
  "[data-external-checksum-link]"
]) {
  if (!js.includes(marker)) {
    throw new Error(`Release runtime is missing professional-route marker: ${marker}`);
  }
}

const downloadPage = downloadPageForFallback;
if (!downloadPage.includes("data-binary-download-link")) {
  throw new Error("Download page must own the real MSI handoff.");
}

for (const [label, pageHtml, activeHref] of [
  ["home", html, "/"],
  ["download", downloadPage, "/download/"],
  ["docs", await readFile(join(site, "docs", "index.html"), "utf8"), "/docs/"],
  ["security", await readFile(join(site, "security", "index.html"), "utf8"), "/security/"],
  ["support", await readFile(join(site, "support", "index.html"), "utf8"), "/support/"]
]) {
  if (!pageHtml.includes("data-product-header")) {
    throw new Error(`${label} page must use the canonical product header.`);
  }

  const activeMarker = `href="${activeHref}" aria-current="page"`;
  if (!pageHtml.includes(activeMarker)) {
    throw new Error(
      `${label} page must expose its canonical active navigation state: ${activeHref}`
    );
  }
}

const faviconMarker = 'rel="icon" type="image/svg+xml" href="/logo.svg"';
for (const [label, path] of [
  ["home", "index.html"],
  ["download", "download/index.html"],
  ["release notes", "release-notes/index.html"],
  ["verify", "verify/index.html"],
  ["architecture", "architecture/index.html"],
  ["security", "security/index.html"],
  ["docs", "docs/index.html"],
  ["support", "support/index.html"],
  ["source", "source/index.html"],
  ["404", "404.html"]
]) {
  const pageHtml =
    path === "index.html" ? html : await readFile(join(site, path), "utf8");

  if (!pageHtml.includes(faviconMarker)) {
    throw new Error(`${label} page must use the canonical root favicon.`);
  }

  if ((pageHtml.match(/rel="icon"/g) ?? []).length !== 1) {
    throw new Error(`${label} page must expose exactly one canonical favicon.`);
  }
}

const verifyPage = await readFile(join(site, "verify", "index.html"), "utf8");
if (!verifyPage.includes("data-external-checksum-link")) {
  throw new Error("Verify page must own the checksum handoff.");
}

const supportPage = await readFile(join(site, "support", "index.html"), "utf8");
if (!supportPage.includes("issues/new")) {
  throw new Error("Support page must provide the beta issue escalation path.");
}

for (const token of [
  'const STORAGE_KEY = "smh-language"',
  '"Fonctionnalités"',
  '"Confidentialité & architecture"',
  '"Télécharger la bêta"',
  '"Gardez vos mods Sims 4 sains, organisés et"',
  "navigator.language",
  "localStorage.setItem"
]) {
  if (!i18n.includes(token)) {
    throw new Error(`Bilingual runtime is missing required marker: ${token}`);
  }
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

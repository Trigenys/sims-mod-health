import { mkdir, readFile, writeFile, copyFile } from "node:fs/promises";
import { gunzipSync } from "node:zlib";
import { join, resolve } from "node:path";
import {
  buildSitePages,
  renderFaviconLinks,
  renderProductHeader
} from "./site-pages.mjs";

const root = resolve(process.cwd());
const source = join(root, "apps", "landing", "stitch-live.html.gz.b64");
const outDir = join(root, "_site");

await mkdir(outDir, { recursive: true });

const encoded = (await readFile(source, "utf8")).trim();
let html = gunzipSync(Buffer.from(encoded, "base64")).toString("utf8");

// Normalize favicon handling on the root page as well as nested generated routes.
// Remove any Stitch-exported icon links first so the browser has one canonical,
// root-absolute product icon to resolve from every URL.
html = html.replace(
  /<link\b[^>]*\brel=["'][^"']*icon[^"']*["'][^>]*>\s*/gi,
  ""
);

if (!html.includes("<head>")) {
  throw new Error("Could not locate head tag for canonical favicon.");
}

html = html.replace(
  "<head>",
  `<head>\n  ${renderFaviconLinks()}`
);

html = html.replace(
  "Telemetry Off by Default Guarantee",
  "Telemetry Off by Default"
);

// The Stitch export uses Tailwind's default 6xl/7xl caps, which leaves
// too much dead space on 1440p/ultrawide displays. Keep readable text
// measures (4xl/2xl) intact, but let the product shells breathe.
html = html
  .replaceAll("max-w-7xl", "max-w-[1600px]")
  .replaceAll("max-w-6xl", "max-w-[1480px]");

const internalRoutes = new Map([
  ["https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi", "/download/"],
  ["https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/SHA256SUMS.txt", "/verify/"],
  ["https://github.com/Trigenys/sims-mod-health/releases/tag/v0.1.0-beta.1", "/release-notes/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/ARCHITECTURE.md", "/architecture/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/dbpf/READ_ONLY_PARSER.md", "/security/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/fingerprints/ARTIFACT_IDENTITIES.md", "/architecture/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/releases/WINDOWS_BETA_VALIDATION.md", "/release-notes/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/security/HARDENING_REVIEW.md", "/security/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/security/STAGED_UPDATE_ROLLBACK.md", "/security/"],
  ["https://github.com/Trigenys/sims-mod-health/blob/main/docs/security/THREAT_MODEL.md", "/security/"],
  ["https://github.com/Trigenys/sims-mod-health/tree/main/docs", "/docs/"],
  ["https://github.com/Trigenys/sims-mod-health", "/source/"]
]);

for (const [external, internal] of internalRoutes) {
  html = html.replaceAll(`href="${external}"`, `href="${internal}"`);
}

const headerPattern = /<header\b[\s\S]*?<\/header>/gi;
const headerMatches = html.match(headerPattern);

if (!headerMatches || headerMatches.length !== 1) {
  throw new Error(
    `Expected exactly one Stitch header, found ${headerMatches?.length ?? 0}.`
  );
}

// Use a replacement callback so '$' sequences inside generated HTML are never
// interpreted as String.replace substitution tokens.
html = html.replace(
  headerPattern,
  () => renderProductHeader("/", { logoSrc: "./logo.svg" })
);

if (!html.includes("</body>")) {
  throw new Error("Could not locate closing body tag for bilingual runtime.");
}

html = html.replace(
  "</body>",
  '<script src="./i18n.js" defer></script></body>'
);

await writeFile(join(outDir, "index.html"), html, "utf8");
await copyFile(join(root, "apps", "landing", "release.js"), join(outDir, "release.js"));
await copyFile(join(root, "apps", "landing", "i18n.js"), join(outDir, "i18n.js"));
await copyFile(join(root, "apps", "landing", "logo.svg"), join(outDir, "logo.svg"));
await buildSitePages(outDir);
await writeFile(join(outDir, ".nojekyll"), "", "utf8");

console.log("Built approved Stitch landing and professional first-party routes into _site.");

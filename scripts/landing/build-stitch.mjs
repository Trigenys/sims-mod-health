import { mkdir, readFile, writeFile, copyFile } from "node:fs/promises";
import { gunzipSync } from "node:zlib";
import { join, resolve } from "node:path";

const root = resolve(process.cwd());
const source = join(root, "apps", "landing", "stitch-live.html.gz.b64");
const outDir = join(root, "_site");

await mkdir(outDir, { recursive: true });

const encoded = (await readFile(source, "utf8")).trim();
let html = gunzipSync(Buffer.from(encoded, "base64")).toString("utf8");

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

const languageSwitcher = `
<div
  data-lang-switch
  class="inline-flex items-center rounded-full bg-surface-container-low p-1 border border-outline-variant/40"
  aria-label="Choose language"
>
  <button
    type="button"
    data-lang="en"
    aria-pressed="true"
    class="px-2.5 py-1.5 rounded-full font-label-md text-label-md font-bold transition-colors"
  >EN</button>
  <button
    type="button"
    data-lang="fr"
    aria-pressed="false"
    class="px-2.5 py-1.5 rounded-full font-label-md text-label-md font-bold transition-colors"
  >FR</button>
</div>
`.replace(/\n\s*/g, "");

const headerActions =
  '<div class="flex items-center gap-3 shrink-0"><a class="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-primary';

if (!html.includes(headerActions)) {
  throw new Error("Could not locate Stitch header action group for language switch.");
}

html = html.replace(
  headerActions,
  '<div class="flex items-center gap-3 shrink-0">' +
    languageSwitcher +
    '<a class="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-primary'
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
await writeFile(join(outDir, ".nojekyll"), "", "utf8");

console.log("Built approved Stitch landing into _site with wide desktop shells and EN/FR switching.");

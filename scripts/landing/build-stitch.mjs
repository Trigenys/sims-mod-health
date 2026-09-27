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

await writeFile(join(outDir, "index.html"), html, "utf8");
await copyFile(join(root, "apps", "landing", "release.js"), join(outDir, "release.js"));
await copyFile(join(root, "apps", "landing", "logo.svg"), join(outDir, "logo.svg"));
await writeFile(join(outDir, ".nojekyll"), "", "utf8");

console.log("Built approved Stitch landing into _site with wide desktop shells.");

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const APPFACTORY_RELEASE_SHA = "6a389276ffb89a94266029ec37d90e7b167cd95d";

const [releaseWorkflow, validationWorkflow, configText, manifestText] =
  await Promise.all([
    readFile(".github/workflows/release.yml", "utf8"),
    readFile(".github/workflows/beta-windows.yml", "utf8"),
    readFile("release-please-config.json", "utf8"),
    readFile(".release-please-manifest.json", "utf8"),
  ]);

const config = JSON.parse(configText);
const manifest = JSON.parse(manifestText);
const root = config.packages?.["."];

const manifestVersion = String(manifest["."] ?? "");
const betaMatch = manifestVersion.match(
  /^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)-beta\.(?<counter>\d+)$/
);

assert.ok(
  betaMatch?.groups,
  "Release manifest must remain on the numeric beta prerelease channel."
);

const betaCounter = Number(betaMatch.groups.counter);
assert.ok(
  Number.isSafeInteger(betaCounter) && betaCounter >= 1 && betaCounter <= 65534,
  "Release manifest beta counter must stay between 1 and 65534 for MSI packaging."
);

assert.equal(root?.["release-type"], "simple");
assert.equal(root?.versioning, "prerelease");
assert.equal(root?.prerelease, true);
assert.equal(root?.["prerelease-type"], "beta");

assert.ok(
  releaseWorkflow.includes(
    `EagleFox31/appfactory-project-automation/.github/workflows/release-tauri-desktop.yml@${APPFACTORY_RELEASE_SHA}`
  ),
  "Consumer release workflow must pin AppFactory to the reviewed immutable SHA."
);
assert.ok(releaseWorkflow.includes("product-name: Sims-Mod-Health"));
assert.ok(releaseWorkflow.includes("release-config-file: release-please-config.json"));
assert.ok(
  releaseWorkflow.includes(
    "release-manifest-file: .release-please-manifest.json"
  )
);
assert.ok(!releaseWorkflow.includes("@v1"), "Release execution must not use a mutable major alias.");

assert.ok(
  !validationWorkflow.includes("BETA_TAG:"),
  "PR installer validation must not own a hard-coded release tag."
);
assert.ok(
  !validationWorkflow.includes("publish-first-beta"),
  "PR installer validation must not publish GitHub Releases."
);
assert.ok(
  !validationWorkflow.includes("gh release create"),
  "PR installer validation must stay release-neutral."
);

console.log("Release automation contract is valid.");

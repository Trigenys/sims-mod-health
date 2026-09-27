const repository = "Trigenys/sims-mod-health";

const fallback = {
  tag: "v0.1.0-beta.1",
  releaseUrl:
    "https://github.com/Trigenys/sims-mod-health/releases/tag/v0.1.0-beta.1",
  downloadUrl:
    "https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi",
  checksumUrl:
    "https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/SHA256SUMS.txt",
  size: 6074368
};

const versionNodes = [...document.querySelectorAll("[data-release-version]")];
const sizeNodes = [...document.querySelectorAll("[data-download-size]")];
const binaryDownloadLinks = [...document.querySelectorAll("[data-binary-download-link]")];
const externalReleaseLinks = [...document.querySelectorAll("[data-external-release-link]")];
const externalChecksumLinks = [...document.querySelectorAll("[data-external-checksum-link]")];
const yearNodes = [...document.querySelectorAll("[data-year]")];

applyRelease(fallback);
for (const node of yearNodes) node.textContent = String(new Date().getFullYear());

const controller = new AbortController();
const timeout = window.setTimeout(() => controller.abort(), 4500);

fetch(`https://api.github.com/repos/${repository}/releases?per_page=10`, {
  headers: { Accept: "application/vnd.github+json" },
  signal: controller.signal
})
  .then((response) => {
    if (!response.ok) throw new Error(`GitHub release lookup returned ${response.status}`);
    return response.json();
  })
  .then((releases) => {
    const release = releases.find(
      (candidate) =>
        !candidate.draft &&
        Array.isArray(candidate.assets) &&
        candidate.assets.some((asset) =>
          asset.name.toLowerCase().endsWith(".msi")
        )
    );

    if (!release) return;

    const msi = release.assets.find((asset) =>
      asset.name.toLowerCase().endsWith(".msi")
    );
    const checksum = release.assets.find(
      (asset) => asset.name === "SHA256SUMS.txt"
    );

    applyRelease({
      tag: release.tag_name,
      releaseUrl: release.html_url,
      downloadUrl: msi.browser_download_url,
      checksumUrl: checksum?.browser_download_url ?? fallback.checksumUrl,
      size: msi.size
    });
  })
  .catch(() => {
    // The hard-coded Beta 1 fallback remains fully usable.
  })
  .finally(() => window.clearTimeout(timeout));

function applyRelease(release) {
  for (const node of versionNodes) node.textContent = release.tag;
  for (const node of sizeNodes) node.textContent = formatBytes(release.size);
  for (const link of binaryDownloadLinks) link.href = release.downloadUrl;
  for (const link of externalReleaseLinks) link.href = release.releaseUrl;
  for (const link of externalChecksumLinks) link.href = release.checksumUrl;
}

function formatBytes(bytes) {
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 10 ? 1 : 2)} MB`;
}

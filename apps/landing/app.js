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

const downloadLinks = [...document.querySelectorAll("[data-download-link]")];
const releaseLinks = [...document.querySelectorAll("[data-release-link]")];
const checksumLinks = [...document.querySelectorAll("[data-checksum-link]")];
const versionLabels = [...document.querySelectorAll("[data-version-label]")];
const downloadMeta = [...document.querySelectorAll("[data-download-meta]")];
const toast = document.querySelector("[data-release-toast]");
const year = document.querySelector("[data-year]");

if (year) {
  year.textContent = String(new Date().getFullYear());
}

applyRelease(fallback, false);

const controller = new AbortController();
const timeout = window.setTimeout(() => controller.abort(), 4500);

fetch(`https://api.github.com/repos/${repository}/releases?per_page=10`, {
  headers: {
    Accept: "application/vnd.github+json"
  },
  signal: controller.signal
})
  .then((response) => {
    if (!response.ok) {
      throw new Error(`GitHub release lookup returned ${response.status}`);
    }
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

    if (!release) {
      throw new Error("No published Windows MSI release found");
    }

    const msi = release.assets.find((asset) =>
      asset.name.toLowerCase().endsWith(".msi")
    );
    const checksum = release.assets.find(
      (asset) => asset.name === "SHA256SUMS.txt"
    );

    applyRelease(
      {
        tag: release.tag_name,
        releaseUrl: release.html_url,
        downloadUrl: msi.browser_download_url,
        checksumUrl: checksum?.browser_download_url ?? fallback.checksumUrl,
        size: msi.size
      },
      true
    );
  })
  .catch(() => {
    showToast("Using the bundled Beta 1 download link.");
  })
  .finally(() => window.clearTimeout(timeout));

function applyRelease(release, fromApi) {
  for (const link of downloadLinks) {
    link.href = release.downloadUrl;
  }
  for (const link of releaseLinks) {
    link.href = release.releaseUrl;
  }
  for (const link of checksumLinks) {
    link.href = release.checksumUrl;
  }
  for (const label of versionLabels) {
    label.textContent = release.tag;
  }
  for (const meta of downloadMeta) {
    meta.textContent = `${release.tag} · MSI · ${formatBytes(release.size)}`;
  }

  if (fromApi && release.tag !== fallback.tag) {
    showToast(`Latest Windows build: ${release.tag}`);
  }
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "Windows x64";
  }

  const megabytes = bytes / 1024 / 1024;
  return `${megabytes.toFixed(megabytes >= 10 ? 1 : 2)} MB`;
}

function showToast(message) {
  if (!toast) return;

  toast.textContent = message;
  toast.classList.add("is-visible");

  window.setTimeout(() => {
    toast.classList.remove("is-visible");
  }, 3200);
}

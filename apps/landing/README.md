# Sims Mod Health landing

Public acquisition/download surface for the Windows beta.

The live page now comes from the approved Stitch export supplied by the product owner. It is intentionally isolated from the Tauri/Vite desktop frontend.

## Source of truth

The approved Stitch HTML is stored as:

```text
apps/landing/stitch-live.html.gz.b64
```

It is compressed only to keep the imported one-file Stitch export manageable in Git. The build script reconstructs the exact page into `_site/index.html`.

Local product assets:

- `apps/landing/logo.svg` — Plumbob shield from the supplied export
- `apps/landing/release.js` — real release/download wiring

Build:

```bash
node scripts/landing/build-stitch.mjs
```

## Release wiring

The page contains a hard fallback to the current published beta:

```text
v0.1.0-beta.1
Sims-Mod-Health-0.1.0-beta.1-x64.msi
```

At runtime, `release.js` queries the public GitHub Releases API and selects the newest non-draft release that contains an MSI asset.

It updates:

- download CTA;
- version labels;
- installer file size;
- release-notes link;
- SHA-256 link.

If the API is unavailable or rate-limited, the Beta 1 links remain usable.

## Local preview

From the repository root:

```bash
node scripts/landing/build-stitch.mjs
python3 -m http.server 4173 --directory _site
```

Then open:

```text
http://localhost:4173
```

## Deployment

GitHub Pages is the production deployment.

```text
.github/workflows/landing-pages.yml
```

The workflow rebuilds the approved Stitch source and publishes `_site`.

Production custom domain:

```text
https://simsmodhealth.trigenys.com
```

## CI

`.github/workflows/landing.yml` validates that:

- the approved Stitch sections are present;
- stale mock release data is gone;
- the Beta 1 MSI fallback is real;
- latest-release MSI discovery remains wired;
- no analytics dependency is introduced;
- the built static site can be served successfully.

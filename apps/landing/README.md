# Sims Mod Health landing

Static public acquisition/download surface for the Windows beta.

It is intentionally isolated from the Tauri/Vite desktop frontend. No landing code is imported by the desktop application.

## Local preview

From the repository root:

```bash
python3 -m http.server 4173 --directory apps/landing
```

Then open:

```text
http://localhost:4173
```

## Release behavior

The HTML contains a direct fallback to the current published Beta 1 MSI.

At runtime, `app.js` queries the public GitHub releases API and selects the newest non-draft release containing an MSI asset. It updates:

- download CTA;
- version label;
- installer file size;
- release-notes link;
- checksum link.

If the GitHub API is rate-limited or unavailable, the Beta 1 fallback remains usable.

## Deployment

This folder is a standalone static Vercel project.

Recommended project settings:

- Root Directory: `apps/landing`
- Framework Preset: Other
- Build Command: none
- Output Directory: `.`

`vercel.json` adds the baseline static response headers.

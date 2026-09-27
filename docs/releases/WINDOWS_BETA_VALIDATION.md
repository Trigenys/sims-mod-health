# Windows Beta 1 validation

Release candidate: `v0.1.0-beta.1`

## CI source of truth

Workflow:

```text
.github/workflows/beta-windows.yml
```

The workflow runs on a clean GitHub-hosted Windows environment for pull requests that affect the beta package and again on `main`.

Publication is allowed only after the Windows `build-validate` job succeeds.

## Validation sequence

The Windows job performs these steps against one source commit:

1. check out the commit;
2. install frontend dependencies;
3. build the Tauri MSI;
4. locate the generated MSI;
5. verify that Sims Mod Health is not already registered on the clean runner;
6. install the MSI silently with `msiexec`;
7. verify the Windows uninstall registration exists;
8. uninstall the same MSI silently;
9. verify the registration has been removed;
10. copy that same validated MSI into the publication directory;
11. compute SHA-256;
12. write build provenance containing commit SHA, run ID and run attempt;
13. upload the installer/checksum/provenance as a workflow artifact.

On the first successful `main` run where the beta release does not already exist, a second job creates the prerelease `v0.1.0-beta.1` from the exact validated commit and uploads the validated files.

The first-beta tag/release is treated as immutable by the workflow. Later `main` runs do not replace its assets.

## Checksum

The release contains:

```text
SHA256SUMS.txt
```

The checksum is calculated after validation from the exact MSI copied to the release artifact directory.

## Upgrade verification

`v0.1.0-beta.1` has no previous supported public build.

Result: **N/A — first supported beta**.

This exception does not carry forward to the next supported release.

## Privacy release review

Verified release default:

- diagnostic telemetry disabled until explicit opt-in;
- malformed local consent value fails closed;
- no automatic raw diagnostic upload;
- local path/user identifiers redacted from diagnostic telemetry preview;
- WebView capability remains `core:default` only.

## Security release review

The package is not eligible for publication unless the hardening work from #21 is on the source branch.

The repository Security CI separately verifies parser abuse cases, dependency advisories, CSP and Tauri capability drift.

## Manual user-facing checks after publication

The first published prerelease should be downloaded once from the GitHub Release page and its checksum compared with `SHA256SUMS.txt`.

Because the installer is unsigned, a Windows publisher/SmartScreen warning is a documented Beta 1 limitation rather than a checksum exception.

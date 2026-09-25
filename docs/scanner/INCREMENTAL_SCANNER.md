# Incremental Mods scanner

## Purpose

The scanner builds the device-local inventory that later parser, matching and health-engine issues consume.

It is deliberately limited to `.package` and `.ts4script` files in this phase. File contents never leave the desktop as part of inventory.

## Modes

### Incremental

The fast path compares:

```text
installation + relative path + file size + modification timestamp
```

with the previous local record.

If the metadata matches **and** a SHA-256 fingerprint already exists, the file is not opened or re-hashed.

If metadata changed or the SHA-256 is missing, the file is streamed through SHA-256 again.

### Full

The full-verification mode ignores the metadata cache and re-hashes every supported file.

This is intentionally more expensive and exists for explicit integrity verification.

## Deterministic inventory

The scanner:

1. recursively enumerates the Mods tree without following symlinks;
2. keeps only `.package` and `.ts4script`;
3. normalizes relative separators to `/`;
4. sorts relative paths before persistence;
5. removes local inventory rows for files absent from a successfully completed scan.

An interrupted/cancelled scan never performs that final removal.

## Transaction and cancellation model

A scan session row is committed first.

All inventory mutations for the scan then happen inside a separate SQLite transaction.

If cancellation is requested:

- hashing checks the cancellation flag between read chunks;
- the inventory transaction is rolled back;
- the prior completed inventory remains intact;
- the scan-session row is marked `cancelled`;
- partial file records are not committed.

Only one scan may run at a time.

## Progress

The native layer emits:

```text
scanner://progress
```

with:

- `filesSeen`;
- `filesHashed`;
- `filesSkipped`;
- `observations`.

Progress is throttled to the first file, every 50 files and final completion/cancellation.

## Recoverable observations

Filesystem problems do not automatically abort the entire inventory.

Examples:

- `permissionDenied`;
- `disappeared` when a file vanishes between enumeration and metadata/read;
- `ioError`;
- `nestedModsDirectory`;
- `invalidDepth`.

Observations are stored in `scan_observations` and tied to the scan session.

## Folder-depth rules

The scanner currently observes these placement rules:

- `.ts4script`: at most one subfolder below Mods;
- package-only content: at most five subfolders below Mods.

These limits follow EA's current Mods-folder guidance. The observation is a layout warning; it is not a claim that the mod itself is broken.

Source: EA Forums, “How to Use Mods and CC”, section “Install in the Right Place in Mods: Script Files and Your Folder Structure”.

## Hashing

SHA-256 is streamed with a 64 KiB buffer.

The scanner stores the exact SHA-256 as local technical identity. Later fingerprint/matching issues may add source-specific fingerprints and resource signatures.

## Privacy

The scanner has no network dependency.

It persists:

- installation-local relative paths;
- file size/mtime;
- technical fingerprints;
- scan/session observations.

Absolute roots remain in the local `installations` table only.

No inventory payload is sent to a registry in this issue.

## Benchmark

A dedicated ignored Rust benchmark fixture creates **5,000 package files**, runs:

1. full scan;
2. unchanged incremental scan.

The benchmark is executed by the scanner benchmark workflow for scanner changes. Its emitted line records full/incremental elapsed milliseconds and hash/skip counts.

The acceptance invariant is:

- full scan hashes all 5,000 files;
- unchanged incremental scan hashes 0;
- unchanged incremental scan skips all 5,000.

Wall-clock time is recorded as evidence rather than used as a brittle hard CI threshold.

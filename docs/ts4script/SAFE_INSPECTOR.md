# Safe TS4Script archive inspector

## Purpose

A `.ts4script` file is treated as an **untrusted ZIP archive containing inert data**.

The inspector extracts only the metadata needed for later identity and diagnostics work:

- archive entry names;
- compressed and expanded sizes;
- CRC32;
- Python module names inferred from `.py` / `.pyc` paths;
- Python package names inferred from `__init__.py[c]`;
- bounded version hints from small text metadata files.

It never imports, executes or evaluates embedded Python.

## ZIP implementation

The native layer uses the Rust `zip` crate with only normal Stored/Deflate reading enabled for our dependency.

Before constructing `ZipArchive`, the inspector reads only the bounded ZIP end-of-central-directory search window and checks the declared entry count. This enforces the 4,096-entry product limit **before** the ZIP library allocates its central-directory collection.

After that preflight, the inspector relies on central-directory metadata and opens archive entries only when a small text metadata file qualifies for bounded reading.

No archive entry is ever extracted to disk.

## Security limits

| Limit | Value |
|---|---:|
| Physical TS4Script archive | 256 MiB |
| Archive entries | 4,096 |
| Single expanded entry | 64 MiB |
| Total declared expanded data | 512 MiB |
| Maximum compression ratio | 500:1 |
| Path depth | 32 components |
| Entry name | 1,024 bytes |
| Text metadata entry read | 64 KiB |
| Total text metadata reads | 256 KiB |
| Version hints retained | 16 |

An archive exceeding a hard archive/entry/expanded-data/path limit fails inspection explicitly.

The metadata-read budget is softer: once exhausted, the archive remains inspectable but further version-hint reads stop and `metadataBudgetExhausted` is set.

## Path safety

Entry paths are rejected when they contain or represent:

- parent traversal (`..`);
- POSIX absolute roots;
- Windows UNC/rooted paths;
- Windows drive prefixes such as `C:\`;
- NUL bytes;
- more than 32 components.

The inspector performs its own separator-neutral validation because production runs on Windows but tests and analysis should not depend on platform-specific path parsing.

Symlink entries are rejected.

## Archive-bomb strategy

The inspector does not trust compressed size alone.

Before reading any entry body it validates:

- declared expanded size;
- compressed-to-expanded ratio;
- cumulative expanded-data budget.

This means a tiny compressed entry claiming an extreme expanded size is rejected even if the inspector would not otherwise need to read its body.

Nested archives are **not recursively opened**. An embedded `.zip` or `.ts4script` is merely another inert entry.

## Module identity

Examples:

```text
creator/mod/core.pyc
→ creator.mod.core

creator/mod/__init__.pyc
→ creator.mod

creator/mod/__pycache__/feature.cpython-37.pyc
→ creator.mod.feature
```

Module identity is path-derived. The inspector does not import Python and does not inspect bytecode semantics.

## Version hints

Only small textual metadata extensions are considered:

- `.json`
- `.txt`
- `.ini`
- `.cfg`
- `.toml`
- `.yaml`
- `.yml`

The inspector looks for bounded lines containing the word `version` and retains semver-like tokens as **hints**, not authoritative release identity.

Python source and bytecode bodies are not read for version discovery.

## Failure model

Typed failures include:

- malformed ZIP;
- physical archive too large;
- too many entries;
- unsafe path;
- excessive path depth/name length;
- symlink or encrypted entry;
- expanded entry too large;
- extreme compression ratio;
- cumulative expanded-data budget exceeded;
- I/O failure.

A malformed archive produces an error; it does not write to the archive, extract files or mutate SQLite scan state.

## Verification

Native tests cover:

- stable module/package extraction;
- `__pycache__` normalization;
- traversal / absolute / drive-path rejection;
- path-depth limit;
- preflight entry-count enforcement before central-directory allocation;
- archive-bomb size, cumulative expanded-data and ratio limits;
- bounded metadata reads;
- malformed archive immutability;
- read-only archive inspection;
- embedded Python source that would throw if executed;
- 1,024 deterministic ZIP mutations/truncations with a no-panic invariant.

## Downstream contract

Issue #9 may use this metadata for fingerprinting and canonical matching.

Diagnostics work may later correlate module names with stack traces.

This inspector does not decide compatibility, execute scripts or update files.

# Artifact fingerprints and exact duplicates

## Purpose

Sims Mod Health uses more than one technical identity because different questions require different evidence.

| Fingerprint | Meaning | Exact content identity? |
|---|---|---|
| SHA-256 | exact bytes of one local file | Yes |
| DBPF resource signature | normalized set of live Type/Group/Instance keys | No |
| TS4Script identity signature | normalized Python module/package names | No |
| Source-specific fingerprint | adapter-defined identity such as a future CurseForge fingerprint | Depends on source |

Only SHA-256 drives the **Exact duplicate** label.

A resource or script signature can help canonical matching later, but it must never be presented as proof that two files are byte-for-byte duplicates.

## SHA-256

The scanner computes SHA-256 with a 64 KiB streaming buffer.

The full file is never loaded into memory.

The scanner cache remains:

```text
relative path + size + modification timestamp + current SHA algorithm version
```

If any file-identity input changes, locally derived fingerprints are invalidated before a new SHA-256 is stored.

## Pluggable fingerprint providers

Structural/source-aware fingerprints implement the Rust `FingerprintProvider` interface:

```text
kind()
algorithm_version()
compute(path)
```

Current implementations:

- `DbpfResourceSignatureProvider`
- `Ts4ScriptIdentityProvider`

A future source adapter can implement the same contract without changing duplicate semantics.

## DBPF resource signature

The resource signature:

1. parses the package through the bounded read-only DBPF parser;
2. keeps live resource keys only;
3. sorts Type/Group/Instance keys;
4. removes duplicate keys;
5. hashes a versioned domain separator, key count and canonical binary keys with SHA-256.

The result is independent of DBPF index ordering.

Algorithm version:

```text
dbpf-resource-keys-v1
```

## TS4Script identity signature

The script signature:

1. inspects the archive without executing Python;
2. takes normalized Python module names;
3. takes normalized Python package names;
4. sorts/deduplicates them;
5. hashes length-delimited values under a versioned domain separator.

Version hints and bytecode contents are deliberately excluded. This signature is intended to help identify a script mod family across releases, not to prove exact equality.

Algorithm version:

```text
ts4script-python-identity-v1
```

## Persistence

Fingerprints are stored against `local_file_id` with:

- kind;
- value;
- algorithm version;
- computation timestamp.

Migration v4 adds `script_signature` as a persisted fingerprint kind.

When a scanner cache miss indicates changed file identity, the scanner removes stale:

- SHA-256;
- resource signature;
- script signature;
- quick fingerprint rows.

Source-owned fingerprints such as a future CurseForge identity are not blindly removed by this local cache invalidation helper.

## Exact duplicate engine

Exact duplicate groups are produced locally by grouping current-version SHA-256 values **within one Sims installation**.

Consequences:

- same bytes in different folders → exact duplicate;
- same filename with different bytes → not a duplicate;
- same DBPF resource signature with different bytes → not automatically a duplicate;
- duplicate detection requires no network access.

Group members are returned in deterministic relative-path order.

## Performance evidence

The dedicated benchmark workflow runs:

- the existing 5,000-file full + incremental scanner benchmark;
- a 5,000-row duplicate-grouping benchmark with 100 exact duplicate groups.

This measures both the expensive streamed hashing path and the indexed local grouping query.

## Downstream use

- #13 may use structural/source fingerprints in artifact-to-mod resolution.
- #15 may compare DBPF resource keys for **Potential conflict** evidence.
- UI work may expose SHA-backed Exact duplicates separately from conflicts.

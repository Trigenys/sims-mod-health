# Read-only Sims 4 DBPF parser

## Purpose

The DBPF parser extracts the minimum package metadata needed by later fingerprint and conflict work:

- DBPF file version;
- resource count;
- Type / Group / Instance keys;
- resource byte offset;
- stored size;
- decompressed size;
- optional compression metadata;
- deleted/tombstone state.

It does **not** decompress or parse resource payloads in this issue.

## Format contract

The parser implements the Sims 4 DBPF 2.x layout used by established community readers:

- 96-byte DBPF header;
- magic `DBPF`;
- resource count at header offset 36;
- low index offset at 40;
- index size at 44;
- high/alternate index offset at 64;
- an index-leading bitmask where bits `0x01`, `0x02` and `0x04` make Type, Group and Instance-high constant across all entries;
- per-entry Instance-low, resource offset, stored-size field and decompressed size;
- the high bit of the stored-size field indicates an additional 4-byte compression pair.

The Instance key is assembled as:

```text
(instanceHigh << 32) | instanceLow
```

### Reference implementations reviewed

- s4py DBPF reader/writer: https://github.com/thequux/s4py/blob/master/lib/s4py/package/dbpf.py
- FiveOS Sims DBPF reader: https://github.com/w3bportal/FiveOS/blob/main/src/Services/Sims/DbpfPackage.cs
- Sims4Reader DBPF reader: https://github.com/IdentityGames/sbox-ts4m/blob/main/Sims4Reader/Core/DbpfPackage.cs

These references agree on the 96-byte header, the constant-field index mask and the Type/Group/64-bit Instance resource identity.

## Security limits

The parser treats every package as hostile input.

Hard limits:

- maximum resource entries: **1,000,000**;
- maximum declared index size: **128 MiB**;
- fixed-size header read: **96 bytes**;
- index reads: only 2- or 4-byte scalar fields;
- resource payload bytes read: **0** in this issue.

The parser validates with checked arithmetic:

- index offset + index size;
- minimum bytes required for declared entry count;
- each non-deleted resource offset + stored size.

Unknown index-layout flag bits are rejected rather than guessed.

## Memory behavior

The package itself is never loaded into memory.

The index is parsed directly from a seekable read-only stream. Memory use is therefore dominated by the resulting `Vec<ResourceEntry>`, bounded by the maximum resource count.

No resource payload allocation or decompression occurs.

## Read-only guarantee

`parse_path()` uses `std::fs::File::open`, which opens the package for reading.

A native test marks a package fixture read-only at filesystem level, parses it successfully and then verifies its bytes are unchanged.

## Deleted entries

Entries with offset `0xFFFFFFFF`, or the known tombstone shape `stored_size = 1` with `decompressed_size = 0xFFFFFFFF`, are retained as metadata but excluded from `resource_keys()`.

They are not range-validated as live payloads.

## Failure model

Malformed packages return typed errors for:

- file too small;
- bad magic;
- unsupported major DBPF version;
- resource count above limit;
- index above size limit;
- missing index;
- unsupported index flags;
- checked-arithmetic overflow;
- out-of-range index;
- truncated index;
- out-of-range resource payload span;
- underlying I/O failure.

No malformed input should panic the desktop process.

## Adversarial harness

The normal Rust test suite includes a deterministic mutation harness.

It starts from a structurally valid DBPF fixture and performs 2,048 reproducible mutations/truncations. Every parse attempt is wrapped by `catch_unwind`; the acceptance invariant is that malformed data may return an error but must never panic.

This is the issue's **equivalent adversarial harness** under the Proof of Done requirement. A libFuzzer/cargo-fuzz target can be added later if parser complexity expands into payload decompression.

## Downstream contract

The parser intentionally returns enough information for:

- #9 resource-signature generation;
- later resource-overlap / potential-conflict analysis;
- type-based classification.

It does not decide whether overlapping resources are actually incompatible. That semantic decision remains outside the parser.

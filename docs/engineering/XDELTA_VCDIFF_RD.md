# Xdelta / VCDIFF R&D gate

Status: **technical prototype GO; production direct-patching NO-GO until payload-source approval**

Related issue: #77

## Why this exists

Sims Mod Health already has a conservative transactional mutation pipeline for mod artifacts: verified restore point, app-controlled staging, SHA-256 verification, persistent journal and rollback.

Issue #77 asks a narrower question: can we reuse a mature VCDIFF implementation as a delta primitive without turning Sims Mod Health into a game/DLC downloader?

The answer from this R&D slice is:

- **GO** for a staged technical adapter and deterministic validation;
- **NO-GO** for production Game/DLC direct patching until an independent review proves that the payload source is authorized, provenance-verifiable and integrity-verifiable.

The default production path remains official-provider handoff.

## Upstream evaluated

Repository: `jmacd/xdelta`

Reviewed on: 2026-09-28

Reviewed line: **Xdelta 3.2.x / v3.2.0**

Relevant upstream facts:

- Xdelta 3 implements VCDIFF / RFC 3284.
- The current upstream README identifies 3.2.x as the active release series.
- Upstream provides a reusable `xdelta3lib` C library and prebuilt Windows binaries.
- The repository README states that `main` and the `release3_2_apl` 3.2.x series continue under Apache License 2.0.
- `xdelta3/LICENSE` contains the Apache License 2.0 text.
- GitHub repository metadata currently reports no detected root license because the license file is under `xdelta3/`; therefore license approval must rely on the upstream README + `xdelta3/LICENSE`, not only the GitHub metadata badge.
- The original GPL line is maintained separately under `jmacd/xdelta-gpl` and is not the dependency line evaluated here.

### Pinned Windows R&D artifact

The R&D workflow downloads:

```text
xdelta3-3.2.0-windows-x86_64.zip
```

from the official `jmacd/xdelta` GitHub Release and requires SHA-256:

```text
af8ef036cb077a48df080c9a8ac1be4a6e7511c32d11f8bec89b6803a9e52576
```

The archive is verified before extraction or execution.

## Adapter boundary

The Rust core now defines a narrow `DeltaApplier` port with an `Xdelta3CliApplier` R&D adapter.

The adapter deliberately does **not**:

- download a delta;
- select a Game/DLC source;
- infer ownership or entitlement;
- write directly into the game installation;
- expose a Tauri command;
- register any production updater path.

Its only responsibility is to transform:

```text
verified source + verified delta -> staged output
```

under caller-supplied staging.

## Integrity contract

Three SHA-256 values are mandatory:

1. expected source SHA-256;
2. expected delta SHA-256;
3. expected resulting target SHA-256.

Execution order:

```text
validate regular source/delta files
        ↓
verify source SHA-256
        ↓
verify delta SHA-256
        ↓
run pinned xdelta3 decoder
        ↓
verify staged target SHA-256
        ↓
return staged result
```

On an execution failure or target-hash mismatch, any partial staged output is removed.

The adapter refuses to overwrite an existing output. This keeps the R&D primitive compatible with the existing mutation rule: construct and verify outside the real target directory first.

## Test evidence

Normal unit tests prove that:

- a wrong source hash fails before xdelta3 can execute;
- a wrong delta hash fails before xdelta3 can execute;
- an existing staged output is never overwritten.

The dedicated Windows R&D workflow additionally uses the pinned xdelta3 3.2.0 binary to prove:

- deterministic source/target fixture round-trip;
- target-hash mismatch removes the reconstructed output;
- an 8 MiB synthetic fixture can be encoded and decoded while printing source, target and delta sizes plus encode/apply duration.

No copyrighted Sims payload is used. Fixtures are generated synthetic bytes owned by this repository.

## Failure-model status

### Corruption

**Covered for source, delta and target integrity.**

A changed source or delta fails before decode. A reconstructed target with the wrong expected hash is deleted before it can be consumed.

### Interruption

**Partially covered by architecture, not yet a production claim.**

The adapter writes only to a staging path and does not mutate the game directory. The existing Sims Mod Health mutation journal/restore-point model remains the intended transaction owner if this primitive is ever promoted.

A future production integration must explicitly journal a delta-apply state and prove startup recovery around an interrupted external process before direct patching can be enabled.

### Low disk

**Not yet proven on a real constrained filesystem.**

The current adapter fails closed on I/O/process failure and leaves the original source untouched, but issue #77 is not complete until a reproducible low-disk test demonstrates cleanup behavior.

## Packaging options

Two production packaging routes remain possible:

### Option A — Tauri sidecar

Bundle the reviewed official xdelta3 executable as a signed/hashed sidecar.

Pros:
- smallest integration change;
- isolates C code from Rust unsafe FFI;
- CLI behavior matches upstream release testing.

Costs:
- one more executable to package, sign, inventory and update;
- Apache 2.0 notice/license distribution obligations must be preserved;
- sidecar architecture/asset mapping is required per Windows target.

### Option B — static xdelta3lib integration

Build the Apache-2.0 `xdelta3lib` through CMake and expose a narrow Rust FFI wrapper.

Pros:
- single process;
- direct streaming/control opportunities.

Costs:
- C toolchain/build integration becomes part of the Rust/Tauri supply chain;
- unsafe FFI surface requires dedicated tests and ABI/version discipline;
- more maintenance than the sidecar route.

For the R&D phase, **Option A is intentionally used**. No production packaging decision is made yet.

## Go / no-go decision

### GO

Continue using the narrow staged adapter for technical experiments because:

- the evaluated 3.2.x upstream line has explicit Apache-2.0 licensing evidence;
- the primitive can be isolated behind `DeltaApplier`;
- source, delta and target integrity can all be required independently;
- the adapter does not need to weaken the existing restore-point/staging model.

### NO-GO

Do **not** enable direct Game/DLC patching in production yet.

Promotion remains blocked until all of the following exist:

- a payload source with documented authorization to distribute/use;
- retained provenance for each delta;
- expected source/delta/target hashes from trusted metadata;
- reproducible interruption recovery evidence;
- reproducible low-disk failure evidence;
- Windows packaging/signing/notice plan;
- explicit integration into the existing transaction journal and restore-point lifecycle.

Until then, #75 official-provider handoff remains the production update strategy.

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

## Reuse decision

### Selected for the prototype

Repository: `liushuyu/xdelta3-rs`

Pinned commit:

```text
7bca8bc72548bd88a92a308dcd01225c3e842bc7
```

Why this repository:

- Rust-facing API for Xdelta/VCDIFF;
- Apache-2.0 repository license;
- its Xdelta submodule is pinned to upstream commit `0525275fe4b553a10f38e455d30c60dc6ed9b45d`;
- that upstream commit is the original-author Xdelta **3.0.12 APL** relicensing commit;
- it keeps native-size detection in its build script instead of hard-coding C type sizes;
- `default-features = false` avoids bringing the optional streaming stack into this R&D adapter.

The dependency is pinned by immutable Git SHA. We do not track a moving branch.

### Candidate rejected after code review

`sigp/xdelta3-rs` was initially attractive because it has newer dependency maintenance and a richer error API. We did **not** keep it as the prototype dependency because its build script deliberately replaces native C-size detection with 64-bit constants for `size_t`, `unsigned int`, `unsigned long` and `unsigned long long`.

That simplification is not a portability assumption we want to inherit for a Windows desktop product. The original binding's runtime compiler probe is slower but preserves the target's actual ABI sizes.

### Other candidates

- `ThinkingJoules/vcdiff-utils`: promising pure-Rust VCDIFF implementation, but no explicit repository license was found during this review, so code reuse is blocked.
- `Speedy37/vcdiff-rs`: MIT and pure Rust, but old (`nom 3`), inactive for years and missing support for compressed delta sections.
- `Speedy37/speedupdate-rs`: useful prior art for update graphs, integrity and recovery, but much broader than the narrow delta primitive required here.
- `jmacd/xdelta` 3.2.x: authoritative modern upstream and still the preferred reference for a future production-grade native integration. Its current `xdelta3lib` may replace the 3.0.12 binding later if we need streaming, larger files or tighter ABI control.

## Adapter boundary

The Rust core defines a narrow `DeltaApplier` port with an `Xdelta3RustApplier` prototype.

The adapter deliberately does **not**:

- download a delta;
- select a Game/DLC source;
- infer ownership or entitlement;
- write directly into the game installation;
- expose a Tauri command;
- register any production updater path.

Its only responsibility is:

```text
verified source + verified delta
        ↓
VCDIFF decode through reused Rust binding
        ↓
verified staged output
```

No standalone xdelta executable or sidecar is downloaded by CI or shipped by this prototype.

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
decode in memory through xdelta3-rs
        ↓
write only to caller-provided staging
        ↓
verify staged target SHA-256
        ↓
return staged result
```

An existing output is never overwritten. A reconstructed target with the wrong expected hash is removed.

## Deliberate R&D limits

The selected binding exposes a memory-oriented API that uses 32-bit lengths internally. Sims Mod Health therefore places an additional **512 MiB per-input R&D limit** and rejects unsafe aggregate lengths before entering the binding.

This is a prototype constraint, not a production Game/DLC limit.

If direct patching is ever promoted, the production implementation must be reevaluated against modern `jmacd/xdelta` 3.2.x / `xdelta3lib` or another implementation that supports the required streaming and large-file behavior.

## Test evidence

The normal native Rust test suite now covers the VCDIFF adapter directly; there is no bespoke xdelta workflow.

Fixtures prove:

- wrong source hash fails before decode;
- wrong delta hash fails before decode;
- existing staging output is not overwritten;
- a deterministic source/target fixture round-trips through the reused binding;
- a corrupted delta fails closed;
- wrong target hash removes the staged result;
- an 8 MiB synthetic fixture is encoded/decoded while printing source, target and delta sizes plus encode/apply duration.

No copyrighted Sims payload is committed or used by these tests.

The existing Impact-Aware native surface is sufficient:

```text
src-tauri/**
    → Rust CI
    → Security
    → Windows installer validation
```

A separate `xdelta-rd` CI gate was removed because reusing the Rust dependency makes it redundant.

## Failure-model status

### Corruption

**Covered for source, delta and target integrity.**

A changed source or delta fails before decode. A corrupted VCDIFF stream either fails decode or produces a target that is rejected by the expected target SHA-256.

### Interruption

**Reduced, but not yet a production claim.**

Removing the external `xdelta3.exe` process eliminates one process-lifecycle failure mode. The adapter still writes only to staging and never mutates the game directory.

A future production integration must explicitly journal the delta-apply state and prove startup recovery around an interrupted staging write before direct patching can be enabled.

### Low disk

**Not yet proven on a constrained filesystem.**

A staging write error leaves the source untouched and the adapter removes failed output where possible, but #77 remains open until a reproducible low-disk test demonstrates cleanup behavior.

## Packaging impact

The prototype no longer needs a Tauri xdelta sidecar.

The trade-off moves into the Rust build:

- C compiler + bindgen/libclang are required to build the dependency;
- Xdelta 3.0.12 APL is compiled into the application through the Rust binding;
- Apache-2.0 notices must be retained in product dependency notices;
- the Windows installer validation remains responsible for proving the binding can compile in the actual desktop packaging environment.

This is simpler than shipping and signing a second executable, but it still introduces native C code and therefore remains an R&D dependency until the production gate is passed.

## Go / no-go decision

### GO

Continue the isolated prototype because:

- an explicitly licensed reusable implementation exists;
- the dependency is immutable-pinned;
- the adapter stays behind `DeltaApplier`;
- source, delta and target integrity remain independently enforced;
- no downloader or entitlement behavior is introduced;
- normal Rust/Windows CI can validate the dependency without bespoke workflow plumbing.

### NO-GO

Do **not** enable direct Game/DLC patching in production yet.

Promotion remains blocked until all of the following exist:

- a payload source with documented authorization to distribute/use;
- retained provenance for each delta;
- expected source/delta/target hashes from trusted metadata;
- reproducible interruption recovery evidence;
- reproducible low-disk failure evidence;
- dependency notice/package review;
- explicit integration into the existing transaction journal and restore-point lifecycle;
- a large-file/streaming decision suitable for real Game/DLC payload sizes.

Until then, #75 official-provider handoff remains the production update strategy.

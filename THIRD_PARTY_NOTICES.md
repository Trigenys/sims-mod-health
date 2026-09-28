# Third-party notices

This file records third-party components that require explicit review for Sims Mod Health desktop packaging.

## xdelta3 Rust binding

- Component: `radu-cendars/xdelta3-rs`
- Repository: https://github.com/radu-cendars/xdelta3-rs
- Pinned commit: `bd20199837ba28cd91e6754e239c0dbe221bc8be`
- Declared license: Apache License 2.0
- Purpose: Rust binding used by the isolated VCDIFF R&D adapter in issue #77.
- Cargo features: `default-features = false`

The dependency remains pinned by immutable Git commit rather than a moving branch.

## Xdelta 3

The binding's `xdelta3` submodule points to:

```text
jmacd/xdelta
0525275fe4b553a10f38e455d30c60dc6ed9b45d
```

That upstream commit is titled:

```text
Introduce APL LICENSE and notices, remove GNU COPYING and notices. Version 3.0.12. (Copyright owner)
```

The Xdelta source at that commit carries the Apache License 2.0 notice.

## Packaging rule

If the R&D adapter is ever promoted into a distributed production build, the release process must retain the applicable Apache-2.0 license and notice material and re-review the exact pinned dependency revision.

This notice does not authorize any Game/DLC payload source. Payload authorization, provenance and integrity remain a separate hard gate.

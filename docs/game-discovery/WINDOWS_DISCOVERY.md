# Windows Sims 4 discovery

## Goal

Find the user's The Sims 4 **user-data installation** without hard-coding a username, assuming a fixed Documents path or guessing a game version.

The relevant user-data root is normally:

```text
<Documents>/Electronic Arts/The Sims 4
```

This is intentionally distinct from the EA/Steam program installation directory.

## Discovery order

### 1. Windows Documents known folder

The primary candidate comes from the operating system's Documents location through the Rust `dirs` crate.

On Windows, `dirs` uses the Windows Known Folder API. This means a Documents folder redirected by Windows/OneDrive can be resolved without constructing `C:\Users\<name>\Documents` ourselves.

### 2. OneDrive fallback variables

For defensive coverage, the native layer also considers:

- `OneDrive`
- `OneDriveConsumer`
- `OneDriveCommercial`

and appends `Documents/Electronic Arts/The Sims 4`.

Duplicate resolved roots are removed before inspection.

### 3. Manual override

The native command accepts a manually supplied path.

It accepts either:

- the exact `The Sims 4` user-data root; or
- a Documents-like parent containing `Electronic Arts/The Sims 4`.

The path is validated entirely in Rust. The frontend does not receive broad filesystem permission.

## Version parsing

`GameVersion.txt` supports both forms:

```text
GameVersion = 1.128.90.1030
```

and:

```text
1.128.90.1030
```

A UTF-8 BOM and surrounding whitespace are tolerated.

A valid version must contain **exactly four numeric components**. The result is normalized by numeric parsing, so leading zeroes do not create a different version identity.

No fallback/guess is produced for:

- missing file;
- empty file;
- too few/many components;
- non-numeric components;
- unreadable file.

Instead the API returns an explicit `missing` or `invalid` version state.

## Mods folder

The candidate exposes `modsAvailable` separately from version state.

This allows these states to remain distinguishable:

- valid Sims root + Mods present + valid version;
- valid Sims root + Mods missing;
- valid Sims root + version missing;
- valid Sims root + version malformed.

## Privacy

Absolute paths exist only inside the desktop trust boundary and may be returned to the local WebView for explicit user interaction.

They are **not part of the registry/telemetry contract** and must be redacted before any future remote diagnostics or analytics event.

## Native commands

```text
discover_sims_installations()
inspect_sims_installation(path)
```

Both commands perform filesystem access inside Rust.

## Verification

Native tests cover:

- labeled version parsing;
- raw version parsing;
- BOM and leading-zero normalization;
- malformed version rejection;
- missing version file;
- common Documents layout discovery;
- redirected-root deduplication;
- manual direct-root override;
- manual Documents-parent override;
- nonexistent manual path.


## Program installation and content-pack inventory

Issue #73 adds a second discovery boundary for the **program installation**. This remains separate from the user-data root above.

### Provider strategies

The native core uses provider probes rather than path guessing in the WebView:

1. **Steam manifest** — inspect Steam libraries for app manifest `1222670` and resolve its `installdir`.
2. **EA/Maxis registry** — on Windows, read `Install Dir` from the Maxis Sims 4 registry keys in HKLM/HKCU and both registry views.
3. **EA default paths** — probe existing `EA Games/The Sims 4` and legacy `Origin Games/The Sims 4` directories.
4. **Manual path** — accept an explicit program-installation directory when automatic discovery is unavailable.

A directory is only accepted as a game installation when program markers exist under `Game/Bin` and `Data/Client`.

Steam wins provider classification when the same install root is also visible through EA/Maxis registry state. The provider describes the update/install source; it does not represent account ownership.

### Program-build evidence

The first version source is:

```text
<Game install>/Game/Bin/Default.ini
```

The `gameversion` value must contain exactly four numeric components.

The native layer also computes SHA-256 for available sentinel files:

```text
Game/Bin/Default.ini
Game/Bin/TS4_x64.exe
Delta/EP01/Version.ini
```

When the version text cannot be resolved, sentinel hashes are retained for later manifest matching but **no version is guessed**. Until #74 provides a provenance-bearing fingerprint manifest, that state remains `Unknown`.

### Content-pack observations

Direct program-installation directories matching the conservative code families below are inventoried:

- `EPnn` — Expansion
- `GPnn` — Game
- `SPnn` — Stuff or Kit
- `FPnn` — Free
- `KITnn` / `KITnnn` — reserved explicit Kit form when present

For each locally present pack the inventory stores:

- canonical upper-case code;
- local state: `installed`, `partial` or `unknown`;
- bounded recursive byte size;
- regular-file marker count;
- observation timestamp.

An empty pack directory is `partial`. An unreadable or over-limit directory is `unknown`. Filesystem presence never becomes an entitlement/ownership state.

### Persistence

Local observations are stored only in SQLite tables:

- `game_content_installations`
- `installed_packs`

Raw program paths remain in the local desktop trust domain. No game path or pack payload is sent to the Registry by this workflow.

### Native commands

```text
refresh_game_content_inventory()
inspect_game_content_installation(path)
```

The refresh command runs blocking filesystem work off the WebView thread and persists the normalized snapshot. Both commands are read-only with respect to the game installation.

### Verification

Fixtures cover:

- valid and malformed `Default.ini` versions;
- sentinel fallback without version invention;
- Steam app-manifest discovery;
- escaped Steam library paths;
- pack-code classification;
- installed versus empty/partial packs;
- neutral manual paths remaining provider-unknown.

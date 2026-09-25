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

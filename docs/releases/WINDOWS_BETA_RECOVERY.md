# Windows beta recovery

This document covers recovery of the **Sims Mod Health application installation**. It is separate from rollback of a mod update.

## Principle

Installing or uninstalling Sims Mod Health must not be used as a way to repair the user's Mods folder.

The application treats the Sims 4 installation and Mods tree as user-owned data.

## If the application installation is damaged

1. Close Sims Mod Health.
2. Do not delete or move the Sims 4 Mods folder.
3. Keep the beta MSI and its verified SHA-256 checksum.
4. Re-run the same MSI to repair/reinstall the application.
5. If repair is not sufficient, uninstall Sims Mod Health from Windows Apps / Installed apps, then install the same verified MSI again.

The CI release gate verifies clean install and clean uninstall of the published MSI on a fresh supported Windows runner.

## Local application state

Sims Mod Health stores its SQLite database, update staging and restore points in the Tauri application-data directory, separate from the Sims Mods folder.

Before destructive troubleshooting of application state, copy the application-data directory instead of deleting it immediately.

The beta does not provide a user-facing “reset all local state” button because deleting state can also delete evidence needed to roll back an interrupted mod update.

## If a mod update was interrupted

Do not manually delete the app-data restore-point or staging directories.

The native transaction journal marks non-terminal updates as interrupted on startup and retains the restore point.

Use the rollback path for that transaction when the app becomes available again.

See [Staged update and rollback](../security/STAGED_UPDATE_ROLLBACK.md).

## If rollback refuses to run

Rollback deliberately refuses to overwrite a target that changed independently after the update.

In that case:

1. preserve the current target file;
2. preserve the Sims Mod Health app-data directory;
3. do not overwrite the Mods file blindly;
4. compare the current artifact with the recorded restore evidence before manual recovery.

This is a safety stop, not a request to delete the restore point.

## Uninstall

The supported beta uninstall path is Windows Apps / Installed apps or the same MSI package.

The release-validation workflow verifies that the product registration appears after installation and is removed after MSI uninstall.

## First-beta upgrade status

There is no previous supported public build, so upgrade validation is **N/A for v0.1.0-beta.1**.

From the next supported release onward, upgrade testing becomes a required release gate.

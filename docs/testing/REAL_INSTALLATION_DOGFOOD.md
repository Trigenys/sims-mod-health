# Real-installation dogfood regression contract

Issue #106 turns the v0.1.0-beta.5 real-machine failure into a repeatable regression contract. These scenarios intentionally use synthetic filesystem and UI fixtures: no The Sims 4 binaries, pack payloads, creator mods, or other copyrighted game assets are committed.

## Required scenarios

| Scenario | Regression evidence |
| --- | --- |
| Mods folder exists, program game path missing | `SimsSetupPanel.dogfood.test.tsx` keeps Scan disabled while still showing the detected Mods folder. |
| Game + user folder + Mods detected | `SimsSetupPanel.dogfood.test.tsx` requires all setup facts before enabling the first scan. |
| Custom EA / Steam install path | Frontend dogfood verifies player-facing provider/path output; native content-inventory tests verify custom path provider classification. |
| Game version unreadable | First-run UX stays blocked and explains the missing version instead of inventing a value. |
| Large real-world-style Mods library with duplicates | Native scanner dogfood creates 2,407 tiny synthetic mod files, scans all of them, and verifies exact duplicate grouping. |
| Discover blocked by missing prerequisites | `DiscoverPage.dogfood.test.tsx` uses the 2,407-file shape and verifies the exact blocker and recovery action. |
| Discover ready with game / pack / mod evidence | `DiscoverPage.dogfood.test.tsx` verifies the ready checklist and a structured player-facing recommendation reason. |
| FR and EN first-run flows | `SimsSetupPanel.dogfood.test.tsx` asserts both localized first-run outcomes. |

## Fixture policy

Fixtures must be generated from tiny strings and empty directory markers. They may mimic directory names and metadata shapes needed by the product, but they must never contain copied EA/Maxis files, DLC data, creator packages, executable payloads, screenshots, or redistributable game content.

The 2,407-file scanner fixture is deliberately synthetic. Its purpose is to reproduce the scale and state transitions seen during dogfooding without shipping or depending on a user's actual Mods library.

## CI contract

The dogfood cases run inside the existing frontend and native test gates. Scanner changes also continue through the scanner-critical gate.

The Windows MSI smoke test remains a separate release/installability gate. It proves that the produced installer installs and uninstalls correctly; it does **not** substitute for first-run product behavior and these dogfood regressions do not replace MSI validation.

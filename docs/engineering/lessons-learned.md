# Engineering lessons learned

This file stores significant RAIDER failure memory for Sims Mod Health.

## 2026-09-28 — Public landing bilingual, desktop application English-only

**Classification:** UX · architecture · testing · release

**Context**  
The public acquisition landing had an FR/EN localization layer, while the Tauri/React desktop application still embedded English copy directly in page components. Real-machine dogfooding of v0.1.0-beta.5 exposed the mismatch immediately on first launch.

**Root cause**  
Localization had been implemented as a surface-specific landing concern instead of a product-level contract. When the real Mods-folder workflow replaced fixture/demo behavior, desktop bilingual support was explicitly deferred. Subsequent Game/DLC work extended the desktop UI without a localization gate, so the deferred gap accumulated new English-only copy.

**Impact**  
A user could discover and download the product in French but immediately lose that language experience after installation. The problem affected navigation, health findings, empty/error states, Settings, Diagnostics and new Game/DLC workflows.

**Fix**  
- introduce one persistent desktop FR/EN localization provider;
- detect the system locale when no preference exists;
- expose a global EN/FR switch without adding navigation;
- localize structured Game/DLC states from domain data instead of translating Rust sentences;
- keep third-party/user content untouched unless it maps to known product vocabulary.

**Prevention**  
Frontend CI runs a TypeScript-AST localization guard. New literal JSX copy in user-facing text/labels/placeholders must be routed through the localization boundary or explicitly allowlisted as a justified brand/proper noun.

**Generalized lesson**  
Localization is a cross-cutting product contract, not a page-level decoration. If one public surface promises a language, every first-party surface in the same product should inherit a reusable locale boundary before feature work adds more copy.

## 2026-09-30 — Component-green did not mean first-run-green on v0.1.0-beta.5

**Classification:** dogfood · integration · UX truthfulness · release

**Context**  
v0.1.0-beta.5 had green component tests, controlled visual fixtures, native parser tests, and a valid Windows installer. A real installation still exposed a broken first-run story: the app could know about a Mods folder while the program game path or game version was missing, then present zeros, empty recommendations, or incomplete setup as if those were trustworthy product results.

**Root cause**  
The test suite proved subsystems in isolation. It did not keep a durable regression matrix for the combinations a real player can have on disk: a user-data folder without a detected program install, custom EA/Steam paths, unreadable version evidence, thousands of local files, and recommendation prerequisites that become available at different times.

**Impact**  
The installer could be valid and individual engines could be correct while the first-run product still looked dead or misleading. Missing facts were too easy to collapse into zero/empty states, and the difference between “nothing found” and “not measured yet” was not protected end to end.

**Fix**  
- add synthetic real-installation dogfood scenarios for setup and Discover;
- exercise custom EA/Steam paths through the native content-inventory boundary;
- run a 2,407-file synthetic Mods scan with an exact duplicate set;
- assert player-facing outcomes in both English and French;
- keep fixtures copyright-clean and generated from tiny synthetic data.

**Prevention**  
The real-installation dogfood contract in `docs/testing/REAL_INSTALLATION_DOGFOOD.md` is part of normal frontend/native CI. The Windows MSI smoke test remains separate: installability is not accepted as evidence that first-run product behavior is correct.

**Generalized lesson**  
A green component suite is necessary, not sufficient. For desktop software that discovers local state, at least one regression layer must test realistic combinations of partial discovery, scale, and recovery actions at the UX boundary.


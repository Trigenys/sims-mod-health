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

# Design Target

## 1. Product character

The interface should feel like a **calm forensic desktop utility**.

It is not a launcher, not a mod storefront, and not a neon gaming dashboard. The user should be able to answer three questions within seconds:

1. Is my installation healthy?
2. What needs my attention?
3. What exactly is this mod and what should I do about it?

The default visual target is already represented by the static Overview shell in `src/App.tsx`.

## 2. Visual direction

### Palette

- App background: `#0B1110`
- Primary surface: `#111917`
- Raised surface: `#16201D`
- Divider: `#26342F`
- Primary text: `#E9F0EF`
- Secondary text: `#91A39D`
- Accent: `#7EE2B8`
- Accent strong: `#43C995`
- Healthy: `#55D6A0`
- Update available: `#68B7FF`
- Warning / potential conflict: `#F3BD62`
- Broken / destructive: `#FF7D7D`

Color is never the only status signal: every status also uses text and/or an icon.

### Typography

Use a modern system-first sans stack during development. Prefer compact, high-legibility typography over display fonts. Headings may be tighter; operational text must remain easy to scan.

### Shape

- 10–16 px radii
- fine 1 px borders
- restrained shadows
- no glassmorphism dependency
- no excessive gradients
- motion 120–180 ms for state transitions

## 3. Desktop frame

Target design resolution: **1440 × 900**.

Minimum supported window: **1024 × 700**.

Primary navigation is a persistent left rail:

- Overview
- Library
- Updates
- Conflicts
- Diagnostics
- Discover
- Backups
- Settings

At narrower desktop widths, the rail collapses to icons.

## 4. Overview

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ THE SIMS 4 · Patch 1.128.90       [ Search 324 mods & CC ]   [ Scan now ] │
├──────────────┬───────────────────────────────────────────────────────────────┤
│ Overview     │ Your mods are mostly healthy.                 87% Health      │
│ Library      │ 28 items need attention.                                      │
│ Updates  21  │                                                               │
│ Conflicts    │ [267 Healthy] [21 Updates] [7 Conflicts] [25 Unknown]         │
│ Diagnostics  │                                                               │
│ Discover     │ Needs attention                       Current installation     │
│ Backups      │ ┌──────────────────────────────┐      ┌───────────────────┐    │
│              │ │ MCCC             Update      │      │ 324 indexed       │    │
│              │ │ RPO              Unknown     │      │ 43 mods           │    │
│              │ │ Duplicate CAS    Duplicate   │      │ 177 CC            │    │
│              │ └──────────────────────────────┘      └───────────────────┘    │
│              │                                                               │
│              │ Discover: relationships · family · realism                    │
└──────────────┴───────────────────────────────────────────────────────────────┘
```

The Overview is a triage screen, not an analytics dashboard. It should prioritize action over charts.

## 5. Library

The Library is the authoritative local inventory.

Each row should provide:

- mod/CC name
- creator
- category
- installed version when known
- current compatibility status
- source
- dependency indicator
- update indicator
- identification confidence when not exact

Filters:

- status
- category
- creator
- source
- script mod vs package-only
- identified vs unknown
- enabled vs disabled

Search must include filename aliases because many players remember a file, not a product name.

## 6. Mod detail

```text
MC COMMAND CENTER
Deaderpool · Gameplay / Story Progression             Compatible

Installed  2026.4.0
Latest     2026.5.0
Patch      1.128.90

[ Update ] [ Disable ] [ Creator page ]

What it does
Controls population, relationships, pregnancies, careers and story progression.

Dependencies
None

Files
mc_cmd_center.package
mc_cmd_center.ts4script

Known health
• Exact fingerprint match
• Current release supports installed patch
• No known incompatibility with installed library

Related mods
...
```

The page must explicitly separate **facts**, **inferred identification**, and **community reports**.

## 7. Updates

Updates are grouped by confidence:

1. verified exact update;
2. likely update requiring review;
3. source changed / unable to verify.

Bulk update must not exist until restore points and rollback are proven.

## 8. Conflicts

Use the wording **Potential conflict** unless a known incompatibility rule or deterministic duplicate proves the problem.

Conflict detail should explain the evidence:

- exact duplicate hash;
- overlapping DBPF resource keys;
- known incompatible release pair;
- missing dependency;
- duplicate script module.

## 9. Diagnostics

Diagnostics translates technical errors into mod-level evidence.

The screen should show:

- report timestamp
- source file
- implicated module/resource
- resolved mod candidate
- confidence
- suggested next action

Never imply a mod caused an exception when the evidence only shows correlation.

## 10. Discover

Recommendations must be explainable:

> Because you use several Family & Relationships mods.

Filtering happens before ranking:

- compatible with installed patch;
- not already installed;
- not abandoned;
- no known incompatibility with current library.

No recommendation should be framed as sponsored unless it actually is and is clearly labeled.

## 11. Empty, loading and error states

Every primary surface requires:

- initial empty state
- scan-in-progress state
- offline registry state
- partial-results state
- source unavailable state
- retry state

The app should remain useful locally when the registry is unavailable.

## 12. Accessibility

- keyboard-accessible primary workflows
- visible focus states
- no color-only statuses
- minimum 4.5:1 contrast for normal text
- reduced-motion support
- readable at Windows text scaling
- controls retain labels when icon meaning is ambiguous

## 13. Design acceptance

A UI issue is not done until:

- it matches this information hierarchy;
- 1024 × 700 and 1440 × 900 layouts are checked;
- light/dark decision is explicitly documented if a light theme is introduced;
- keyboard navigation is verified;
- screenshots or visual-regression evidence are attached to the PR;
- status wording follows the product state model.


## 14. Implemented primitive map

The target shell is implemented through reusable production boundaries rather than one monolithic page:

- `src/components/layout/AppShell.tsx` — persistent application frame;
- `src/components/layout/Sidebar.tsx` — primary navigation and registry state;
- `src/components/layout/Topbar.tsx` — game context, search and scan action;
- `src/components/ui/Button.tsx` — primary, secondary and text actions;
- `src/components/ui/Panel.tsx` — semantic surface primitive;
- `src/components/ui/SearchField.tsx` — labelled search control;
- `src/components/ui/StatusBadge.tsx` — text + symbol + color status treatment;
- `src/design/tokens.css` — canonical visual tokens;
- `src/features/overview/OverviewPage.tsx` — real-data triage composition with scan/registry states;
- `src/features/overview/overview.gateway.ts` — Tauri snapshot, scan command and progress-event boundary;
- `src/features/overview/overview.types.ts` — frontend contract for real Overview data;
- `src/features/overview/overview.visual.ts` — visual-test-only fixture, never the default runtime data source;
- `src/features/library/LibraryPage.tsx` — searchable/filterable local inventory;
- `src/features/library/ModDetailPage.tsx` — evidence-first detail surface;
- `src/features/diagnostics/DiagnosticsPage.tsx` — correlation-first diagnostic evidence surface;
- `src/features/library/LibraryStateNotice.tsx` — offline, partial and failure resilience states;
- `src/features/library/library.fixture.ts` — product-surface fixture isolated from future scanner/registry adapters.

The Overview now uses the local scanner plus registry resolution, compatibility and dependency/conflict engines. Its fixture exists only for deterministic visual evidence. Library adapters can be wired to resolved scanner/registry data without changing the presentation contract.

## 15. Visual evidence automation

Pull requests that change the UI trigger `.github/workflows/visual-evidence.yml`.

The workflow:

1. builds the production frontend;
2. starts the Vite preview server;
3. launches Playwright Chromium;
4. verifies no horizontal overflow at 1024×700 and 1440×900;
5. verifies the active navigation destination remains exposed;
6. captures full-page screenshots;
7. verifies visible keyboard focus on Library and Mod Detail;
8. captures Overview, Library, Mod Detail and degraded-registry evidence;
9. uploads them as the `product-surfaces-visual-evidence` artifact.

This provides repeatable evidence for UI acceptance without requiring a manual local screenshot workflow.


## Diagnostics evidence language

Diagnostic traceback/module/resource matches are presented as implicated or correlated candidates. The UI does not transform correlation evidence into a causal statement.

The Diagnostics surface exposes malformed/unsupported states without crashing, keeps local candidates visible while the Registry is offline, and shows privacy-redaction evidence for optional telemetry summaries.

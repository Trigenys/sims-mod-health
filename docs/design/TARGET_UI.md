# Sims Mod Health — Product UI Target

## 1. Approved product direction

The supplied Stitch package is the visual and information-architecture reference for the desktop application.

The product should feel like an **optimistic, evidence-first Windows utility** rather than a gaming launcher or a dense engineering console.

The user should be able to answer these questions quickly:

1. How healthy is my current Sims 4 installation?
2. What game build, packs, mods and CC are present?
3. What needs review or action?
4. What can I safely add?
5. How is the local engine configured?

## 2. Final primary navigation

The desktop application has **five primary destinations**:

~~~text
Overview
Library
Health
Discover

Settings
~~~

Settings is visually separated at the bottom of the persistent sidebar.

The following are **not** primary navigation destinations:

- Mod Detail — contextual view opened from Library, Health or Discover.
- Updates — Health subview.
- Conflicts — Health subview.
- Diagnostics — Health subview.
- Recovery / restore points — Health subview.
- Game / DLC / Packs — contextual health domains, not destinations.
- Updater / Downloader / Launcher — capabilities, not destinations.

This prevents Game, DLC and Mods from becoming separate products that duplicate the same health evidence.

## 3. Surface responsibilities

### Overview

Question answered:

> How is my installation doing right now?

Contains:

- installed game build/platform/provider context;
- installed pack count and mod/CC count;
- scan action and scan state;
- verified health score when evidence allows it;
- installed item count;
- healthy/update/conflict/unknown summary;
- top priority findings only;
- Registry/offline/partial state;
- links into the appropriate Health filter.

Overview is a triage surface. It must not reproduce the full Health inbox.

### Library

Question answered:

> What do I have installed?

Library is the authoritative local inventory.

It owns:

- search by canonical name, creator and filename aliases;
- status/category/creator/source/type/identity/enabled filters;
- installed version;
- source;
- compatibility state;
- identity confidence;
- enabled state;
- contextual Mod Detail.

Library may display a health badge, but remediation belongs to Health.

### Health

Question answered:

> What needs attention and what can I safely do?

Health is the consolidated action center.

Subviews:

~~~text
All findings | Updates | Conflicts | Diagnostics | Recovery
~~~

Health combines existing evidence without flattening evidence strength.

The **Updates** subview is one queue for Game, Pack and Mod findings. Type is a lightweight marker, not a separate navigation hierarchy. Healthy packs collapse into a compact summary instead of one card per pack. Pack detail opens contextually through progressive disclosure.

A Game/Pack finding may expose one compact provider action such as **Open EA app to update** or **Open Steam to update**. After handoff, the same finding switches to an awaiting-verification state and exposes **Verify update**. Provider launch is never presented as completed update evidence.

It must keep these states distinct:

- deterministic duplicate;
- known incompatibility;
- potential conflict;
- missing/outdated dependency;
- update available;
- broken/abandoned sourced state;
- unknown compatibility;
- diagnostic correlation.

Diagnostics must never turn correlation into causality.

Recovery is not a general Sims backup feature. It represents app-controlled restore points, update journals and rollback safety for supported mutations.

### Discover

Question answered:

> What could I safely add?

Filtering happens **before** ranking:

- explicit current-patch compatibility;
- not already installed;
- not broken/abandoned;
- no known incompatibility with the current library.

Ranking is deterministic and every result exposes a concrete `Because you use…` reason.

Discover remains separate from health evidence and contains no sponsored placement unless sponsorship is explicitly introduced and labelled later.

### Settings

Question answered:

> How does Sims Mod Health behave on this machine?

Owns:

- detected Sims 4 game / user-data / Mods paths;
- detected update provider and handoff behavior;
- Registry behavior and offline expectations;
- privacy / diagnostic telemetry consent;
- scan behavior;
- recovery/storage policy;
- appearance;
- product/about information.

Settings is configuration, not a sixth product workflow.

## 4. Approved visual system

The approved Stitch reference replaces the previous dark forensic theme.

### Palette

- canvas: porcelain / blue-tinted `#F7F9FF`;
- surfaces: white and soft blue containers;
- primary: Plumbob emerald (`#059669`);
- safe/healthy: emerald/mint;
- update/metadata: indigo;
- warning/potential conflict: amber;
- deterministic failure/destructive state: rose;
- text: deep navy rather than pure black.

Color is never the sole status signal.

### Shape

- application panels: 18–24 px radii;
- interactive chips/buttons: pill geometry;
- soft hairline borders;
- diffuse elevation instead of dark heavy shadows.

### Typography

Use a high-legibility Windows/system sans stack in the packaged beta.

The Stitch reference uses Plus Jakarta Sans for visual direction, but the desktop beta must not require a remote font request in order to render correctly.

### Layout

Target: `1440 × 900`.

Minimum supported window: `1024 × 700`.

- persistent left rail;
- rail collapses toward icon mode at compact desktop widths;
- workspace remains fluid on wide monitors;
- master/detail layouts are preferred for Library and evidence review.

## 5. Evidence and action language

The UI must preserve product semantics:

- Unknown ≠ Broken.
- Potential conflict ≠ confirmed incompatibility.
- Diagnostic correlation ≠ causality.
- Update available ≠ currently incompatible.
- Community report ≠ verified fact.
- Pack present ≠ pack owned/entitled.
- Provider opened ≠ update completed.

Actions that mutate the Mods folder must not be presented as safer than the underlying mutation pipeline proves. Game/DLC provider handoff is only marked complete after local rescan evidence verifies the resulting state.

## 6. Current production boundaries

Existing real-data boundaries remain authoritative:

- `src/features/overview/overview.gateway.ts` — local scanner + Registry-backed overview;
- `src/features/diagnostics/diagnostics.gateway.ts` — local diagnostic parser/resolution;
- `src-tauri/src/mutation` — staged single-artifact update and rollback;
- `src-tauri/src/registry.rs` — Registry resolution, health, relationships and discovery;
- `src/features/library` — current inventory presentation boundary pending full scanner/Registry adapter;
- `src/features/discover` — explainable Registry recommendation presentation.

Visual fixtures are permitted only behind explicit visual-harness query parameters and must never silently become production evidence.

## 7. Accessibility

Every primary surface requires:

- keyboard-accessible workflows;
- visible focus state;
- no color-only status;
- normal-text contrast of at least 4.5:1;
- reduced-motion behavior;
- readability at Windows text scaling;
- textual names for ambiguous icons.

## 8. Visual acceptance

UI changes are not done until:

- 1024 × 700 has no horizontal overflow;
- 1440 × 900 has no horizontal overflow;
- the active primary destination is exposed through `aria-current`;
- Overview, Library, contextual Mod Detail, Health, Discover and Settings have screenshot evidence;
- keyboard focus remains visible on core workflows;
- degraded/offline states remain useful locally;
- wording follows the evidence model above.

## 9. Approved reference assets

The source Stitch package supplied by the product owner contains:

- Overview reference;
- Library reference;
- Health & Action Center reference;
- Discover reference;
- Settings reference;
- Sims Mod Health design-system specification.

The screenshots define visual direction. Production React components and real data gateways remain the implementation source of truth.

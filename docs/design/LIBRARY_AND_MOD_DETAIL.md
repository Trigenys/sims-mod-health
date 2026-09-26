# Library and Mod Detail surfaces

## Intent

The Library is the authoritative local inventory. It is designed for fast triage rather than browsing a storefront.

The Mod Detail surface answers three questions:

1. what is installed;
2. what the app actually knows about it;
3. what needs review before the user changes anything.

## Library behavior

Search matches:

- canonical mod/CC name;
- creator;
- local filename aliases.

Unknown or unresolved files stay first-class rows instead of disappearing from the product.

Available filters:

- compatibility/health status;
- category;
- creator;
- source;
- script mod vs package-only;
- identified vs unknown;
- enabled vs disabled.

Identification confidence is visible for non-exact matches. Exact identities use a distinct exact-fingerprint label.

## Evidence hierarchy

Mod Detail keeps evidence classes visually separate:

- **Verified fact** — deterministic or sourced registry evidence;
- **Inferred identification** — probabilistic identity evidence;
- **Community report** — useful context that does not become authoritative health state.

Community reports never overwrite verified facts or patch-scoped compatibility evidence.

## Resilience states

The Library remains useful when shared data is degraded:

- **ready** — local and registry evidence available;
- **offline** — local inventory/cached evidence still available;
- **partial** — one or more registry sources unavailable;
- **failure** — registry request failed and exposes retry;
- **empty search/filter result** — clear-filters action without hiding unresolved inventory.

These states do not imply loss of local data.

## Keyboard and focus behavior

All Library rows are native buttons, filters are labelled native selects, and search is a labelled input.

Primary actions, navigation, rows, selects and search controls preserve visible focus rings. The visual-evidence script verifies focus visibility on both Library and Mod Detail.

## Responsive targets

The product surfaces are verified at:

- 1024 × 700;
- 1440 × 900.

At the minimum desktop width:

- the left navigation collapses to icons;
- the Library drops the Source column before horizontal scrolling would be required;
- Mod Detail moves the secondary panels below the main evidence column.

## Visual evidence

Pull requests changing the UI capture:

- Overview at both target resolutions;
- Library at both target resolutions;
- Mod Detail at both target resolutions;
- Library offline state at 1024 × 700.

The workflow also fails on horizontal overflow, missing active navigation, or missing visible keyboard focus.

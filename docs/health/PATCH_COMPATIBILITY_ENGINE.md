# Patch compatibility state engine

## Purpose

The health engine evaluates one resolved installed release against the player's current Sims patch without inventing certainty.

The engine keeps two ideas separate:

1. **compatibility evidence** for the installed release on the current patch;
2. **update availability** based on whether a newer release exists.

This distinction prevents a newer release from being presented as proof that either the installed or target release is compatible.

## States

The product states are:

- `compatible`
- `update_available`
- `unknown`
- `potential_conflict`
- `broken`
- `abandoned`

`unknown` is a first-class state. Lack of evidence never becomes `broken`.

## Patch scope

Compatibility evidence must have source provenance and one explicit scope:

- an exact `GamePatch`; or
- an inclusive `patch_min_version` / `patch_max_version` range.

A range may be open on either side.

When a new patch appears, an old exact-patch report does not carry forward. The result becomes `unknown` unless explicit range evidence covers the new patch or new exact evidence exists.

## Evidence precedence

Precedence is intentionally conservative:

1. exact-patch evidence outranks range evidence;
2. within the selected scope specificity, only the newest report from each source is considered;
3. if those current source reports disagree, the result is `unknown` with `disputed: true`;
4. the engine does not invent a source-trust ranking.

This keeps conflicting claims inspectable instead of silently choosing a winner.

## Update availability

A newer release is determined independently from compatibility evidence, preferring release timestamps and using normalized version ordering as a fallback.

The response includes:

- whether an update exists;
- target release/version;
- the target release's compatibility state on the current patch;
- whether target compatibility is disputed.

The top-level state becomes `update_available` only when the installed release is not already `broken`, `abandoned` or in `potential_conflict`.

Therefore `update_available` never means “confirmed compatible update”.

## API

`POST /v1/health/evaluate`

Request:

```json
{
  "patch_version": "1.128.90.1030",
  "platform": "windows",
  "installed_release_ids": ["<uuid>"]
}
```

Response items expose:

- final triage state;
- underlying compatibility state;
- dispute flag and reason;
- evidence with source/provenance and patch scope;
- update assessment.

The endpoint accepts at most 100 installed release IDs per batch.

## Privacy

Only registry release IDs and the normalized game patch version are required. No local paths, raw mod files, saves, households or diagnostics are sent.

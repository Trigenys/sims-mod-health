# Explainable deterministic discovery

## Purpose

Discovery recommends mods from the user's actual resolved library without using generative ranking.

The MVP is intentionally deterministic:

1. start from installed canonical releases;
2. compare normalized categories and features;
3. remove unsafe candidates before ranking;
4. rank by a fixed similarity score;
5. expose one concrete “Because you use…” explanation for every result.

There is no sponsored-placement concept in the schema, service or response contract.

## Taxonomy

Each canonical Mod stores two normalized JSON lists:

- `categories` — broad product areas such as `gameplay`, `relationships`, `build-buy`, `cas`;
- `features` — narrower capabilities such as `population-management`, `relationship-management`, `build-tools`, `diagnostics`.

Source ingestion applies a conservative deterministic keyword classifier only when the canonical mod does not already have curated taxonomy. Curated taxonomy is therefore preserved.

Mods with no taxonomy overlap are not recommendation candidates in the MVP.

## Endpoint

```text
POST /v1/discovery/recommend
```

Request:

```json
{
  "patch_version": "1.128.90",
  "platform": "windows",
  "installed_release_ids": ["..."],
  "limit": 12
}
```

The installed set may contain up to 500 resolved releases.

## Safety filtering

Filtering happens before ranking.

A candidate is excluded when:

- its canonical mod is already installed;
- its latest known release does not have explicit `compatible` evidence for the current patch;
- its latest release is broken, abandoned or in a potential-conflict state;
- a known incompatibility rule is active between that candidate release and any installed release.

Unknown compatibility is not promoted to safe. Discovery requires explicit compatible evidence.

Known incompatibility rules are checked in both directions:

- candidate release → installed target;
- installed release → candidate target.

Target version constraints are respected.

## Ranking

For one candidate and one installed mod:

```text
score =
  10 × shared feature count
   + 4 × shared category count
```

The candidate's anchor is the installed mod with the highest pair score.

Ties are resolved deterministically by:

1. installed mod name;
2. installed mod ID;
3. installed release ID.

Recommendation ordering is deterministic by:

1. descending score;
2. candidate name;
3. candidate mod ID;
4. candidate release ID.

No popularity, download count, advertising or generative model changes this ordering.

## Explanation

Every recommendation includes:

- the installed anchor mod;
- shared categories;
- shared features;
- a generated deterministic sentence such as:

```text
Because you use MC Command Center: Story Plus has shared features
population-management; shared categories story-progression.
```

This sentence is generated from the same evidence used by the score.

## Current limitations

The MVP recommends only canonical mods with enough taxonomy and a latest release with explicit current-patch compatibility evidence.

It does not yet:

- personalize from playtime;
- rank by popularity;
- use embeddings or an LLM;
- infer taste from personal profile data;
- contain sponsored placement.

Those are deliberate boundaries, not missing fallback behavior.

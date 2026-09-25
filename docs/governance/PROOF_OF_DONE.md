# Proof of Done

## Principle

A task is not Done because code was written or a screenshot looks correct.

**Done means the repository contains enough evidence for another engineer to verify the claim without relying on the author's memory.**

## Repository-wide Proof of Done

Every implementation issue must satisfy all applicable items:

- acceptance criteria in the issue are met;
- implementation is linked by PR to the issue;
- CI is green;
- automated tests cover new deterministic behavior where practical;
- regressions have a test when the failure can be reproduced;
- security and privacy impact is reviewed;
- user-facing behavior is documented when it changes;
- architecture documentation is updated when boundaries or contracts change;
- no secret, token, local user path or personal data is committed;
- logs do not expose raw user content unnecessarily;
- error and empty states are handled;
- the PR includes concrete verification evidence;
- follow-up work is filed as an issue instead of hidden in comments or TODO folklore.

## Evidence block

Each implementation PR should end with:

```text
Proof of Done
- Issue:
- Automated tests:
- Manual verification:
- Security/privacy check:
- Performance evidence:
- Visual evidence:
- Docs updated:
- Known limitations / follow-ups:
```

Use `N/A — reason` instead of deleting a line.

## Parser / scanner work

Additional evidence:

- malformed input fixture;
- bounded-read behavior;
- no execution of script content;
- cancellation or failure does not corrupt local state;
- representative large-library benchmark;
- unchanged-file rescan behavior verified;
- path traversal and archive bomb controls covered where relevant.

## Registry / source connector work

Additional evidence:

- source terms/API contract reviewed;
- rate-limit behavior handled;
- provenance stored with ingested data;
- deletion/withdrawal behavior considered;
- stale-source state is distinguishable from confirmed compatibility;
- connector failure does not produce false `Broken` states.

## Matching / health-engine work

Additional evidence:

- exact and probabilistic matches are distinguishable;
- confidence/evidence can be inspected;
- `Unknown` remains a first-class state;
- compatibility is scoped to an explicit game patch or supported range;
- conflicting evidence has deterministic precedence or is surfaced as disputed.

## UI work

Additional evidence:

- checked at 1024 × 700 and 1440 × 900;
- keyboard path verified;
- focus state visible;
- status is not color-only;
- loading, empty, offline and failure states considered;
- screenshot or visual-regression evidence attached.

## Update / rollback work

Additional evidence:

- restore point created before mutation;
- download source is allowlisted;
- fingerprint/hash verification performed when available;
- interrupted update leaves a recoverable state;
- rollback restores the previous working layout;
- dependencies are checked before disable/remove.

## API work

Additional evidence:

- request/response schema tests;
- auth behavior tested where applicable;
- pagination/batching considered;
- validation errors are stable and actionable;
- migration included for schema change;
- rollback or forward-fix plan documented for destructive migrations.

## Security-sensitive work

Additional evidence:

- threat model updated;
- abuse case tested;
- input size and resource limits explicit;
- no new broad Tauri capability without justification;
- dependency risk reviewed.

## Release work

A beta/release is Done only when:

- release version is traceable to commit;
- Windows artifact builds from CI;
- checksum is published with artifact;
- installation and uninstall path is verified;
- upgrade from previous supported version is tested;
- rollback guidance exists;
- release notes name known limitations;
- privacy defaults are reviewed;
- critical/high-severity open security issues are zero.

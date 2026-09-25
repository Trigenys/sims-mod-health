# Contributing

## Workflow

1. Start from an open GitHub Issue.
2. Keep the change scoped to that issue.
3. Use a short-lived branch.
4. Open a pull request that links the issue.
5. Complete the Proof of Done evidence block.
6. Merge only with green CI.

## Branches

Recommended prefixes:

- `feat/`
- `fix/`
- `docs/`
- `test/`
- `refactor/`
- `chore/`

## Commits

Use conventional, descriptive commit messages where practical.

Examples:

```text
feat(scanner): cache unchanged artifact hashes
fix(dbpf): reject resource offsets past EOF
docs(design): clarify unknown compatibility state
```

## Engineering expectations

- local mod files are untrusted input;
- script mods are inspected, never executed;
- privileged filesystem access remains in Rust;
- compatibility claims require provenance;
- `Unknown` is a valid product state;
- source terms and API policies are respected;
- avoid adding infrastructure without a measured operational benefit.

## Pull requests

A PR should include:

- what changed;
- why;
- linked issue;
- verification evidence;
- screenshots for UI changes;
- performance evidence for scanner/parser changes;
- security/privacy implications;
- documentation changes.

See [Proof of Done](docs/governance/PROOF_OF_DONE.md).

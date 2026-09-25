# Security Policy

Sims Mod Health processes untrusted third-party mod files. Security issues involving parser safety, filesystem access, update integrity, registry poisoning or privacy are treated as product-security defects.

## Supported code

Until the first beta release, only the current `main` branch is supported.

## Reporting

Do not publish exploit details, malicious fixtures or sensitive user data in a public issue.

Use a private Trigenys communication channel to report the finding to repository owners. Include:

- affected component;
- reproduction conditions;
- impact;
- minimal safe reproduction material;
- suggested mitigation if known.

## Security baseline

Review [Threat Model](docs/security/THREAT_MODEL.md) before changing:

- filesystem permissions;
- DBPF parsing;
- archive/TS4Script inspection;
- download/update behavior;
- telemetry;
- authentication;
- source ingestion.

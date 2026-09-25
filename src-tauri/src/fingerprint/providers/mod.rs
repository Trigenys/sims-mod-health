mod dbpf;
mod ts4script;

use std::path::Path;

use super::domain::{ArtifactKind, FingerprintError, FingerprintProvider, FingerprintValue};

static DBPF_PROVIDER: dbpf::DbpfResourceSignatureProvider = dbpf::DbpfResourceSignatureProvider;
static TS4SCRIPT_PROVIDER: ts4script::Ts4ScriptIdentityProvider =
    ts4script::Ts4ScriptIdentityProvider;

fn provider_for(kind: ArtifactKind) -> &'static dyn FingerprintProvider {
    match kind {
        ArtifactKind::Package => &DBPF_PROVIDER,
        ArtifactKind::Ts4Script => &TS4SCRIPT_PROVIDER,
    }
}

pub(crate) fn compute_structural_fingerprint(
    kind: ArtifactKind,
    path: &Path,
) -> Result<Option<FingerprintValue>, FingerprintError> {
    provider_for(kind).fingerprint(path)
}

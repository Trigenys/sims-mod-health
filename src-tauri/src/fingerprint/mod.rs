mod domain;
mod duplicates;
mod providers;
mod repository;
mod sha256;

pub(crate) use domain::{
    ArtifactKind, FingerprintError, FingerprintKind, FingerprintProvider, FingerprintValue,
    DBPF_RESOURCE_SIGNATURE_VERSION, SHA256_ALGORITHM_VERSION, TS4SCRIPT_SIGNATURE_VERSION,
};
pub(crate) use duplicates::{exact_duplicate_groups, ExactDuplicateGroup};
pub(crate) use providers::compute_structural_fingerprint;
pub(crate) use repository::{
    delete_cached_fingerprints, refresh_structural_fingerprint, store_sha256, FingerprintRepository,
};
pub(crate) use sha256::sha256_file;

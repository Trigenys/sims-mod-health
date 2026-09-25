use std::{
    fmt::{Display, Formatter},
    io,
    path::Path,
};

use crate::{dbpf, ts4script};

pub(crate) const SHA256_ALGORITHM_VERSION: &str = "sha256-v1";
pub(crate) const DBPF_RESOURCE_SIGNATURE_VERSION: &str = "dbpf-resource-keys-v1";
pub(crate) const TS4SCRIPT_SIGNATURE_VERSION: &str = "ts4script-python-identity-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FingerprintKind {
    Sha256,
    CurseForge,
    ResourceSignature,
    ScriptSignature,
    Quick,
}

impl FingerprintKind {
    pub(crate) fn database_value(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::CurseForge => "curseforge",
            Self::ResourceSignature => "resource_signature",
            Self::ScriptSignature => "script_signature",
            Self::Quick => "quick",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    Package,
    Ts4Script,
}

impl ArtifactKind {
    pub(crate) fn structural_fingerprint_kind(self) -> FingerprintKind {
        match self {
            Self::Package => FingerprintKind::ResourceSignature,
            Self::Ts4Script => FingerprintKind::ScriptSignature,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FingerprintValue {
    pub(crate) kind: FingerprintKind,
    pub(crate) value: String,
    pub(crate) algorithm_version: &'static str,
}

pub(crate) trait FingerprintProvider {
    fn kind(&self) -> FingerprintKind;
    fn algorithm_version(&self) -> &'static str;
    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError>;

    fn fingerprint(&self, path: &Path) -> Result<Option<FingerprintValue>, FingerprintError> {
        Ok(self.compute(path)?.map(|value| FingerprintValue {
            kind: self.kind(),
            value,
            algorithm_version: self.algorithm_version(),
        }))
    }
}

#[derive(Debug)]
pub(crate) enum FingerprintError {
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Dbpf(dbpf::DbpfError),
    Ts4Script(ts4script::Ts4ScriptError),
    ValueTooLarge(&'static str),
}

impl Display for FingerprintError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "fingerprint I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "fingerprint SQLite error: {error}"),
            Self::Dbpf(error) => write!(formatter, "DBPF fingerprint failed: {error}"),
            Self::Ts4Script(error) => write!(formatter, "TS4Script fingerprint failed: {error}"),
            Self::ValueTooLarge(context) => {
                write!(formatter, "{context} is too large to fingerprint safely")
            }
        }
    }
}

impl std::error::Error for FingerprintError {}

impl From<io::Error> for FingerprintError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for FingerprintError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<dbpf::DbpfError> for FingerprintError {
    fn from(value: dbpf::DbpfError) -> Self {
        Self::Dbpf(value)
    }
}

impl From<ts4script::Ts4ScriptError> for FingerprintError {
    fn from(value: ts4script::Ts4ScriptError) -> Self {
        Self::Ts4Script(value)
    }
}

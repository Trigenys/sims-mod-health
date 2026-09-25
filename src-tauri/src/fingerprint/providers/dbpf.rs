use std::path::Path;

use sha2::{Digest, Sha256};

use crate::dbpf;

use super::super::domain::{
    FingerprintError, FingerprintKind, FingerprintProvider, DBPF_RESOURCE_SIGNATURE_VERSION,
};

#[derive(Debug, Default)]
pub(super) struct DbpfResourceSignatureProvider;

impl FingerprintProvider for DbpfResourceSignatureProvider {
    fn kind(&self) -> FingerprintKind {
        FingerprintKind::ResourceSignature
    }

    fn algorithm_version(&self) -> &'static str {
        DBPF_RESOURCE_SIGNATURE_VERSION
    }

    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError> {
        let metadata = dbpf::parse_path(path)?;
        let mut keys = metadata.resource_keys().collect::<Vec<_>>();
        keys.sort_unstable();
        keys.dedup();

        let mut hasher = Sha256::new();
        hasher.update(b"sims-mod-health:dbpf-resource-signature:v1\0");
        hasher.update((keys.len() as u64).to_be_bytes());

        for key in keys {
            hasher.update(key.resource_type.to_be_bytes());
            hasher.update(key.group.to_be_bytes());
            hasher.update(key.instance.to_be_bytes());
        }

        Ok(Some(format!("{:x}", hasher.finalize())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbpf::ResourceKey;
    use tempfile::TempDir;

    fn minimal_dbpf(keys: &[ResourceKey]) -> Vec<u8> {
        const HEADER_SIZE: usize = 96;
        let mut index = Vec::new();
        index.extend_from_slice(&0_u32.to_le_bytes());

        for key in keys {
            index.extend_from_slice(&key.resource_type.to_le_bytes());
            index.extend_from_slice(&key.group.to_le_bytes());
            index.extend_from_slice(&((key.instance >> 32) as u32).to_le_bytes());
            index.extend_from_slice(&(key.instance as u32).to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
        }

        let mut bytes = vec![0_u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"DBPF");
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&1_u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&(keys.len() as u32).to_le_bytes());
        bytes[40..44].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        bytes[44..48].copy_from_slice(&(index.len() as u32).to_le_bytes());
        bytes.extend(index);
        bytes
    }

    #[test]
    fn resource_signature_is_independent_of_index_order() {
        let temp = TempDir::new().expect("create DBPF signature temp directory");
        let a = ResourceKey {
            resource_type: 0x1000,
            group: 0x2000,
            instance: 0x3000,
        };
        let b = ResourceKey {
            resource_type: 0x1001,
            group: 0x2001,
            instance: 0x3001,
        };
        let left = temp.path().join("left.package");
        let right = temp.path().join("right.package");

        std::fs::write(&left, minimal_dbpf(&[a, b])).expect("write left DBPF");
        std::fs::write(&right, minimal_dbpf(&[b, a])).expect("write right DBPF");

        let provider = DbpfResourceSignatureProvider;
        assert_eq!(
            provider.compute(&left).expect("left resource signature"),
            provider.compute(&right).expect("right resource signature")
        );
    }
}

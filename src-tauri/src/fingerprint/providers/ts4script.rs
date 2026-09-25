use std::{collections::BTreeSet, path::Path};

use sha2::{Digest, Sha256};

use crate::ts4script;

use super::super::domain::{
    FingerprintError, FingerprintKind, FingerprintProvider, TS4SCRIPT_SIGNATURE_VERSION,
};

#[derive(Debug, Default)]
pub(super) struct Ts4ScriptIdentityProvider;

impl FingerprintProvider for Ts4ScriptIdentityProvider {
    fn kind(&self) -> FingerprintKind {
        FingerprintKind::ScriptSignature
    }

    fn algorithm_version(&self) -> &'static str {
        TS4SCRIPT_SIGNATURE_VERSION
    }

    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError> {
        let metadata = ts4script::inspect_path(path)?;
        let modules = metadata.module_names.into_iter().collect::<BTreeSet<_>>();
        let packages = metadata.package_names.into_iter().collect::<BTreeSet<_>>();

        if modules.is_empty() && packages.is_empty() {
            return Ok(None);
        }

        let mut hasher = Sha256::new();
        hasher.update(b"sims-mod-health:ts4script-python-identity:v1\0");
        hash_string_set(&mut hasher, b"modules\0", &modules)?;
        hash_string_set(&mut hasher, b"packages\0", &packages)?;

        Ok(Some(format!("{:x}", hasher.finalize())))
    }
}

fn hash_string_set(
    hasher: &mut Sha256,
    domain: &[u8],
    values: &BTreeSet<String>,
) -> Result<(), FingerprintError> {
    hasher.update(domain);
    hasher.update((values.len() as u64).to_be_bytes());

    for value in values {
        let bytes = value.as_bytes();
        let length = u32::try_from(bytes.len())
            .map_err(|_| FingerprintError::ValueTooLarge("fingerprint identity component"))?;
        hasher.update(length.to_be_bytes());
        hasher.update(bytes);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::TempDir;
    use zip::{
        write::{SimpleFileOptions, ZipWriter},
        CompressionMethod,
    };

    fn ts4script(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::DEFLATE);

        for (name, content) in entries {
            writer.start_file(*name, options).expect("start ZIP entry");
            writer.write_all(content).expect("write ZIP entry");
        }

        writer.finish().expect("finish TS4Script").into_inner()
    }

    #[test]
    fn identity_ignores_archive_order_version_hints_and_bytecode_content() {
        let temp = TempDir::new().expect("create TS4Script signature temp directory");
        let left = temp.path().join("left.ts4script");
        let right = temp.path().join("right.ts4script");

        std::fs::write(
            &left,
            ts4script(&[
                ("creator/mod/core.pyc", b"compiled-a"),
                ("creator/mod/__init__.pyc", b"init"),
                ("manifest.txt", b"version = 1.0.0"),
            ]),
        )
        .expect("write left TS4Script");

        std::fs::write(
            &right,
            ts4script(&[
                ("manifest.txt", b"version = 2.0.0"),
                ("creator/mod/__init__.pyc", b"different-init"),
                ("creator/mod/core.pyc", b"different-bytecode"),
            ]),
        )
        .expect("write right TS4Script");

        let provider = Ts4ScriptIdentityProvider;
        assert_eq!(
            provider.compute(&left).expect("left script signature"),
            provider.compute(&right).expect("right script signature")
        );
    }
}

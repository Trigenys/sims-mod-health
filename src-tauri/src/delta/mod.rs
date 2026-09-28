use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const MAX_RND_INPUT_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) trait DeltaApplier {
    fn apply(&self, request: &DeltaApplyRequest) -> Result<DeltaApplyResult, DeltaApplyError>;
}

#[derive(Debug, Clone)]
pub(crate) struct DeltaApplyRequest {
    pub(crate) source_path: PathBuf,
    pub(crate) delta_path: PathBuf,
    pub(crate) output_path: PathBuf,
    pub(crate) expected_source_sha256: String,
    pub(crate) expected_delta_sha256: String,
    pub(crate) expected_target_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeltaApplyResult {
    pub(crate) source_sha256: String,
    pub(crate) delta_sha256: String,
    pub(crate) target_sha256: String,
    pub(crate) output_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Xdelta3RustApplier;

#[derive(Debug)]
pub(crate) enum DeltaApplyError {
    Io(std::io::Error),
    InvalidHash {
        artifact: &'static str,
    },
    UnsafeInput(String),
    InputTooLarge {
        artifact: &'static str,
        bytes: u64,
    },
    IntegrityMismatch {
        artifact: &'static str,
        expected: String,
        observed: String,
    },
    Decode(String),
    StagingInterrupted,
    InsufficientDiskSpace,
}

impl Display for DeltaApplyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "delta I/O error: {error}"),
            Self::InvalidHash { artifact } => {
                write!(
                    formatter,
                    "{artifact} SHA-256 must be exactly 64 hexadecimal characters"
                )
            }
            Self::UnsafeInput(reason) => write!(formatter, "unsafe delta input: {reason}"),
            Self::InputTooLarge { artifact, bytes } => write!(
                formatter,
                "{artifact} is {bytes} bytes, above the {MAX_RND_INPUT_BYTES}-byte R&D memory limit"
            ),
            Self::IntegrityMismatch {
                artifact,
                expected,
                observed,
            } => write!(
                formatter,
                "{artifact} SHA-256 mismatch: expected {expected}, observed {observed}"
            ),
            Self::Decode(detail) => write!(formatter, "xdelta3 decode failed: {detail}"),
            Self::StagingInterrupted => {
                write!(formatter, "delta staging write was interrupted and cleaned up")
            }
            Self::InsufficientDiskSpace => {
                write!(formatter, "delta staging failed because the destination ran out of space")
            }
        }
    }
}

impl Error for DeltaApplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DeltaApplyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

trait StagingWriter {
    fn write_staged(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
struct FsStagingWriter;

impl StagingWriter for FsStagingWriter {
    fn write_staged(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }
}

impl DeltaApplier for Xdelta3RustApplier {
    fn apply(&self, request: &DeltaApplyRequest) -> Result<DeltaApplyResult, DeltaApplyError> {
        apply_with_writer(request, &FsStagingWriter)
    }
}

fn apply_with_writer(
    request: &DeltaApplyRequest,
    writer: &dyn StagingWriter,
) -> Result<DeltaApplyResult, DeltaApplyError> {
    validate_request(request)?;

    let expected_source = normalize_hash("source", &request.expected_source_sha256)?;
    let expected_delta = normalize_hash("delta", &request.expected_delta_sha256)?;
    let expected_target = normalize_hash("target", &request.expected_target_sha256)?;

    let source_sha = hash_file(&request.source_path)?;
    verify_hash("source", &expected_source, &source_sha)?;

    let delta_sha = hash_file(&request.delta_path)?;
    verify_hash("delta", &expected_delta, &delta_sha)?;

    let source_bytes = read_bounded(&request.source_path, "source")?;
    let delta_bytes = read_bounded(&request.delta_path, "delta")?;
    let target_bytes = decode_bounded(&delta_bytes, &source_bytes)?;

    if let Err(error) = writer.write_staged(&request.output_path, &target_bytes) {
        cleanup_output(&request.output_path);
        return Err(classify_staging_write_error(error));
    }

    let target_sha = hash_file(&request.output_path)?;
    if let Err(error) = verify_hash("target", &expected_target, &target_sha) {
        cleanup_output(&request.output_path);
        return Err(error);
    }

    Ok(DeltaApplyResult {
        source_sha256: source_sha,
        delta_sha256: delta_sha,
        target_sha256: target_sha,
        output_bytes: target_bytes.len() as u64,
    })
}

fn classify_staging_write_error(error: std::io::Error) -> DeltaApplyError {
    if error.kind() == std::io::ErrorKind::Interrupted {
        return DeltaApplyError::StagingInterrupted;
    }

    if matches!(error.raw_os_error(), Some(28 | 112)) {
        return DeltaApplyError::InsufficientDiskSpace;
    }

    DeltaApplyError::Io(error)
}

fn validate_request(request: &DeltaApplyRequest) -> Result<(), DeltaApplyError> {
    ensure_regular_input(&request.source_path, "source")?;
    ensure_regular_input(&request.delta_path, "delta")?;

    if request.output_path.exists() {
        return Err(DeltaApplyError::UnsafeInput(
            "output must not already exist; delta application is staging-only".to_string(),
        ));
    }

    let source = fs::canonicalize(&request.source_path)?;
    let delta = fs::canonicalize(&request.delta_path)?;
    let output_parent = request.output_path.parent().ok_or_else(|| {
        DeltaApplyError::UnsafeInput("output has no parent directory".to_string())
    })?;

    if !output_parent.is_dir() {
        return Err(DeltaApplyError::UnsafeInput(
            "output parent must already exist".to_string(),
        ));
    }

    let output_parent = fs::canonicalize(output_parent)?;
    let output_name = request
        .output_path
        .file_name()
        .ok_or_else(|| DeltaApplyError::UnsafeInput("output has no file name".to_string()))?;
    let staged_output = output_parent.join(output_name);

    if staged_output == source || staged_output == delta {
        return Err(DeltaApplyError::UnsafeInput(
            "output must be distinct from source and delta inputs".to_string(),
        ));
    }

    Ok(())
}

fn ensure_regular_input(path: &Path, label: &'static str) -> Result<(), DeltaApplyError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DeltaApplyError::UnsafeInput(format!("{label} is not readable: {error}"))
    })?;

    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(DeltaApplyError::UnsafeInput(format!(
            "{label} must be a regular non-symlink file"
        )));
    }

    if metadata.len() > MAX_RND_INPUT_BYTES {
        return Err(DeltaApplyError::InputTooLarge {
            artifact: label,
            bytes: metadata.len(),
        });
    }

    Ok(())
}

fn read_bounded(path: &Path, artifact: &'static str) -> Result<Vec<u8>, DeltaApplyError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_RND_INPUT_BYTES {
        return Err(DeltaApplyError::InputTooLarge {
            artifact,
            bytes: metadata.len(),
        });
    }
    fs::read(path).map_err(DeltaApplyError::Io)
}

fn decode_bounded(delta: &[u8], source: &[u8]) -> Result<Vec<u8>, DeltaApplyError> {
    let combined = source.len().saturating_add(delta.len()) as u64;
    if combined > (u32::MAX as u64) / 2 {
        return Err(DeltaApplyError::Decode(
            "source + delta exceed the safe in-memory bound of the reused xdelta3 API".to_string(),
        ));
    }

    xdelta3::decode(delta, source).ok_or_else(|| {
        DeltaApplyError::Decode(
            "reused xdelta3 binding rejected the VCDIFF stream or its output buffer was insufficient"
                .to_string(),
        )
    })
}

fn normalize_hash(artifact: &'static str, value: &str) -> Result<String, DeltaApplyError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DeltaApplyError::InvalidHash { artifact });
    }
    Ok(normalized)
}

fn verify_hash(
    artifact: &'static str,
    expected: &str,
    observed: &str,
) -> Result<(), DeltaApplyError> {
    if expected == observed {
        Ok(())
    } else {
        Err(DeltaApplyError::IntegrityMismatch {
            artifact,
            expected: expected.to_string(),
            observed: observed.to_string(),
        })
    }
}

fn hash_file(path: &Path) -> Result<String, DeltaApplyError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn cleanup_output(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn write_fixture(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("write fixture");
    }

    fn valid_patch(source: &[u8], target: &[u8]) -> Vec<u8> {
        xdelta3::encode(target, source).expect("encode fixture")
    }

    fn request_for(source: &Path, delta: &Path, target: &Path, output: &Path) -> DeltaApplyRequest {
        DeltaApplyRequest {
            source_path: source.to_path_buf(),
            delta_path: delta.to_path_buf(),
            output_path: output.to_path_buf(),
            expected_source_sha256: hash_file(source).expect("source hash"),
            expected_delta_sha256: hash_file(delta).expect("delta hash"),
            expected_target_sha256: hash_file(target).expect("target hash"),
        }
    }

    #[test]
    fn rejects_source_hash_mismatch_before_decode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&target, b"target");
        write_fixture(&delta, &valid_patch(b"source", b"target"));

        let mut request = request_for(&source, &delta, &target, &output);
        request.expected_source_sha256 = "0".repeat(64);

        let error = Xdelta3RustApplier
            .apply(&request)
            .expect_err("source mismatch must fail before decode");

        assert!(matches!(
            error,
            DeltaApplyError::IntegrityMismatch {
                artifact: "source",
                ..
            }
        ));
        assert!(!output.exists());
    }

    #[test]
    fn rejects_delta_hash_mismatch_before_decode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&target, b"target");
        write_fixture(&delta, &valid_patch(b"source", b"target"));

        let mut request = request_for(&source, &delta, &target, &output);
        request.expected_delta_sha256 = "0".repeat(64);

        let error = Xdelta3RustApplier
            .apply(&request)
            .expect_err("delta mismatch must fail before decode");

        assert!(matches!(
            error,
            DeltaApplyError::IntegrityMismatch {
                artifact: "delta",
                ..
            }
        ));
        assert!(!output.exists());
    }

    #[test]
    fn refuses_existing_output() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&target, b"target");
        write_fixture(&delta, &valid_patch(b"source", b"target"));
        write_fixture(&output, b"do not overwrite");

        let request = request_for(&source, &delta, &target, &output);

        let error = Xdelta3RustApplier
            .apply(&request)
            .expect_err("staging adapter must never overwrite an existing output");

        assert!(matches!(error, DeltaApplyError::UnsafeInput(_)));
        assert_eq!(
            fs::read(&output).expect("existing output"),
            b"do not overwrite"
        );
    }

    #[test]
    fn rust_binding_fixture_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        let source_bytes = b"Sims Mod Health\nfixture-version=1\nstate=before\n";
        let target_bytes =
            b"Sims Mod Health\nfixture-version=1\nstate=after\ncompatibility=verified\n";
        let patch = valid_patch(source_bytes, target_bytes);

        write_fixture(&source, source_bytes);
        write_fixture(&target, target_bytes);
        write_fixture(&delta, &patch);

        let request = request_for(&source, &delta, &target, &output);
        let result = Xdelta3RustApplier
            .apply(&request)
            .expect("verified delta apply");

        assert_eq!(fs::read(&output).expect("output"), target_bytes);
        assert_eq!(
            result.target_sha256,
            hash_file(&target).expect("target hash")
        );
        assert_eq!(result.output_bytes, target_bytes.len() as u64);
    }

    #[test]
    fn corrupted_delta_fails_closed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        let source_bytes = b"before-before-before-before";
        let target_bytes = b"after-after-after-after";
        let mut patch = valid_patch(source_bytes, target_bytes);
        let middle = patch.len() / 2;
        patch[middle] ^= 0x5A;

        write_fixture(&source, source_bytes);
        write_fixture(&target, target_bytes);
        write_fixture(&delta, &patch);

        let request = request_for(&source, &delta, &target, &output);
        let error = Xdelta3RustApplier
            .apply(&request)
            .expect_err("corrupted patch must not be accepted");

        assert!(matches!(
            error,
            DeltaApplyError::Decode(_)
                | DeltaApplyError::IntegrityMismatch {
                    artifact: "target",
                    ..
                }
        ));
        assert!(!output.exists());
    }

    struct PartialFailureWriter {
        bytes_before_failure: usize,
        raw_os_error: Option<i32>,
        interrupted: bool,
    }

    impl StagingWriter for PartialFailureWriter {
        fn write_staged(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
            let mut file = File::create(path)?;
            let prefix = self.bytes_before_failure.min(bytes.len());
            file.write_all(&bytes[..prefix])?;
            file.sync_all()?;

            if self.interrupted {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "simulated interrupted staging write",
                ));
            }

            Err(std::io::Error::from_raw_os_error(
                self.raw_os_error.unwrap_or(112),
            ))
        }
    }

    #[test]
    fn interrupted_staging_write_removes_partial_output_and_preserves_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"before-before-before-before");
        write_fixture(&target, b"after-after-after-after");
        write_fixture(
            &delta,
            &valid_patch(b"before-before-before-before", b"after-after-after-after"),
        );

        let source_before = hash_file(&source).expect("source hash");
        let request = request_for(&source, &delta, &target, &output);
        let writer = PartialFailureWriter {
            bytes_before_failure: 7,
            raw_os_error: None,
            interrupted: true,
        };

        let error = apply_with_writer(&request, &writer)
            .expect_err("interrupted staging write must fail closed");

        assert!(matches!(error, DeltaApplyError::StagingInterrupted));
        assert!(!output.exists(), "partial staged output must be removed");
        assert_eq!(
            hash_file(&source).expect("source hash after interruption"),
            source_before,
            "source must remain untouched"
        );
    }

    #[test]
    fn low_disk_staging_failure_removes_partial_output_and_preserves_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"before-before-before-before");
        write_fixture(&target, b"after-after-after-after");
        write_fixture(
            &delta,
            &valid_patch(b"before-before-before-before", b"after-after-after-after"),
        );

        let source_before = hash_file(&source).expect("source hash");
        let request = request_for(&source, &delta, &target, &output);
        let writer = PartialFailureWriter {
            bytes_before_failure: 5,
            raw_os_error: Some(112),
            interrupted: false,
        };

        let error =
            apply_with_writer(&request, &writer).expect_err("disk-full staging write must fail");

        assert!(matches!(error, DeltaApplyError::InsufficientDiskSpace));
        assert!(!output.exists(), "partial staged output must be removed");
        assert_eq!(
            hash_file(&source).expect("source hash after disk-full failure"),
            source_before,
            "source must remain untouched"
        );
    }

    #[test]
    fn target_hash_mismatch_removes_staged_output() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"before");
        write_fixture(&target, b"after");
        write_fixture(&delta, &valid_patch(b"before", b"after"));

        let mut request = request_for(&source, &delta, &target, &output);
        request.expected_target_sha256 = "0".repeat(64);

        let error = Xdelta3RustApplier
            .apply(&request)
            .expect_err("wrong target hash must fail closed");

        assert!(matches!(
            error,
            DeltaApplyError::IntegrityMismatch {
                artifact: "target",
                ..
            }
        ));
        assert!(!output.exists(), "failed staged output must be removed");
    }

    #[test]
    #[ignore = "R&D benchmark is invoked explicitly by Impact-aware Rust CI"]
    fn eight_megabyte_fixture_benchmark() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        let mut source_bytes = vec![b'A'; 8 * 1024 * 1024];
        for index in (0..source_bytes.len()).step_by(4096) {
            source_bytes[index] = (index % 251) as u8;
        }
        let mut target_bytes = source_bytes.clone();
        for index in (2048..target_bytes.len()).step_by(8192) {
            target_bytes[index] ^= 0x5A;
        }
        target_bytes.extend_from_slice(b"SMH-XDELTA-RD-TARGET");

        let encode_started = Instant::now();
        let patch = valid_patch(&source_bytes, &target_bytes);
        let encode_elapsed = encode_started.elapsed();

        write_fixture(&source, &source_bytes);
        write_fixture(&target, &target_bytes);
        write_fixture(&delta, &patch);

        let request = request_for(&source, &delta, &target, &output);
        let apply_started = Instant::now();
        let result = Xdelta3RustApplier
            .apply(&request)
            .expect("verified delta apply");
        let apply_elapsed = apply_started.elapsed();

        assert_eq!(result.output_bytes, target_bytes.len() as u64);
        assert_eq!(fs::read(&output).expect("output"), target_bytes);

        let binding_output_capacity_bytes = source_bytes
            .len()
            .saturating_add(patch.len())
            .saturating_mul(2);
        let rust_decode_buffer_budget_bytes = source_bytes
            .len()
            .saturating_add(patch.len())
            .saturating_add(binding_output_capacity_bytes);
        let staged_disk_bytes = patch.len().saturating_add(target_bytes.len());

        eprintln!(
            "XDELTA_BENCH source_bytes={} target_bytes={} delta_bytes={} staged_disk_bytes={} rust_decode_buffer_budget_bytes={} encode_ms={} apply_ms={}",
            source_bytes.len(),
            target_bytes.len(),
            patch.len(),
            staged_disk_bytes,
            rust_decode_buffer_budget_bytes,
            encode_elapsed.as_millis(),
            apply_elapsed.as_millis()
        );
    }
}

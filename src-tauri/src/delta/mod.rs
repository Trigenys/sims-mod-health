use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone)]
pub(crate) struct Xdelta3CliApplier {
    binary_path: PathBuf,
}

impl Xdelta3CliApplier {
    pub(crate) fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum DeltaApplyError {
    Io(std::io::Error),
    InvalidHash { artifact: &'static str },
    UnsafeInput(String),
    IntegrityMismatch {
        artifact: &'static str,
        expected: String,
        observed: String,
    },
    ExecutionFailed { status: Option<i32>, stderr: String },
    MissingOutput,
}

impl Display for DeltaApplyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "delta I/O error: {error}"),
            Self::InvalidHash { artifact } => {
                write!(formatter, "{artifact} SHA-256 must be exactly 64 hexadecimal characters")
            }
            Self::UnsafeInput(reason) => write!(formatter, "unsafe delta input: {reason}"),
            Self::IntegrityMismatch {
                artifact,
                expected,
                observed,
            } => write!(
                formatter,
                "{artifact} SHA-256 mismatch: expected {expected}, observed {observed}"
            ),
            Self::ExecutionFailed { status, stderr } => write!(
                formatter,
                "xdelta3 decode failed with status {}: {}",
                status
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "terminated".to_string()),
                stderr.trim()
            ),
            Self::MissingOutput => {
                write!(formatter, "xdelta3 reported success without producing an output file")
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

impl DeltaApplier for Xdelta3CliApplier {
    fn apply(&self, request: &DeltaApplyRequest) -> Result<DeltaApplyResult, DeltaApplyError> {
        validate_request(request)?;

        let expected_source = normalize_hash("source", &request.expected_source_sha256)?;
        let expected_delta = normalize_hash("delta", &request.expected_delta_sha256)?;
        let expected_target = normalize_hash("target", &request.expected_target_sha256)?;

        let source_sha = hash_file(&request.source_path)?;
        verify_hash("source", &expected_source, &source_sha)?;

        let delta_sha = hash_file(&request.delta_path)?;
        verify_hash("delta", &expected_delta, &delta_sha)?;

        let execution = Command::new(&self.binary_path)
            .arg("-d")
            .arg("-s")
            .arg(&request.source_path)
            .arg(&request.delta_path)
            .arg(&request.output_path)
            .output()?;

        if !execution.status.success() {
            cleanup_output(&request.output_path);
            return Err(DeltaApplyError::ExecutionFailed {
                status: execution.status.code(),
                stderr: String::from_utf8_lossy(&execution.stderr).into_owned(),
            });
        }

        if !request.output_path.is_file() {
            cleanup_output(&request.output_path);
            return Err(DeltaApplyError::MissingOutput);
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
            output_bytes: fs::metadata(&request.output_path)?.len(),
        })
    }
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
    let output_parent = request
        .output_path
        .parent()
        .ok_or_else(|| DeltaApplyError::UnsafeInput("output has no parent directory".to_string()))?;

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

    Ok(())
}

fn normalize_hash(
    artifact: &'static str,
    value: &str,
) -> Result<String, DeltaApplyError> {
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
    use std::{env, fs, process::Command, time::Instant};

    fn write_fixture(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("write fixture");
    }

    #[test]
    fn rejects_source_hash_mismatch_before_execution() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&delta, b"delta");

        let request = DeltaApplyRequest {
            source_path: source,
            delta_path: delta.clone(),
            output_path: output,
            expected_source_sha256: "0".repeat(64),
            expected_delta_sha256: hash_file(&delta).expect("delta hash"),
            expected_target_sha256: "1".repeat(64),
        };

        let error = Xdelta3CliApplier::new(temp.path().join("missing-xdelta3"))
            .apply(&request)
            .expect_err("source mismatch must fail before process execution");

        assert!(matches!(
            error,
            DeltaApplyError::IntegrityMismatch {
                artifact: "source",
                ..
            }
        ));
    }

    #[test]
    fn rejects_delta_hash_mismatch_before_execution() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&delta, b"delta");

        let request = DeltaApplyRequest {
            source_path: source.clone(),
            delta_path: delta,
            output_path: output,
            expected_source_sha256: hash_file(&source).expect("source hash"),
            expected_delta_sha256: "0".repeat(64),
            expected_target_sha256: "1".repeat(64),
        };

        let error = Xdelta3CliApplier::new(temp.path().join("missing-xdelta3"))
            .apply(&request)
            .expect_err("delta mismatch must fail before process execution");

        assert!(matches!(
            error,
            DeltaApplyError::IntegrityMismatch {
                artifact: "delta",
                ..
            }
        ));
    }

    #[test]
    fn refuses_existing_output() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"source");
        write_fixture(&delta, b"delta");
        write_fixture(&output, b"do not overwrite");

        let request = DeltaApplyRequest {
            source_path: source.clone(),
            delta_path: delta.clone(),
            output_path: output,
            expected_source_sha256: hash_file(&source).expect("source hash"),
            expected_delta_sha256: hash_file(&delta).expect("delta hash"),
            expected_target_sha256: "1".repeat(64),
        };

        let error = Xdelta3CliApplier::new(temp.path().join("missing-xdelta3"))
            .apply(&request)
            .expect_err("staging adapter must never overwrite an existing output");

        assert!(matches!(error, DeltaApplyError::UnsafeInput(_)));
    }

    #[test]
    #[ignore = "requires the pinned xdelta3 3.2.0 binary via XDELTA3_BIN"]
    fn xdelta3_fixture_round_trip() {
        let binary = env::var_os("XDELTA3_BIN").expect("XDELTA3_BIN");
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        let source_bytes = b"Sims Mod Health\nfixture-version=1\nstate=before\n";
        let target_bytes = b"Sims Mod Health\nfixture-version=1\nstate=after\ncompatibility=verified\n";
        write_fixture(&source, source_bytes);
        write_fixture(&target, target_bytes);

        let encoded = Command::new(&binary)
            .arg("-e")
            .arg("-s")
            .arg(&source)
            .arg(&target)
            .arg(&delta)
            .output()
            .expect("run xdelta3 encoder");
        assert!(
            encoded.status.success(),
            "xdelta3 encode failed: {}",
            String::from_utf8_lossy(&encoded.stderr)
        );

        let request = DeltaApplyRequest {
            source_path: source.clone(),
            delta_path: delta.clone(),
            output_path: output.clone(),
            expected_source_sha256: hash_file(&source).expect("source hash"),
            expected_delta_sha256: hash_file(&delta).expect("delta hash"),
            expected_target_sha256: hash_file(&target).expect("target hash"),
        };

        let result = Xdelta3CliApplier::new(binary)
            .apply(&request)
            .expect("verified delta apply");

        assert_eq!(fs::read(&output).expect("output"), target_bytes);
        assert_eq!(result.target_sha256, hash_file(&target).expect("target hash"));
    }

    #[test]
    #[ignore = "requires the pinned xdelta3 3.2.0 binary via XDELTA3_BIN"]
    fn xdelta3_target_hash_mismatch_removes_staged_output() {
        let binary = env::var_os("XDELTA3_BIN").expect("XDELTA3_BIN");
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");
        let delta = temp.path().join("patch.vcdiff");
        let output = temp.path().join("output.bin");

        write_fixture(&source, b"before");
        write_fixture(&target, b"after");

        let encoded = Command::new(&binary)
            .arg("-e")
            .arg("-s")
            .arg(&source)
            .arg(&target)
            .arg(&delta)
            .output()
            .expect("run xdelta3 encoder");
        assert!(encoded.status.success());

        let request = DeltaApplyRequest {
            source_path: source.clone(),
            delta_path: delta.clone(),
            output_path: output.clone(),
            expected_source_sha256: hash_file(&source).expect("source hash"),
            expected_delta_sha256: hash_file(&delta).expect("delta hash"),
            expected_target_sha256: "0".repeat(64),
        };

        let error = Xdelta3CliApplier::new(binary)
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
    #[ignore = "requires the pinned xdelta3 3.2.0 binary via XDELTA3_BIN"]
    fn xdelta3_benchmark_fixture() {
        let binary = env::var_os("XDELTA3_BIN").expect("XDELTA3_BIN");
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

        write_fixture(&source, &source_bytes);
        write_fixture(&target, &target_bytes);

        let encode_started = Instant::now();
        let encoded = Command::new(&binary)
            .arg("-e")
            .arg("-s")
            .arg(&source)
            .arg(&target)
            .arg(&delta)
            .output()
            .expect("run xdelta3 encoder");
        assert!(encoded.status.success());
        let encode_elapsed = encode_started.elapsed();

        let request = DeltaApplyRequest {
            source_path: source.clone(),
            delta_path: delta.clone(),
            output_path: output.clone(),
            expected_source_sha256: hash_file(&source).expect("source hash"),
            expected_delta_sha256: hash_file(&delta).expect("delta hash"),
            expected_target_sha256: hash_file(&target).expect("target hash"),
        };

        let apply_started = Instant::now();
        let result = Xdelta3CliApplier::new(binary)
            .apply(&request)
            .expect("verified delta apply");
        let apply_elapsed = apply_started.elapsed();

        assert_eq!(result.output_bytes, target_bytes.len() as u64);
        assert_eq!(fs::read(&output).expect("output"), target_bytes);

        eprintln!(
            "XDELTA_BENCH source_bytes={} target_bytes={} delta_bytes={} encode_ms={} apply_ms={}",
            source_bytes.len(),
            target_bytes.len(),
            fs::metadata(&delta).expect("delta metadata").len(),
            encode_elapsed.as_millis(),
            apply_elapsed.as_millis()
        );
    }
}

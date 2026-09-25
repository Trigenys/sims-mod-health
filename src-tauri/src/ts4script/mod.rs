use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
    fs::File,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use serde::Serialize;
use zip::{result::ZipError, ZipArchive};

pub(crate) const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_ARCHIVE_ENTRIES: usize = 4_096;
pub(crate) const MAX_ENTRY_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_TOTAL_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_COMPRESSION_RATIO: u64 = 500;
pub(crate) const MAX_PATH_DEPTH: usize = 32;
pub(crate) const MAX_ENTRY_NAME_BYTES: usize = 1_024;
pub(crate) const MAX_METADATA_ENTRY_BYTES: u64 = 64 * 1024;
pub(crate) const MAX_TOTAL_METADATA_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_VERSION_HINTS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScriptArchiveEntry {
    pub(crate) path: String,
    pub(crate) compressed_size: u64,
    pub(crate) expanded_size: u64,
    pub(crate) crc32: u32,
    pub(crate) directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionHint {
    pub(crate) source_entry: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScriptArchiveMetadata {
    pub(crate) entries: Vec<ScriptArchiveEntry>,
    pub(crate) module_names: Vec<String>,
    pub(crate) package_names: Vec<String>,
    pub(crate) version_hints: Vec<VersionHint>,
    pub(crate) total_expanded_bytes: u64,
    pub(crate) metadata_bytes_read: u64,
    pub(crate) metadata_budget_exhausted: bool,
}

#[derive(Debug)]
pub(crate) enum Ts4ScriptError {
    Io(io::Error),
    Zip(ZipError),
    ArchiveTooLarge {
        actual: u64,
        maximum: u64,
    },
    TooManyEntries {
        count: usize,
        maximum: usize,
    },
    EntryNameTooLong {
        name: String,
        maximum: usize,
    },
    UnsafePath {
        name: String,
    },
    PathTooDeep {
        name: String,
        depth: usize,
        maximum: usize,
    },
    SymlinkEntry {
        name: String,
    },
    EncryptedEntry {
        name: String,
    },
    EntryTooLarge {
        name: String,
        expanded: u64,
        maximum: u64,
    },
    CompressionRatioExceeded {
        name: String,
        compressed: u64,
        expanded: u64,
        maximum_ratio: u64,
    },
    ExpandedBudgetExceeded {
        expanded: u64,
        maximum: u64,
    },
    ArithmeticOverflow(&'static str),
}

impl Display for Ts4ScriptError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "TS4Script I/O error: {error}"),
            Self::Zip(error) => write!(formatter, "invalid TS4Script ZIP archive: {error}"),
            Self::ArchiveTooLarge { actual, maximum } => write!(
                formatter,
                "TS4Script archive is {actual} bytes; maximum supported is {maximum}"
            ),
            Self::TooManyEntries { count, maximum } => write!(
                formatter,
                "TS4Script archive has {count} entries; maximum supported is {maximum}"
            ),
            Self::EntryNameTooLong { name, maximum } => write!(
                formatter,
                "TS4Script entry name exceeds {maximum} bytes: {name}"
            ),
            Self::UnsafePath { name } => {
                write!(formatter, "TS4Script entry uses an unsafe path: {name}")
            }
            Self::PathTooDeep {
                name,
                depth,
                maximum,
            } => write!(
                formatter,
                "TS4Script entry path depth {depth} exceeds {maximum}: {name}"
            ),
            Self::SymlinkEntry { name } => {
                write!(formatter, "TS4Script symlink entries are not accepted: {name}")
            }
            Self::EncryptedEntry { name } => {
                write!(formatter, "TS4Script encrypted entries are not inspectable: {name}")
            }
            Self::EntryTooLarge {
                name,
                expanded,
                maximum,
            } => write!(
                formatter,
                "TS4Script entry expands to {expanded} bytes; maximum is {maximum}: {name}"
            ),
            Self::CompressionRatioExceeded {
                name,
                compressed,
                expanded,
                maximum_ratio,
            } => write!(
                formatter,
                "TS4Script entry compression ratio exceeds {maximum_ratio}:1 ({compressed} -> {expanded} bytes): {name}"
            ),
            Self::ExpandedBudgetExceeded { expanded, maximum } => write!(
                formatter,
                "TS4Script expanded-data budget is {expanded} bytes; maximum is {maximum}"
            ),
            Self::ArithmeticOverflow(context) => {
                write!(formatter, "integer overflow while validating {context}")
            }
        }
    }
}

impl std::error::Error for Ts4ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Zip(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Ts4ScriptError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ZipError> for Ts4ScriptError {
    fn from(value: ZipError) -> Self {
        Self::Zip(value)
    }
}

pub(crate) fn inspect_path(path: &Path) -> Result<ScriptArchiveMetadata, Ts4ScriptError> {
    let file = File::open(path)?;
    let archive_size = file.metadata()?.len();

    if archive_size > MAX_ARCHIVE_BYTES {
        return Err(Ts4ScriptError::ArchiveTooLarge {
            actual: archive_size,
            maximum: MAX_ARCHIVE_BYTES,
        });
    }

    inspect_reader(file)
}

fn inspect_reader<R: Read + Seek>(reader: R) -> Result<ScriptArchiveMetadata, Ts4ScriptError> {
    let mut archive = ZipArchive::new(reader)?;

    validate_entry_count(archive.len())?;

    let mut entries = Vec::with_capacity(archive.len());
    let mut module_names = BTreeSet::new();
    let mut package_names = BTreeSet::new();
    let mut version_hints = Vec::new();
    let mut total_expanded_bytes = 0_u64;
    let mut metadata_bytes_read = 0_u64;
    let mut metadata_budget_exhausted = false;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();

        validate_entry_path(&name)?;

        if entry.is_symlink() {
            return Err(Ts4ScriptError::SymlinkEntry { name });
        }

        if entry.encrypted() {
            return Err(Ts4ScriptError::EncryptedEntry { name });
        }

        let compressed_size = entry.compressed_size();
        let expanded_size = entry.size();
        validate_entry_limits(&name, compressed_size, expanded_size)?;

        total_expanded_bytes = add_expanded_budget(total_expanded_bytes, expanded_size)?;

        let directory = entry.is_dir();
        if !directory {
            if let Some((module_name, package_name)) = python_identity(&name) {
                module_names.insert(module_name);
                if let Some(package_name) = package_name {
                    package_names.insert(package_name);
                }
            }

            if is_metadata_candidate(&name) && expanded_size <= MAX_METADATA_ENTRY_BYTES {
                let remaining = MAX_TOTAL_METADATA_BYTES.saturating_sub(metadata_bytes_read);
                if remaining == 0 || expanded_size > remaining {
                    metadata_budget_exhausted = true;
                } else {
                    let mut bytes = Vec::with_capacity(expanded_size as usize);
                    entry.by_ref().take(expanded_size).read_to_end(&mut bytes)?;
                    metadata_bytes_read =
                        metadata_bytes_read.checked_add(bytes.len() as u64).ok_or(
                            Ts4ScriptError::ArithmeticOverflow("TS4Script metadata read budget"),
                        )?;

                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        for value in extract_version_hints(text) {
                            if version_hints.len() >= MAX_VERSION_HINTS {
                                metadata_budget_exhausted = true;
                                break;
                            }
                            version_hints.push(VersionHint {
                                source_entry: name.clone(),
                                value,
                            });
                        }
                    }
                }
            }
        }

        entries.push(ScriptArchiveEntry {
            path: name,
            compressed_size,
            expanded_size,
            crc32: entry.crc32(),
            directory,
        });
    }

    Ok(ScriptArchiveMetadata {
        entries,
        module_names: module_names.into_iter().collect(),
        package_names: package_names.into_iter().collect(),
        version_hints,
        total_expanded_bytes,
        metadata_bytes_read,
        metadata_budget_exhausted,
    })
}

fn validate_entry_count(count: usize) -> Result<(), Ts4ScriptError> {
    if count > MAX_ARCHIVE_ENTRIES {
        return Err(Ts4ScriptError::TooManyEntries {
            count,
            maximum: MAX_ARCHIVE_ENTRIES,
        });
    }

    Ok(())
}

fn add_expanded_budget(current: u64, next: u64) -> Result<u64, Ts4ScriptError> {
    let expanded = current
        .checked_add(next)
        .ok_or(Ts4ScriptError::ArithmeticOverflow(
            "total TS4Script expanded bytes",
        ))?;

    if expanded > MAX_TOTAL_EXPANDED_BYTES {
        return Err(Ts4ScriptError::ExpandedBudgetExceeded {
            expanded,
            maximum: MAX_TOTAL_EXPANDED_BYTES,
        });
    }

    Ok(expanded)
}

fn validate_entry_path(name: &str) -> Result<(), Ts4ScriptError> {
    if name.as_bytes().len() > MAX_ENTRY_NAME_BYTES {
        return Err(Ts4ScriptError::EntryNameTooLong {
            name: name.to_string(),
            maximum: MAX_ENTRY_NAME_BYTES,
        });
    }

    if name.contains('\0')
        || name.starts_with('/')
        || name.starts_with('\\')
        || has_windows_drive_prefix(name)
    {
        return Err(Ts4ScriptError::UnsafePath {
            name: name.to_string(),
        });
    }

    let mut depth = 0_usize;
    for component in name.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." => {
                return Err(Ts4ScriptError::UnsafePath {
                    name: name.to_string(),
                });
            }
            _ => {
                depth += 1;
            }
        }
    }

    if depth > MAX_PATH_DEPTH {
        return Err(Ts4ScriptError::PathTooDeep {
            name: name.to_string(),
            depth,
            maximum: MAX_PATH_DEPTH,
        });
    }

    let path = Path::new(name);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(Ts4ScriptError::UnsafePath {
            name: name.to_string(),
        });
    }

    Ok(())
}

fn has_windows_drive_prefix(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn validate_entry_limits(
    name: &str,
    compressed_size: u64,
    expanded_size: u64,
) -> Result<(), Ts4ScriptError> {
    if expanded_size > MAX_ENTRY_EXPANDED_BYTES {
        return Err(Ts4ScriptError::EntryTooLarge {
            name: name.to_string(),
            expanded: expanded_size,
            maximum: MAX_ENTRY_EXPANDED_BYTES,
        });
    }

    if expanded_size == 0 {
        return Ok(());
    }

    if compressed_size == 0 {
        return Err(Ts4ScriptError::CompressionRatioExceeded {
            name: name.to_string(),
            compressed: compressed_size,
            expanded: expanded_size,
            maximum_ratio: MAX_COMPRESSION_RATIO,
        });
    }

    let maximum_expanded = u128::from(compressed_size)
        .checked_mul(u128::from(MAX_COMPRESSION_RATIO))
        .ok_or(Ts4ScriptError::ArithmeticOverflow(
            "TS4Script compression-ratio limit",
        ))?;

    if u128::from(expanded_size) > maximum_expanded {
        return Err(Ts4ScriptError::CompressionRatioExceeded {
            name: name.to_string(),
            compressed: compressed_size,
            expanded: expanded_size,
            maximum_ratio: MAX_COMPRESSION_RATIO,
        });
    }

    Ok(())
}

fn python_identity(name: &str) -> Option<(String, Option<String>)> {
    let normalized = name.replace('\\', "/");
    let mut parts = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_string)
        .collect::<Vec<_>>();

    let filename = parts.pop()?;
    let lower = filename.to_ascii_lowercase();
    let stem = if lower.ends_with(".pyc") {
        &filename[..filename.len() - 4]
    } else if lower.ends_with(".py") {
        &filename[..filename.len() - 3]
    } else {
        return None;
    };

    if parts.last().is_some_and(|part| part == "__pycache__") {
        parts.pop();
    }

    let normalized_stem = stem
        .split(".cpython-")
        .next()
        .unwrap_or(stem)
        .trim_end_matches(".opt-1")
        .trim_end_matches(".opt-2");

    if normalized_stem == "__init__" {
        if parts.is_empty() {
            return None;
        }
        let package = parts.join(".");
        return Some((package.clone(), Some(package)));
    }

    if normalized_stem.is_empty() {
        return None;
    }

    parts.push(normalized_stem.to_string());
    let module = parts.join(".");
    let package = if parts.len() > 1 {
        Some(parts[..parts.len() - 1].join("."))
    } else {
        None
    };

    Some((module, package.filter(|value| !value.is_empty())))
}

fn is_metadata_candidate(name: &str) -> bool {
    let extension = PathBuf::from(name.replace('\\', "/"))
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    matches!(
        extension.as_deref(),
        Some("json" | "txt" | "ini" | "cfg" | "toml" | "yaml" | "yml")
    )
}

fn extract_version_hints(text: &str) -> Vec<String> {
    let mut values = BTreeSet::new();

    for line in text.lines().take(512) {
        let lowercase = line.to_ascii_lowercase();
        let Some(version_offset) = lowercase.find("version") else {
            continue;
        };

        let tail = &line[version_offset + "version".len()..];
        let trimmed = tail.trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, ':' | '=' | '"' | '\'' | '_' | '-')
        });

        let token = trimmed
            .trim_start_matches(['v', 'V'])
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
            })
            .take(64)
            .collect::<String>();

        if token.chars().any(|character| character.is_ascii_digit())
            && token.contains('.')
            && token.len() <= 64
        {
            values.insert(token);
        }
    }

    values.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Write},
        panic::{catch_unwind, AssertUnwindSafe},
    };
    use tempfile::TempDir;
    use zip::{
        write::{SimpleFileOptions, ZipWriter},
        CompressionMethod,
    };

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::DEFLATE);

        for (name, content) in entries {
            writer.start_file(*name, options).expect("start ZIP entry");
            writer.write_all(content).expect("write ZIP entry");
        }

        writer.finish().expect("finish ZIP fixture").into_inner()
    }

    fn inspect_bytes(bytes: &[u8]) -> Result<ScriptArchiveMetadata, Ts4ScriptError> {
        inspect_reader(Cursor::new(bytes))
    }

    #[test]
    fn representative_archive_exposes_stable_modules_packages_and_version() {
        let bytes = archive(&[
            ("example_mod/__init__.pyc", b"compiled-init"),
            ("example_mod/core.pyc", b"compiled-core"),
            (
                "manifest.json",
                br#"{ "name": "Example Mod", "version": "2.4.1" }"#,
            ),
        ]);

        let metadata = inspect_bytes(&bytes).expect("inspect representative TS4Script");

        assert_eq!(
            metadata.module_names,
            vec!["example_mod", "example_mod.core"]
        );
        assert_eq!(metadata.package_names, vec!["example_mod"]);
        assert_eq!(
            metadata.version_hints,
            vec![VersionHint {
                source_entry: "manifest.json".to_string(),
                value: "2.4.1".to_string(),
            }]
        );
        assert_eq!(metadata.entries.len(), 3);
        assert!(metadata.metadata_bytes_read > 0);
    }

    #[test]
    fn pycache_module_names_are_normalized() {
        let bytes = archive(&[(
            "creator/pkg/__pycache__/feature.cpython-37.pyc",
            b"compiled",
        )]);

        let metadata = inspect_bytes(&bytes).expect("inspect pycache archive");

        assert_eq!(
            metadata.module_names,
            vec!["creator.pkg.feature".to_string()]
        );
        assert_eq!(metadata.package_names, vec!["creator.pkg".to_string()]);
    }

    #[test]
    fn parent_traversal_absolute_and_drive_paths_are_rejected() {
        for path in [
            "../escape.pyc",
            "safe/../../escape.pyc",
            "/absolute.pyc",
            r"\\server\share\evil.pyc",
            r"C:\Windows\evil.pyc",
        ] {
            assert!(
                matches!(
                    validate_entry_path(path),
                    Err(Ts4ScriptError::UnsafePath { .. })
                ),
                "path should be rejected: {path}"
            );
        }
    }

    #[test]
    fn full_archive_rejects_parent_traversal_entry() {
        let bytes = archive(&[("../escape.pyc", b"compiled")]);

        assert!(matches!(
            inspect_bytes(&bytes),
            Err(Ts4ScriptError::UnsafePath { .. })
        ));
    }

    #[test]
    fn entry_count_and_total_expanded_budgets_are_enforced() {
        assert!(matches!(
            validate_entry_count(MAX_ARCHIVE_ENTRIES + 1),
            Err(Ts4ScriptError::TooManyEntries { .. })
        ));

        assert_eq!(
            add_expanded_budget(MAX_TOTAL_EXPANDED_BYTES - 1, 1)
                .expect("exact expanded-data budget"),
            MAX_TOTAL_EXPANDED_BYTES
        );

        assert!(matches!(
            add_expanded_budget(MAX_TOTAL_EXPANDED_BYTES, 1),
            Err(Ts4ScriptError::ExpandedBudgetExceeded { .. })
        ));
    }

    #[test]
    fn excessive_path_depth_is_rejected() {
        let path = (0..=MAX_PATH_DEPTH)
            .map(|index| format!("p{index}"))
            .collect::<Vec<_>>()
            .join("/")
            + "/module.pyc";

        assert!(matches!(
            validate_entry_path(&path),
            Err(Ts4ScriptError::PathTooDeep { .. })
        ));
    }

    #[test]
    fn archive_bomb_limits_reject_large_entries_and_ratios() {
        assert!(matches!(
            validate_entry_limits("huge.pyc", 1024, MAX_ENTRY_EXPANDED_BYTES + 1),
            Err(Ts4ScriptError::EntryTooLarge { .. })
        ));

        assert!(matches!(
            validate_entry_limits("ratio.pyc", 1024, 1024 * (MAX_COMPRESSION_RATIO + 1)),
            Err(Ts4ScriptError::CompressionRatioExceeded { .. })
        ));

        assert!(matches!(
            validate_entry_limits("zero-compressed.pyc", 0, 1),
            Err(Ts4ScriptError::CompressionRatioExceeded { .. })
        ));
    }

    #[test]
    fn metadata_reads_are_bounded_and_non_metadata_payloads_remain_unread() {
        let large_binary = vec![0x7f; 512 * 1024];
        let bytes = archive(&[
            ("payload.pyc", &large_binary),
            ("version.txt", b"Version = 7.8.9"),
        ]);

        let metadata = inspect_bytes(&bytes).expect("inspect bounded metadata archive");

        assert_eq!(metadata.version_hints[0].value, "7.8.9");
        assert!(metadata.metadata_bytes_read <= MAX_TOTAL_METADATA_BYTES);
        assert!(metadata.metadata_bytes_read < large_binary.len() as u64);
    }

    #[test]
    fn malformed_archive_fails_without_mutating_source() {
        let temp = TempDir::new().expect("create TS4Script temp directory");
        let path = temp.path().join("broken.ts4script");
        let bytes = b"this is not a zip archive".to_vec();
        std::fs::write(&path, &bytes).expect("write malformed fixture");

        let result = inspect_path(&path);

        assert!(matches!(result, Err(Ts4ScriptError::Zip(_))));
        assert_eq!(std::fs::read(&path).expect("read malformed source"), bytes);
    }

    #[test]
    fn inspector_opens_archive_read_only_and_never_executes_python() {
        let temp = TempDir::new().expect("create TS4Script temp directory");
        let path = temp.path().join("readonly.ts4script");
        let python_source = b"raise RuntimeError('this must never execute')\n";
        let bytes = archive(&[
            ("danger.py", python_source),
            ("metadata.txt", b"version = 1.2.3"),
        ]);
        std::fs::write(&path, &bytes).expect("write TS4Script fixture");

        let mut permissions = std::fs::metadata(&path)
            .expect("fixture metadata")
            .permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).expect("set read-only fixture");

        let metadata = inspect_path(&path).expect("inspect read-only TS4Script");

        assert_eq!(metadata.module_names, vec!["danger"]);
        assert_eq!(
            std::fs::read(&path).expect("read archive after inspection"),
            bytes
        );
    }

    #[test]
    fn deterministic_mutation_harness_never_panics() {
        let seed = archive(&[
            ("example/core.pyc", b"compiled"),
            ("manifest.json", br#"{ "version": "1.2.3" }"#),
        ]);
        let mut state = 0xa341_316c_u32;

        for iteration in 0..1_024_u32 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let mut candidate = seed.clone();

            if iteration % 3 == 0 {
                let new_len = (state as usize) % (candidate.len() + 1);
                candidate.truncate(new_len);
            } else if !candidate.is_empty() {
                let offset = (state as usize) % candidate.len();
                candidate[offset] ^= (state >> 24) as u8 | 1;
            }

            let result = catch_unwind(AssertUnwindSafe(|| {
                let _ = inspect_bytes(&candidate);
            }));
            assert!(
                result.is_ok(),
                "TS4Script inspector panicked for mutation {iteration}"
            );
        }
    }
}

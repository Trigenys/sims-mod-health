use std::{
    fs,
    fs::File,
    io::{self, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

use super::super::GameVersion;
use super::{
    EvidenceConfidence, GameBuildEvidence, GameVersionProbe, SentinelFingerprint,
    VersionEvidenceKind,
};

const SENTINELS: &[&str] = &[
    "Game/Bin/Default.ini",
    "Game/Bin/TS4_x64.exe",
    "Delta/EP01/Version.ini",
];

pub(super) struct LocalVersionProbe;

impl GameVersionProbe for LocalVersionProbe {
    fn probe(&self, install_root: &Path) -> GameBuildEvidence {
        let default_ini = install_root.join("Game").join("Bin").join("Default.ini");
        let sentinels = fingerprint_sentinels(install_root);

        match fs::read_to_string(&default_ini) {
            Ok(content) => match parse_default_ini_game_version(&content) {
                Ok(version) => GameBuildEvidence {
                    version: Some(version),
                    evidence_kind: VersionEvidenceKind::DefaultIni,
                    confidence: EvidenceConfidence::Definitive,
                    sentinels,
                    detail: "Version parsed from Game/Bin/Default.ini.".to_string(),
                },
                Err(reason) if !sentinels.is_empty() => GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::SentinelFingerprint,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail: format!(
                        "Default.ini was present but unusable ({reason}); sentinel fingerprints were recorded without guessing a version."
                    ),
                },
                Err(reason) => GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::Unknown,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail: format!(
                        "Default.ini was present but unusable ({reason}) and no sentinel fingerprint was available."
                    ),
                },
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound && !sentinels.is_empty() => {
                GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::SentinelFingerprint,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail:
                        "No readable local version string; sentinel fingerprints were recorded for later manifest resolution."
                            .to_string(),
                }
            }
            Err(error) => GameBuildEvidence {
                version: None,
                evidence_kind: VersionEvidenceKind::Unknown,
                confidence: EvidenceConfidence::Unknown,
                sentinels,
                detail: format!("Unable to resolve the local game version: {error}"),
            },
        }
    }
}

fn parse_default_ini_game_version(content: &str) -> Result<GameVersion, String> {
    let value = content
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            if key.trim().eq_ignore_ascii_case("gameversion") {
                Some(value.trim())
            } else {
                None
            }
        })
        .ok_or_else(|| "gameversion key is missing".to_string())?;

    parse_version_value(value)
}

fn parse_version_value(value: &str) -> Result<GameVersion, String> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("gameversion must contain exactly four numeric components".to_string());
    }

    let mut numbers = [0_u32; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return Err("gameversion contains a non-numeric component".to_string());
        }
        numbers[index] = part
            .parse::<u32>()
            .map_err(|_| "gameversion component is outside the supported range".to_string())?;
    }

    Ok(GameVersion {
        major: numbers[0],
        minor: numbers[1],
        patch: numbers[2],
        build: numbers[3],
        normalized: format!(
            "{}.{}.{}.{}",
            numbers[0], numbers[1], numbers[2], numbers[3]
        ),
    })
}

fn fingerprint_sentinels(install_root: &Path) -> Vec<SentinelFingerprint> {
    SENTINELS
        .iter()
        .filter_map(|relative| {
            let path = relative
                .split('/')
                .fold(install_root.to_path_buf(), |current, part| {
                    current.join(part)
                });
            fingerprint_file(&path)
                .ok()
                .map(|(sha256, size_bytes)| SentinelFingerprint {
                    relative_path: (*relative).to_string(),
                    sha256,
                    size_bytes,
                })
        })
        .collect()
}

fn fingerprint_file(path: &Path) -> io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }

    Ok((format!("{:x}", hasher.finalize()), total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_game_root() -> TempDir {
        let temp = TempDir::new().expect("create temp root");
        fs::create_dir_all(temp.path().join("Game").join("Bin")).expect("create bin");
        fs::write(
            temp.path().join("Game").join("Bin").join("TS4_x64.exe"),
            b"fixture",
        )
        .expect("write sentinel");
        temp
    }

    #[test]
    fn parses_default_ini_version_without_guessing() {
        let version = parse_default_ini_game_version(
            "[Version]\ngameversion = 1.128.90.1030\ncodeversion = 1.0\n",
        )
        .expect("version");

        assert_eq!(version.normalized, "1.128.90.1030");
    }

    #[test]
    fn malformed_default_ini_returns_error() {
        let error = parse_default_ini_game_version("[Version]\ngameversion = 1.bad.90.1030")
            .expect_err("malformed version");
        assert!(error.contains("non-numeric"));
    }

    #[test]
    fn sentinel_fallback_records_hashes_without_inventing_version() {
        let game = create_game_root();
        let evidence = LocalVersionProbe.probe(game.path());

        assert!(evidence.version.is_none());
        assert_eq!(
            evidence.evidence_kind,
            VersionEvidenceKind::SentinelFingerprint
        );
        assert_eq!(evidence.confidence, EvidenceConfidence::Unknown);
        assert!(!evidence.sentinels.is_empty());
    }
}

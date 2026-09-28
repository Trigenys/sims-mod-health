use std::{fs, io, path::Path};

use super::{InstalledPackObservation, PackInventoryProbe, PackKind, PackLocalState};

const MAX_PACK_WALK_ENTRIES: usize = 2_000_000;

pub(super) struct LocalPackProbe;

impl PackInventoryProbe for LocalPackProbe {
    fn probe(&self, install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation> {
        inventory_packs(install_root, observed_at)
    }
}

fn inventory_packs(install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation> {
    let Ok(entries) = fs::read_dir(install_root) else {
        return Vec::new();
    };

    let mut packs = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let Some(pack_kind) = pack_kind_from_code(&name) else {
            continue;
        };

        let normalized = name.to_ascii_uppercase();
        let (local_state, size_bytes, marker_count) = match measure_pack(&entry.path()) {
            Ok((bytes, markers)) if bytes > 0 => (PackLocalState::Installed, Some(bytes), markers),
            Ok((bytes, markers)) => (PackLocalState::Partial, Some(bytes), markers),
            Err(_) => (PackLocalState::Unknown, None, 0),
        };

        packs.push(InstalledPackObservation {
            pack_code: normalized,
            pack_kind,
            local_state,
            size_bytes,
            marker_count,
            observed_at: observed_at.to_string(),
        });
    }

    packs.sort_by(|left, right| left.pack_code.cmp(&right.pack_code));
    packs
}

fn measure_pack(root: &Path) -> io::Result<(i64, i64)> {
    let mut stack = vec![root.to_path_buf()];
    let mut total_bytes = 0_i64;
    let mut marker_count = 0_i64;
    let mut entries_seen = 0_usize;

    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries_seen += 1;
            if entries_seen > MAX_PACK_WALK_ENTRIES {
                return Err(io::Error::other(
                    "pack inventory exceeded bounded entry limit",
                ));
            }

            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }

            if file_type.is_file() {
                let metadata = entry.metadata()?;
                let file_size = i64::try_from(metadata.len())
                    .map_err(|_| io::Error::other("pack file size exceeds SQLite integer range"))?;
                total_bytes = total_bytes
                    .checked_add(file_size)
                    .ok_or_else(|| io::Error::other("pack size exceeds SQLite integer range"))?;
                marker_count = marker_count.checked_add(1).ok_or_else(|| {
                    io::Error::other("pack marker count exceeds SQLite integer range")
                })?;
            }
        }
    }

    Ok((total_bytes, marker_count))
}

fn pack_kind_from_code(name: &str) -> Option<PackKind> {
    let upper = name.to_ascii_uppercase();
    if !upper.is_ascii() {
        return None;
    }

    let (prefix, digits) = if upper.starts_with("KIT") {
        ("KIT", &upper[3..])
    } else if upper.len() >= 3 {
        (&upper[..2], &upper[2..])
    } else {
        return None;
    };

    if digits.len() < 2 || digits.len() > 3 || !digits.chars().all(|value| value.is_ascii_digit()) {
        return None;
    }

    match prefix {
        "EP" => Some(PackKind::Expansion),
        "GP" => Some(PackKind::Game),
        "SP" => Some(PackKind::StuffOrKit),
        "FP" => Some(PackKind::Free),
        "KIT" => Some(PackKind::Kit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn pack_codes_are_classified_conservatively() {
        assert_eq!(pack_kind_from_code("EP01"), Some(PackKind::Expansion));
        assert_eq!(pack_kind_from_code("GP12"), Some(PackKind::Game));
        assert_eq!(pack_kind_from_code("SP81"), Some(PackKind::StuffOrKit));
        assert_eq!(pack_kind_from_code("FP01"), Some(PackKind::Free));
        assert_eq!(pack_kind_from_code("KIT001"), Some(PackKind::Kit));
        assert_eq!(pack_kind_from_code("Mods"), None);
        assert_eq!(pack_kind_from_code("EP1"), None);
    }

    #[test]
    fn empty_pack_is_partial_and_nonempty_pack_is_installed() {
        let game = TempDir::new().expect("temp");
        fs::create_dir_all(game.path().join("EP01")).expect("empty pack");
        fs::create_dir_all(game.path().join("GP01")).expect("pack");
        fs::write(
            game.path().join("GP01").join("ClientFullBuild0.package"),
            b"pack",
        )
        .expect("pack payload");

        let packs = inventory_packs(game.path(), "test");
        let ep = packs
            .iter()
            .find(|pack| pack.pack_code == "EP01")
            .expect("ep");
        let gp = packs
            .iter()
            .find(|pack| pack.pack_code == "GP01")
            .expect("gp");

        assert_eq!(ep.local_state, PackLocalState::Partial);
        assert_eq!(ep.size_bytes, Some(0));
        assert_eq!(gp.local_state, PackLocalState::Installed);
        assert_eq!(gp.size_bytes, Some(4));
    }
}

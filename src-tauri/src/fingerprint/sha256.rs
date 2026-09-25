use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

const HASH_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) fn sha256_file<F>(path: &Path, mut is_cancelled: F) -> Result<Option<String>, io::Error>
where
    F: FnMut() -> bool,
{
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_SIZE];

    loop {
        if is_cancelled() {
            return Ok(None);
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(Some(format!("{:x}", hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn identical_bytes_hash_identically_across_paths() {
        let temp = TempDir::new().expect("create hash temp directory");
        let left = temp.path().join("A/one.package");
        let right = temp.path().join("B/two.package");

        std::fs::create_dir_all(left.parent().expect("left parent")).expect("create left parent");
        std::fs::create_dir_all(right.parent().expect("right parent"))
            .expect("create right parent");
        std::fs::write(&left, b"same artifact bytes").expect("write left fixture");
        std::fs::write(&right, b"same artifact bytes").expect("write right fixture");

        let left_hash = sha256_file(&left, || false)
            .expect("hash left")
            .expect("left hash not cancelled");
        let right_hash = sha256_file(&right, || false)
            .expect("hash right")
            .expect("right hash not cancelled");

        assert_eq!(left_hash, right_hash);
    }

    #[test]
    fn cancellation_stops_streaming_hash() {
        let temp = TempDir::new().expect("create hash temp directory");
        let path = temp.path().join("large.package");
        std::fs::write(&path, vec![0x5a; 256 * 1024]).expect("write hash fixture");

        let result = sha256_file(&path, || true).expect("cancelled hash");

        assert!(result.is_none());
    }
}

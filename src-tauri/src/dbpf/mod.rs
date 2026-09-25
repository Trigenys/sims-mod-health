use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

use serde::Serialize;

const HEADER_SIZE: u64 = 96;
const MAGIC: &[u8; 4] = b"DBPF";
const SUPPORTED_MAJOR_VERSION: u32 = 2;
const KNOWN_INDEX_FLAGS: u32 = 0x07;
const CONST_TYPE: u32 = 0x01;
const CONST_GROUP: u32 = 0x02;
const CONST_INSTANCE_HIGH: u32 = 0x04;

pub(crate) const MAX_RESOURCE_ENTRIES: u32 = 1_000_000;
pub(crate) const MAX_INDEX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceKey {
    pub(crate) resource_type: u32,
    pub(crate) group: u32,
    pub(crate) instance: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressionMetadata {
    pub(crate) compression_type: u16,
    pub(crate) committed: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceEntry {
    pub(crate) key: ResourceKey,
    pub(crate) offset: u32,
    pub(crate) stored_size: u32,
    pub(crate) decompressed_size: u32,
    pub(crate) compression: Option<CompressionMetadata>,
    pub(crate) deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageMetadata {
    pub(crate) file_version_major: u32,
    pub(crate) file_version_minor: u32,
    pub(crate) index_offset: u64,
    pub(crate) index_size: u32,
    pub(crate) entries: Vec<ResourceEntry>,
}

impl PackageMetadata {
    pub(crate) fn resource_keys(&self) -> impl Iterator<Item = ResourceKey> + '_ {
        self.entries
            .iter()
            .filter(|entry| !entry.deleted)
            .map(|entry| entry.key)
    }
}

#[derive(Debug)]
pub(crate) enum DbpfError {
    Io(io::Error),
    FileTooSmall { actual: u64, minimum: u64 },
    InvalidMagic,
    UnsupportedVersion { major: u32, minor: u32 },
    TooManyResources { count: u32, maximum: u32 },
    IndexTooLarge { size: u64, maximum: u64 },
    MissingIndex { count: u32 },
    UnsupportedIndexFlags { flags: u32 },
    ArithmeticOverflow(&'static str),
    IndexOutOfBounds {
        offset: u64,
        size: u64,
        file_size: u64,
    },
    TruncatedIndex {
        needed: u64,
        available: u64,
    },
    ResourceOutOfBounds {
        key: ResourceKey,
        offset: u64,
        size: u64,
        file_size: u64,
    },
}

impl Display for DbpfError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "DBPF I/O error: {error}"),
            Self::FileTooSmall { actual, minimum } => {
                write!(formatter, "DBPF file is {actual} bytes; minimum is {minimum}")
            }
            Self::InvalidMagic => write!(formatter, "file does not start with DBPF magic"),
            Self::UnsupportedVersion { major, minor } => {
                write!(formatter, "unsupported DBPF version {major}.{minor}")
            }
            Self::TooManyResources { count, maximum } => write!(
                formatter,
                "DBPF declares {count} resources; maximum supported is {maximum}"
            ),
            Self::IndexTooLarge { size, maximum } => write!(
                formatter,
                "DBPF index is {size} bytes; maximum supported is {maximum}"
            ),
            Self::MissingIndex { count } => {
                write!(formatter, "DBPF declares {count} resources but no index offset")
            }
            Self::UnsupportedIndexFlags { flags } => {
                write!(formatter, "DBPF index uses unsupported flags 0x{flags:08x}")
            }
            Self::ArithmeticOverflow(context) => {
                write!(formatter, "integer overflow while validating {context}")
            }
            Self::IndexOutOfBounds {
                offset,
                size,
                file_size,
            } => write!(
                formatter,
                "DBPF index range {offset}..{} exceeds file size {file_size}",
                offset.saturating_add(*size)
            ),
            Self::TruncatedIndex { needed, available } => write!(
                formatter,
                "DBPF index needs {needed} bytes but only {available} remain"
            ),
            Self::ResourceOutOfBounds {
                key,
                offset,
                size,
                file_size,
            } => write!(
                formatter,
                "resource {:08x}:{:08x}:{:016x} range {offset}..{} exceeds file size {file_size}",
                key.resource_type,
                key.group,
                key.instance,
                offset.saturating_add(*size)
            ),
        }
    }
}

impl std::error::Error for DbpfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for DbpfError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub(crate) fn parse_path(path: &Path) -> Result<PackageMetadata, DbpfError> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    parse_reader(&mut file, file_size)
}

fn parse_reader<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
) -> Result<PackageMetadata, DbpfError> {
    if file_size < HEADER_SIZE {
        return Err(DbpfError::FileTooSmall {
            actual: file_size,
            minimum: HEADER_SIZE,
        });
    }

    let mut header = [0_u8; HEADER_SIZE as usize];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut header)?;

    if &header[0..4] != MAGIC {
        return Err(DbpfError::InvalidMagic);
    }

    let major = u32_at(&header, 4);
    let minor = u32_at(&header, 8);
    if major != SUPPORTED_MAJOR_VERSION {
        return Err(DbpfError::UnsupportedVersion { major, minor });
    }

    let index_count = u32_at(&header, 36);
    if index_count > MAX_RESOURCE_ENTRIES {
        return Err(DbpfError::TooManyResources {
            count: index_count,
            maximum: MAX_RESOURCE_ENTRIES,
        });
    }

    let index_offset_low = u64::from(u32_at(&header, 40));
    let index_size = u32_at(&header, 44);
    let index_offset_high = u64::from(u32_at(&header, 64));
    let index_offset = if index_offset_high != 0 {
        index_offset_high
    } else {
        index_offset_low
    };

    if u64::from(index_size) > MAX_INDEX_BYTES {
        return Err(DbpfError::IndexTooLarge {
            size: u64::from(index_size),
            maximum: MAX_INDEX_BYTES,
        });
    }

    if index_count == 0 {
        return Ok(PackageMetadata {
            file_version_major: major,
            file_version_minor: minor,
            index_offset,
            index_size,
            entries: Vec::new(),
        });
    }

    if index_offset == 0 {
        return Err(DbpfError::MissingIndex { count: index_count });
    }

    let index_end = index_offset
        .checked_add(u64::from(index_size))
        .ok_or(DbpfError::ArithmeticOverflow("index range"))?;
    if index_end > file_size {
        return Err(DbpfError::IndexOutOfBounds {
            offset: index_offset,
            size: u64::from(index_size),
            file_size,
        });
    }

    reader.seek(SeekFrom::Start(index_offset))?;
    let mut cursor = IndexCursor::new(reader, u64::from(index_size));

    let flags = cursor.read_u32()?;
    if flags & !KNOWN_INDEX_FLAGS != 0 {
        return Err(DbpfError::UnsupportedIndexFlags { flags });
    }

    let constant_type = if flags & CONST_TYPE != 0 {
        Some(cursor.read_u32()?)
    } else {
        None
    };
    let constant_group = if flags & CONST_GROUP != 0 {
        Some(cursor.read_u32()?)
    } else {
        None
    };
    let constant_instance_high = if flags & CONST_INSTANCE_HIGH != 0 {
        Some(cursor.read_u32()?)
    } else {
        None
    };

    let per_entry_minimum = 16_u64
        .checked_add(if constant_type.is_none() { 4 } else { 0 })
        .and_then(|value| value.checked_add(if constant_group.is_none() { 4 } else { 0 }))
        .and_then(|value| {
            value.checked_add(if constant_instance_high.is_none() {
                4
            } else {
                0
            })
        })
        .ok_or(DbpfError::ArithmeticOverflow("minimum index entry size"))?;

    let required_minimum = u64::from(index_count)
        .checked_mul(per_entry_minimum)
        .and_then(|value| value.checked_add(cursor.consumed()))
        .ok_or(DbpfError::ArithmeticOverflow("minimum index size"))?;
    if required_minimum > u64::from(index_size) {
        return Err(DbpfError::TruncatedIndex {
            needed: required_minimum,
            available: u64::from(index_size),
        });
    }

    let capacity = usize::try_from(index_count)
        .map_err(|_| DbpfError::ArithmeticOverflow("resource vector capacity"))?;
    let mut entries = Vec::with_capacity(capacity);

    for _ in 0..index_count {
        let resource_type = match constant_type {
            Some(value) => value,
            None => cursor.read_u32()?,
        };
        let group = match constant_group {
            Some(value) => value,
            None => cursor.read_u32()?,
        };
        let instance_high = match constant_instance_high {
            Some(value) => value,
            None => cursor.read_u32()?,
        };

        let instance_low = cursor.read_u32()?;
        let offset = cursor.read_u32()?;
        let size_field = cursor.read_u32()?;
        let decompressed_size = cursor.read_u32()?;

        let has_extended_compression = size_field & 0x8000_0000 != 0;
        let stored_size = size_field & 0x7fff_ffff;
        let compression = if has_extended_compression {
            Some(CompressionMetadata {
                compression_type: cursor.read_u16()?,
                committed: cursor.read_u16()?,
            })
        } else {
            None
        };

        let key = ResourceKey {
            resource_type,
            group,
            instance: (u64::from(instance_high) << 32) | u64::from(instance_low),
        };

        let deleted = offset == u32::MAX || (stored_size == 1 && decompressed_size == u32::MAX);
        if !deleted {
            validate_resource_span(key, offset, stored_size, file_size)?;
        }

        entries.push(ResourceEntry {
            key,
            offset,
            stored_size,
            decompressed_size,
            compression,
            deleted,
        });
    }

    Ok(PackageMetadata {
        file_version_major: major,
        file_version_minor: minor,
        index_offset,
        index_size,
        entries,
    })
}

fn validate_resource_span(
    key: ResourceKey,
    offset: u32,
    stored_size: u32,
    file_size: u64,
) -> Result<(), DbpfError> {
    let offset = u64::from(offset);
    let size = u64::from(stored_size);
    let end = offset
        .checked_add(size)
        .ok_or(DbpfError::ArithmeticOverflow("resource range"))?;

    if end > file_size {
        return Err(DbpfError::ResourceOutOfBounds {
            key,
            offset,
            size,
            file_size,
        });
    }

    Ok(())
}

fn u32_at(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        buffer[offset..offset + 4]
            .try_into()
            .expect("header offsets are compile-time bounded"),
    )
}

struct IndexCursor<'a, R> {
    reader: &'a mut R,
    consumed: u64,
    limit: u64,
}

impl<'a, R: Read> IndexCursor<'a, R> {
    fn new(reader: &'a mut R, limit: u64) -> Self {
        Self {
            reader,
            consumed: 0,
            limit,
        }
    }

    fn consumed(&self) -> u64 {
        self.consumed
    }

    fn read_u16(&mut self) -> Result<u16, DbpfError> {
        let bytes = self.read_exact::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, DbpfError> {
        let bytes = self.read_exact::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DbpfError> {
        let needed = u64::try_from(N).expect("fixed read size fits u64");
        let after = self
            .consumed
            .checked_add(needed)
            .ok_or(DbpfError::ArithmeticOverflow("index cursor"))?;

        if after > self.limit {
            return Err(DbpfError::TruncatedIndex {
                needed: after,
                available: self.limit,
            });
        }

        let mut bytes = [0_u8; N];
        self.reader.read_exact(&mut bytes)?;
        self.consumed = after;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Cursor,
        panic::{catch_unwind, AssertUnwindSafe},
    };
    use tempfile::TempDir;

    const INDEX_OFFSET: usize = HEADER_SIZE as usize;

    #[derive(Clone, Copy)]
    struct FixtureEntry {
        key: ResourceKey,
        payload_size: u32,
        decompressed_size: u32,
        compression: Option<CompressionMetadata>,
        deleted: bool,
    }

    fn header(index_count: u32, index_size: u32) -> Vec<u8> {
        let mut bytes = vec![0_u8; HEADER_SIZE as usize];
        bytes[0..4].copy_from_slice(MAGIC);
        put_u32(&mut bytes, 4, 2);
        put_u32(&mut bytes, 8, 1);
        put_u32(&mut bytes, 36, index_count);
        put_u32(&mut bytes, 40, INDEX_OFFSET as u32);
        put_u32(&mut bytes, 44, index_size);
        bytes
    }

    fn package(flags: u32, entries: &[FixtureEntry]) -> Vec<u8> {
        let constant_type = if flags & CONST_TYPE != 0 {
            entries.first().map(|entry| entry.key.resource_type)
        } else {
            None
        };
        let constant_group = if flags & CONST_GROUP != 0 {
            entries.first().map(|entry| entry.key.group)
        } else {
            None
        };
        let constant_instance_high = if flags & CONST_INSTANCE_HIGH != 0 {
            entries
                .first()
                .map(|entry| (entry.key.instance >> 32) as u32)
        } else {
            None
        };

        let mut index = Vec::new();
        push_u32(&mut index, flags);
        if let Some(value) = constant_type {
            push_u32(&mut index, value);
        }
        if let Some(value) = constant_group {
            push_u32(&mut index, value);
        }
        if let Some(value) = constant_instance_high {
            push_u32(&mut index, value);
        }

        let mut payload_offset = HEADER_SIZE as u32;
        let payload_bytes = entries
            .iter()
            .filter(|entry| !entry.deleted)
            .map(|entry| entry.payload_size)
            .sum::<u32>();
        payload_offset += payload_bytes;

        for entry in entries {
            if constant_type.is_none() {
                push_u32(&mut index, entry.key.resource_type);
            }
            if constant_group.is_none() {
                push_u32(&mut index, entry.key.group);
            }
            if constant_instance_high.is_none() {
                push_u32(&mut index, (entry.key.instance >> 32) as u32);
            }

            push_u32(&mut index, entry.key.instance as u32);

            if entry.deleted {
                push_u32(&mut index, u32::MAX);
                push_u32(&mut index, 1);
                push_u32(&mut index, u32::MAX);
            } else {
                let offset = HEADER_SIZE as u32
                    + entries
                        .iter()
                        .take_while(|candidate| candidate.key != entry.key)
                        .filter(|candidate| !candidate.deleted)
                        .map(|candidate| candidate.payload_size)
                        .sum::<u32>();
                push_u32(&mut index, offset);

                let size_field = if entry.compression.is_some() {
                    entry.payload_size | 0x8000_0000
                } else {
                    entry.payload_size
                };
                push_u32(&mut index, size_field);
                push_u32(&mut index, entry.decompressed_size);

                if let Some(compression) = entry.compression {
                    push_u16(&mut index, compression.compression_type);
                    push_u16(&mut index, compression.committed);
                }
            }
        }

        let index_size = index.len() as u32;
        let mut bytes = header(entries.len() as u32, index_size);
        bytes.resize(INDEX_OFFSET, 0);

        for entry in entries.iter().filter(|entry| !entry.deleted) {
            bytes.extend(std::iter::repeat_n(0xa5, entry.payload_size as usize));
        }

        assert_eq!(bytes.len(), payload_offset as usize);
        bytes.extend(index);

        put_u32(&mut bytes, 40, payload_offset);
        bytes
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn parse_bytes(bytes: &[u8]) -> Result<PackageMetadata, DbpfError> {
        parse_reader(&mut Cursor::new(bytes), bytes.len() as u64)
    }

    #[test]
    fn parses_resource_keys_deterministically_without_reading_payloads() {
        let entries = [
            FixtureEntry {
                key: ResourceKey {
                    resource_type: 0x545a_6b4a,
                    group: 0,
                    instance: 0x0102_0304_0506_0708,
                },
                payload_size: 32,
                decompressed_size: 32,
                compression: None,
                deleted: false,
            },
            FixtureEntry {
                key: ResourceKey {
                    resource_type: 0x0166_1233,
                    group: 0x8000_0000,
                    instance: 0xf102_0304_1112_1314,
                },
                payload_size: 12,
                decompressed_size: 64,
                compression: Some(CompressionMetadata {
                    compression_type: 0x5a42,
                    committed: 1,
                }),
                deleted: false,
            },
        ];
        let bytes = package(0, &entries);

        let parsed = parse_bytes(&bytes).expect("parse representative DBPF");

        assert_eq!(parsed.file_version_major, 2);
        assert_eq!(parsed.file_version_minor, 1);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(
            parsed.resource_keys().collect::<Vec<_>>(),
            entries.iter().map(|entry| entry.key).collect::<Vec<_>>()
        );
        assert_eq!(parsed.entries[1].stored_size, 12);
        assert_eq!(parsed.entries[1].decompressed_size, 64);
        assert_eq!(
            parsed.entries[1].compression,
            Some(CompressionMetadata {
                compression_type: 0x5a42,
                committed: 1
            })
        );
    }

    #[test]
    fn parses_constant_index_fields_and_deleted_entries() {
        let entries = [
            FixtureEntry {
                key: ResourceKey {
                    resource_type: 0x2205_57da,
                    group: 0,
                    instance: 0x1234_5678_0000_0001,
                },
                payload_size: 4,
                decompressed_size: 4,
                compression: None,
                deleted: false,
            },
            FixtureEntry {
                key: ResourceKey {
                    resource_type: 0x2205_57da,
                    group: 0,
                    instance: 0x1234_5678_0000_0002,
                },
                payload_size: 0,
                decompressed_size: u32::MAX,
                compression: None,
                deleted: true,
            },
        ];
        let bytes = package(
            CONST_TYPE | CONST_GROUP | CONST_INSTANCE_HIGH,
            &entries,
        );

        let parsed = parse_bytes(&bytes).expect("parse constant-field index");

        assert_eq!(parsed.entries.len(), 2);
        assert!(!parsed.entries[0].deleted);
        assert!(parsed.entries[1].deleted);
        assert_eq!(parsed.resource_keys().count(), 1);
    }

    #[test]
    fn rejects_invalid_magic_and_unsupported_major_version() {
        let mut invalid_magic = header(0, 0);
        invalid_magic[0..4].copy_from_slice(b"NOPE");
        assert!(matches!(
            parse_bytes(&invalid_magic),
            Err(DbpfError::InvalidMagic)
        ));

        let mut unsupported = header(0, 0);
        put_u32(&mut unsupported, 4, 3);
        assert!(matches!(
            parse_bytes(&unsupported),
            Err(DbpfError::UnsupportedVersion {
                major: 3,
                minor: 1
            })
        ));
    }

    #[test]
    fn rejects_resource_count_and_index_size_above_limits() {
        let too_many = header(MAX_RESOURCE_ENTRIES + 1, 4);
        assert!(matches!(
            parse_bytes(&too_many),
            Err(DbpfError::TooManyResources { .. })
        ));

        let too_large = header(1, (MAX_INDEX_BYTES + 1) as u32);
        assert!(matches!(
            parse_reader(&mut Cursor::new(&too_large), u64::MAX),
            Err(DbpfError::IndexTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_out_of_range_index_and_truncated_index_entries() {
        let mut out_of_range = header(1, 64);
        put_u32(&mut out_of_range, 40, 90);
        assert!(matches!(
            parse_bytes(&out_of_range),
            Err(DbpfError::IndexOutOfBounds { .. })
        ));

        let mut truncated = header(1, 8);
        truncated.extend_from_slice(&0_u32.to_le_bytes());
        truncated.extend_from_slice(&0_u32.to_le_bytes());
        assert!(matches!(
            parse_bytes(&truncated),
            Err(DbpfError::TruncatedIndex { .. })
        ));
    }

    #[test]
    fn rejects_resource_ranges_past_end_of_file() {
        let entry = FixtureEntry {
            key: ResourceKey {
                resource_type: 1,
                group: 2,
                instance: 3,
            },
            payload_size: 8,
            decompressed_size: 8,
            compression: None,
            deleted: false,
        };
        let mut bytes = package(0, &[entry]);

        let index_offset = u32_at(&bytes, 40) as usize;
        let offset_field = index_offset + 4 + 12 + 4;
        put_u32(&mut bytes, offset_field, u32::MAX - 4);

        assert!(matches!(
            parse_bytes(&bytes),
            Err(DbpfError::ResourceOutOfBounds { .. })
        ));
    }

    #[test]
    fn parser_opens_read_only_package_without_mutating_source() {
        let temp = TempDir::new().expect("create DBPF temp directory");
        let path = temp.path().join("readonly.package");
        let bytes = package(
            0,
            &[FixtureEntry {
                key: ResourceKey {
                    resource_type: 1,
                    group: 2,
                    instance: 3,
                },
                payload_size: 4,
                decompressed_size: 4,
                compression: None,
                deleted: false,
            }],
        );
        std::fs::write(&path, &bytes).expect("write package fixture");

        let mut permissions = std::fs::metadata(&path)
            .expect("fixture metadata")
            .permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).expect("set read-only fixture");

        let parsed = parse_path(&path).expect("parse read-only package");
        assert_eq!(parsed.resource_keys().count(), 1);
        assert_eq!(
            std::fs::read(&path).expect("read package after parsing"),
            bytes
        );
    }

    #[test]
    fn deterministic_adversarial_mutation_harness_never_panics() {
        let seed_entry = FixtureEntry {
            key: ResourceKey {
                resource_type: 0x545a_6b4a,
                group: 0,
                instance: 0x1020_3040_5060_7080,
            },
            payload_size: 16,
            decompressed_size: 16,
            compression: None,
            deleted: false,
        };
        let seed = package(0, &[seed_entry]);
        let mut state = 0x6d2b_79f5_u32;

        for iteration in 0..2_048_u32 {
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
                let _ = parse_bytes(&candidate);
            }));
            assert!(
                result.is_ok(),
                "parser panicked for adversarial iteration {iteration}"
            );
        }
    }
}

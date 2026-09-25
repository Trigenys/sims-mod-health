ALTER TABLE local_files ADD COLUMN size_bytes INTEGER
    CHECK (size_bytes IS NULL OR size_bytes >= 0);

ALTER TABLE local_files ADD COLUMN modified_ns INTEGER
    CHECK (modified_ns IS NULL OR modified_ns >= 0);

ALTER TABLE local_files ADD COLUMN quick_fingerprint TEXT;

ALTER TABLE local_files ADD COLUMN hashed_at TEXT;

CREATE INDEX idx_local_files_incremental_cache
    ON local_files(installation_id, relative_path, size_bytes, modified_ns);

PRAGMA user_version = 2;

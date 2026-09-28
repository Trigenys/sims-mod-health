CREATE TABLE game_content_manifest_cache (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    manifest_json TEXT NOT NULL,
    source_identity TEXT NOT NULL,
    source_url TEXT,
    retrieved_at TEXT NOT NULL,
    expires_at TEXT,
    checksum_sha256 TEXT,
    cached_at TEXT NOT NULL
);

PRAGMA user_version = 8;

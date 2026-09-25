CREATE TABLE installations (
    id INTEGER PRIMARY KEY,
    game_root TEXT NOT NULL,
    mods_root TEXT NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('windows')),
    game_version TEXT,
    discovered_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    UNIQUE (game_root, mods_root)
);

CREATE TABLE scan_sessions (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    mode TEXT NOT NULL CHECK (mode IN ('incremental', 'full')),
    files_seen INTEGER NOT NULL DEFAULT 0 CHECK (files_seen >= 0),
    files_hashed INTEGER NOT NULL DEFAULT 0 CHECK (files_hashed >= 0),
    parse_failures INTEGER NOT NULL DEFAULT 0 CHECK (parse_failures >= 0),
    error_count INTEGER NOT NULL DEFAULT 0 CHECK (error_count >= 0),
    FOREIGN KEY (installation_id) REFERENCES installations(id) ON DELETE CASCADE
);

CREATE TABLE local_files (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    file_kind TEXT NOT NULL CHECK (file_kind IN ('package', 'ts4script', 'diagnostic', 'other')),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    parse_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (parse_status IN ('pending', 'parsed', 'unsupported', 'malformed', 'error')),
    first_seen_scan_id INTEGER,
    last_seen_scan_id INTEGER,
    FOREIGN KEY (installation_id) REFERENCES installations(id) ON DELETE CASCADE,
    FOREIGN KEY (first_seen_scan_id) REFERENCES scan_sessions(id) ON DELETE SET NULL,
    FOREIGN KEY (last_seen_scan_id) REFERENCES scan_sessions(id) ON DELETE SET NULL,
    UNIQUE (installation_id, relative_path)
);

CREATE TABLE local_artifacts (
    id INTEGER PRIMARY KEY,
    local_file_id INTEGER NOT NULL UNIQUE,
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN ('package', 'ts4script', 'archive', 'unknown')),
    resource_count INTEGER CHECK (resource_count IS NULL OR resource_count >= 0),
    archive_entry_count INTEGER CHECK (archive_entry_count IS NULL OR archive_entry_count >= 0),
    embedded_version TEXT,
    creator_hint TEXT,
    parser_version TEXT NOT NULL,
    inspected_at TEXT NOT NULL,
    FOREIGN KEY (local_file_id) REFERENCES local_files(id) ON DELETE CASCADE
);

CREATE TABLE fingerprints (
    id INTEGER PRIMARY KEY,
    local_file_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('sha256', 'curseforge', 'resource_signature', 'quick')),
    value TEXT NOT NULL,
    algorithm_version TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    FOREIGN KEY (local_file_id) REFERENCES local_files(id) ON DELETE CASCADE,
    UNIQUE (local_file_id, kind, value)
);

CREATE TABLE conflict_observations (
    id INTEGER PRIMARY KEY,
    scan_session_id INTEGER NOT NULL,
    left_file_id INTEGER NOT NULL,
    right_file_id INTEGER,
    kind TEXT NOT NULL,
    evidence_key TEXT,
    evidence_json TEXT NOT NULL DEFAULT '{}',
    observed_at TEXT NOT NULL,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (left_file_id) REFERENCES local_files(id) ON DELETE CASCADE,
    FOREIGN KEY (right_file_id) REFERENCES local_files(id) ON DELETE CASCADE,
    CHECK (right_file_id IS NULL OR left_file_id <> right_file_id)
);

CREATE TABLE diagnostics (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    scan_session_id INTEGER,
    source_kind TEXT NOT NULL,
    relative_path TEXT,
    content_hash TEXT,
    parse_status TEXT NOT NULL CHECK (parse_status IN ('pending', 'parsed', 'unsupported', 'malformed', 'error')),
    summary_json TEXT NOT NULL DEFAULT '{}',
    observed_at TEXT NOT NULL,
    parsed_at TEXT,
    FOREIGN KEY (installation_id) REFERENCES installations(id) ON DELETE CASCADE,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE SET NULL
);

CREATE TABLE restore_points (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    reason TEXT NOT NULL,
    location TEXT NOT NULL,
    manifest_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK (status IN ('creating', 'ready', 'restored', 'failed', 'deleted')),
    FOREIGN KEY (installation_id) REFERENCES installations(id) ON DELETE CASCADE
);

CREATE TABLE preferences (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_scan_sessions_installation_started
    ON scan_sessions(installation_id, started_at DESC);

CREATE INDEX idx_local_files_installation_kind
    ON local_files(installation_id, file_kind);

CREATE INDEX idx_fingerprints_kind_value
    ON fingerprints(kind, value);

CREATE INDEX idx_conflicts_scan
    ON conflict_observations(scan_session_id);

CREATE INDEX idx_diagnostics_installation_observed
    ON diagnostics(installation_id, observed_at DESC);

CREATE INDEX idx_restore_points_installation_created
    ON restore_points(installation_id, created_at DESC);

PRAGMA user_version = 1;

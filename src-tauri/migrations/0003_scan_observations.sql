CREATE TABLE scan_observations (
    id INTEGER PRIMARY KEY,
    scan_session_id INTEGER NOT NULL,
    relative_path TEXT,
    kind TEXT NOT NULL,
    detail TEXT NOT NULL,
    observed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_scan_observations_session_kind
    ON scan_observations(scan_session_id, kind);

PRAGMA user_version = 3;

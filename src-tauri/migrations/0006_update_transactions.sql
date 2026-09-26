CREATE TABLE update_transactions (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    restore_point_id INTEGER NOT NULL UNIQUE,
    target_relative_path TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_url TEXT NOT NULL,
    current_release_id TEXT NOT NULL,
    replacement_release_id TEXT NOT NULL,
    expected_sha256 TEXT,
    observed_sha256 TEXT,
    original_sha256 TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN (
            'prepared',
            'downloading',
            'staged',
            'dependency_checked',
            'archiving',
            'archived',
            'installed',
            'validated',
            'completed',
            'rolling_back',
            'rolled_back',
            'failed',
            'interrupted'
        )
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    error TEXT,
    FOREIGN KEY (installation_id) REFERENCES installations(id) ON DELETE CASCADE,
    FOREIGN KEY (restore_point_id) REFERENCES restore_points(id) ON DELETE RESTRICT
);

CREATE TABLE update_events (
    id INTEGER PRIMARY KEY,
    update_transaction_id INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    phase TEXT NOT NULL,
    detail TEXT NOT NULL,
    FOREIGN KEY (update_transaction_id) REFERENCES update_transactions(id) ON DELETE CASCADE
);

CREATE INDEX idx_update_transactions_installation_created
    ON update_transactions(installation_id, created_at DESC);

CREATE INDEX idx_update_transactions_status
    ON update_transactions(status);

CREATE INDEX idx_update_events_transaction_created
    ON update_events(update_transaction_id, created_at, id);

PRAGMA user_version = 6;

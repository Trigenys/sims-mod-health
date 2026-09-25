ALTER TABLE fingerprints RENAME TO fingerprints_v3;

CREATE TABLE fingerprints (
    id INTEGER PRIMARY KEY,
    local_file_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (
        kind IN (
            'sha256',
            'curseforge',
            'resource_signature',
            'script_signature',
            'quick'
        )
    ),
    value TEXT NOT NULL,
    algorithm_version TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    FOREIGN KEY (local_file_id) REFERENCES local_files(id) ON DELETE CASCADE,
    UNIQUE (local_file_id, kind)
);

INSERT INTO fingerprints (
    id,
    local_file_id,
    kind,
    value,
    algorithm_version,
    computed_at
)
SELECT
    id,
    local_file_id,
    kind,
    value,
    algorithm_version,
    computed_at
FROM fingerprints_v3 legacy
WHERE legacy.id = (
    SELECT MAX(current.id)
    FROM fingerprints_v3 current
    WHERE current.local_file_id = legacy.local_file_id
      AND current.kind = legacy.kind
);

DROP TABLE fingerprints_v3;

CREATE INDEX idx_fingerprints_kind_value
    ON fingerprints(kind, value);

PRAGMA user_version = 4;

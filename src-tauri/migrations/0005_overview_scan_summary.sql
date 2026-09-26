ALTER TABLE scan_sessions ADD COLUMN files_skipped INTEGER NOT NULL DEFAULT 0
    CHECK (files_skipped >= 0);

ALTER TABLE scan_sessions ADD COLUMN observation_count INTEGER NOT NULL DEFAULT 0
    CHECK (observation_count >= 0);

PRAGMA user_version = 5;

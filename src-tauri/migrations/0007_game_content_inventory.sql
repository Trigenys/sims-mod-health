CREATE TABLE game_content_installations (
    id INTEGER PRIMARY KEY,
    install_root TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL CHECK (provider IN ('ea_app', 'steam', 'unknown')),
    provider_evidence TEXT NOT NULL,
    game_version TEXT,
    version_evidence_kind TEXT NOT NULL
        CHECK (version_evidence_kind IN ('default_ini', 'sentinel_fingerprint', 'unknown')),
    version_confidence TEXT NOT NULL
        CHECK (version_confidence IN ('definitive', 'probable', 'unknown')),
    sentinel_json TEXT NOT NULL DEFAULT '[]',
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);

CREATE TABLE installed_packs (
    id INTEGER PRIMARY KEY,
    game_content_installation_id INTEGER NOT NULL,
    pack_code TEXT NOT NULL,
    pack_kind TEXT NOT NULL
        CHECK (pack_kind IN ('expansion', 'game', 'stuff_or_kit', 'free', 'kit', 'unknown')),
    local_state TEXT NOT NULL
        CHECK (local_state IN ('installed', 'partial', 'unknown')),
    size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    marker_count INTEGER NOT NULL DEFAULT 0 CHECK (marker_count >= 0),
    observed_at TEXT NOT NULL,
    FOREIGN KEY (game_content_installation_id)
        REFERENCES game_content_installations(id) ON DELETE CASCADE,
    UNIQUE (game_content_installation_id, pack_code)
);

CREATE INDEX idx_game_content_provider
    ON game_content_installations(provider);

CREATE INDEX idx_installed_packs_installation_code
    ON installed_packs(game_content_installation_id, pack_code);

PRAGMA user_version = 7;

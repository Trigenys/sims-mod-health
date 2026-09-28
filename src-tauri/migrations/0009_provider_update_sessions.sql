CREATE TABLE provider_update_sessions (
    id INTEGER PRIMARY KEY,
    game_content_installation_id INTEGER NOT NULL,
    provider TEXT NOT NULL CHECK (provider IN ('ea_app', 'steam', 'unknown')),
    target_kind TEXT NOT NULL CHECK (target_kind IN ('game', 'pack')),
    target_id TEXT NOT NULL,
    baseline_game_version TEXT,
    state TEXT NOT NULL CHECK (
        state IN (
            'detected',
            'action_required',
            'provider_opened',
            'awaiting_rescan',
            'verified',
            'still_outdated',
            'unknown',
            'failed'
        )
    ),
    detail TEXT NOT NULL,
    provider_executable_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    last_error TEXT,
    FOREIGN KEY (game_content_installation_id)
        REFERENCES game_content_installations(id) ON DELETE CASCADE
);

CREATE TABLE provider_update_events (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN (
            'detected',
            'action_required',
            'provider_opened',
            'awaiting_rescan',
            'verified',
            'still_outdated',
            'unknown',
            'failed'
        )
    ),
    detail TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES provider_update_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_provider_update_sessions_state
    ON provider_update_sessions(state, updated_at DESC);

CREATE INDEX idx_provider_update_events_session
    ON provider_update_events(session_id, observed_at);

PRAGMA user_version = 9;

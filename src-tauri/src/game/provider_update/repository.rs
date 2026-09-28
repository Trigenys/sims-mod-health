use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::state::{
    valid_transition, ProviderUpdateState, ProviderUpdateTargetKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderInstallationContext {
    pub(crate) id: i64,
    pub(crate) provider: String,
    pub(crate) game_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderUpdateSessionView {
    pub(crate) id: i64,
    pub(crate) provider: String,
    pub(crate) target_kind: ProviderUpdateTargetKind,
    pub(crate) target_id: String,
    pub(crate) baseline_game_version: Option<String>,
    pub(crate) state: ProviderUpdateState,
    pub(crate) detail: String,
    pub(crate) provider_executable_name: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
    pub(crate) last_error: Option<String>,
}

pub(crate) fn latest_provider_installation(
    connection: &Connection,
) -> Result<Option<ProviderInstallationContext>, String> {
    connection
        .query_row(
            "SELECT id, provider, game_version
             FROM game_content_installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| {
                Ok(ProviderInstallationContext {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    game_version: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub(crate) fn validate_target(
    connection: &Connection,
    installation_id: i64,
    target_kind: ProviderUpdateTargetKind,
    target_id: &str,
) -> Result<String, String> {
    match target_kind {
        ProviderUpdateTargetKind::Game => {
            if target_id.eq_ignore_ascii_case("game") {
                Ok("game".to_string())
            } else {
                Err("game update target must use targetId 'game'".to_string())
            }
        }
        ProviderUpdateTargetKind::Pack => {
            let normalized = target_id.trim().to_ascii_uppercase();
            if normalized.is_empty()
                || normalized.len() > 8
                || !normalized
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
            {
                return Err("pack update target is invalid".to_string());
            }

            let exists = connection
                .query_row(
                    "SELECT 1
                     FROM installed_packs
                     WHERE game_content_installation_id = ?1
                       AND pack_code = ?2
                     LIMIT 1",
                    params![installation_id, normalized],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .is_some();

            if !exists {
                return Err(format!(
                    "{normalized} is not present in the current local content inventory"
                ));
            }

            Ok(normalized)
        }
    }
}

pub(crate) fn create_session(
    connection: &Connection,
    installation: &ProviderInstallationContext,
    target_kind: ProviderUpdateTargetKind,
    target_id: &str,
) -> Result<ProviderUpdateSessionView, String> {
    let detail = "An update action was detected and is waiting for provider capability resolution.";

    connection
        .execute(
            "INSERT INTO provider_update_sessions (
                game_content_installation_id, provider, target_kind, target_id,
                baseline_game_version, state, detail, created_at, updated_at
             )
             VALUES (
                ?1, ?2, ?3, ?4, ?5, 'detected', ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                installation.id,
                installation.provider,
                target_kind.as_str(),
                target_id,
                installation.game_version,
                detail,
            ],
        )
        .map_err(|error| error.to_string())?;

    let session_id = connection.last_insert_rowid();
    insert_event(
        connection,
        session_id,
        ProviderUpdateState::Detected,
        detail,
    )?;

    load_session(connection, session_id)?
        .ok_or_else(|| "provider update session disappeared after creation".to_string())
}

pub(crate) fn load_session(
    connection: &Connection,
    session_id: i64,
) -> Result<Option<ProviderUpdateSessionView>, String> {
    connection
        .query_row(
            "SELECT
                id, provider, target_kind, target_id, baseline_game_version,
                state, detail, provider_executable_name, created_at, updated_at,
                completed_at, last_error
             FROM provider_update_sessions
             WHERE id = ?1",
            [session_id],
            |row| {
                let target_kind: String = row.get(2)?;
                let state: String = row.get(5)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    target_kind,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    state,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(
            |(
                id,
                provider,
                target_kind,
                target_id,
                baseline_game_version,
                state,
                detail,
                provider_executable_name,
                created_at,
                updated_at,
                completed_at,
                last_error,
            )| {
                let target_kind = ProviderUpdateTargetKind::from_str(&target_kind)
                    .ok_or_else(|| "provider update target kind is invalid".to_string())?;
                let state = ProviderUpdateState::from_str(&state)
                    .ok_or_else(|| "provider update state is invalid".to_string())?;

                Ok(ProviderUpdateSessionView {
                    id,
                    provider,
                    target_kind,
                    target_id,
                    baseline_game_version,
                    state,
                    detail,
                    provider_executable_name,
                    created_at,
                    updated_at,
                    completed_at,
                    last_error,
                })
            },
        )
        .transpose()
}

pub(crate) fn transition_session(
    connection: &Connection,
    session_id: i64,
    next: ProviderUpdateState,
    detail: &str,
    executable_name: Option<&str>,
    last_error: Option<&str>,
) -> Result<ProviderUpdateSessionView, String> {
    let current = load_session(connection, session_id)?
        .ok_or_else(|| "provider update session was not found".to_string())?;

    if !valid_transition(current.state, next) {
        return Err(format!(
            "invalid provider update transition: {} -> {}",
            current.state.as_str(),
            next.as_str()
        ));
    }

    connection
        .execute(
            "UPDATE provider_update_sessions
             SET state = ?2,
                 detail = ?3,
                 provider_executable_name = COALESCE(?4, provider_executable_name),
                 last_error = ?5,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 completed_at = CASE
                    WHEN ?6 = 1 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                    ELSE NULL
                 END
             WHERE id = ?1",
            params![
                session_id,
                next.as_str(),
                detail,
                executable_name,
                last_error,
                if next.is_terminal() { 1 } else { 0 },
            ],
        )
        .map_err(|error| error.to_string())?;

    insert_event(connection, session_id, next, detail)?;

    load_session(connection, session_id)?
        .ok_or_else(|| "provider update session disappeared after transition".to_string())
}

pub(crate) fn begin_verification(
    connection: &Connection,
    session_id: i64,
) -> Result<ProviderUpdateSessionView, String> {
    let current = load_session(connection, session_id)?
        .ok_or_else(|| "provider update session was not found".to_string())?;

    match current.state {
        ProviderUpdateState::AwaitingRescan => Ok(current),
        ProviderUpdateState::ActionRequired => transition_session(
            connection,
            session_id,
            ProviderUpdateState::AwaitingRescan,
            "Manual provider update is being verified against local files.",
            None,
            None,
        ),
        ProviderUpdateState::ProviderOpened => transition_session(
            connection,
            session_id,
            ProviderUpdateState::AwaitingRescan,
            "Provider was opened; local verification is starting.",
            None,
            None,
        ),
        state => Err(format!(
            "provider update session in state '{}' cannot be verified",
            state.as_str()
        )),
    }
}

pub(crate) fn latest_mod_user_root(
    connection: &Connection,
) -> Result<Option<PathBuf>, String> {
    connection
        .query_row(
            "SELECT game_root
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| Ok(PathBuf::from(row.get::<_, String>(0)?)),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub(crate) fn sync_latest_mod_game_version(
    connection: &Connection,
    game_version: &str,
) -> Result<(), String> {
    let latest_id = connection
        .query_row(
            "SELECT id
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if let Some(installation_id) = latest_id {
        connection
            .execute(
                "UPDATE installations
                 SET game_version = ?2,
                     last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                params![installation_id, game_version],
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

pub(crate) fn current_program_version_for_session(
    connection: &Connection,
    session_id: i64,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT g.game_version
             FROM provider_update_sessions s
             JOIN game_content_installations g
               ON g.id = s.game_content_installation_id
             WHERE s.id = ?1",
            [session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
        .map(Option::flatten)
}

fn insert_event(
    connection: &Connection,
    session_id: i64,
    state: ProviderUpdateState,
    detail: &str,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO provider_update_events (
                session_id, state, detail, observed_at
             )
             VALUES (
                ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![session_id, state.as_str(), detail],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().expect("db");
        connection
            .execute_batch(include_str!(
                "../../../migrations/0007_game_content_inventory.sql"
            ))
            .expect("inventory schema");
        connection
            .execute_batch(include_str!(
                "../../../migrations/0009_provider_update_sessions.sql"
            ))
            .expect("provider session schema");
        connection
    }

    fn installation(connection: &Connection) -> ProviderInstallationContext {
        connection
            .execute(
                "INSERT INTO game_content_installations (
                    install_root, provider, provider_evidence, game_version,
                    version_evidence_kind, version_confidence, sentinel_json,
                    first_seen_at, last_seen_at
                 )
                 VALUES (
                    'C:/Games/The Sims 4', 'steam', 'steam_manifest',
                    '1.1.1.1', 'default_ini', 'definitive', '[]', 'now', 'now'
                 )",
                [],
            )
            .expect("installation");

        ProviderInstallationContext {
            id: connection.last_insert_rowid(),
            provider: "steam".to_string(),
            game_version: Some("1.1.1.1".to_string()),
        }
    }

    #[test]
    fn awaiting_rescan_survives_reload() {
        let connection = connection();
        let installation = installation(&connection);
        let created = create_session(
            &connection,
            &installation,
            ProviderUpdateTargetKind::Game,
            "game",
        )
        .expect("session");
        let action = transition_session(
            &connection,
            created.id,
            ProviderUpdateState::ActionRequired,
            "action",
            None,
            None,
        )
        .expect("action");
        transition_session(
            &connection,
            action.id,
            ProviderUpdateState::AwaitingRescan,
            "waiting",
            None,
            None,
        )
        .expect("awaiting");

        let reloaded = load_session(&connection, created.id)
            .expect("load")
            .expect("session");
        assert_eq!(reloaded.state, ProviderUpdateState::AwaitingRescan);
    }

    #[test]
    fn installed_pack_target_is_normalized_and_validated() {
        let connection = connection();
        let installation = installation(&connection);
        connection
            .execute(
                "INSERT INTO installed_packs (
                    game_content_installation_id, pack_code, pack_kind,
                    local_state, size_bytes, marker_count, observed_at
                 )
                 VALUES (?1, 'EP01', 'expansion', 'installed', 1, 1, 'now')",
                [installation.id],
            )
            .expect("pack");

        assert_eq!(
            validate_target(
                &connection,
                installation.id,
                ProviderUpdateTargetKind::Pack,
                "ep01",
            )
            .expect("target"),
            "EP01"
        );
    }
}

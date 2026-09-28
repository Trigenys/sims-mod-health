mod adapters;
mod repository;
mod state;

use std::path::Path;

use crate::{
    game::content_health::{
        GameContentHealthSnapshot, GameContentHealthState,
    },
    storage,
};

pub(crate) use adapters::ProviderUpdateCapability;
pub(crate) use repository::ProviderUpdateSessionView;
pub(crate) use state::{
    ProviderUpdateState, ProviderUpdateTargetKind,
};

use adapters::{capability, open_provider};
use repository::{
    begin_verification as begin_verification_transition, create_session,
    latest_pending_session, latest_provider_installation, load_session, transition_session,
    validate_target,
};

pub(crate) fn get_capability(
    database_path: &Path,
) -> Result<ProviderUpdateCapability, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    let provider = latest_provider_installation(&connection)?
        .map(|installation| installation.provider)
        .unwrap_or_else(|| "unknown".to_string());

    Ok(capability(&provider))
}

pub(crate) fn start_provider_update(
    database_path: &Path,
    target_kind: ProviderUpdateTargetKind,
    target_id: &str,
) -> Result<ProviderUpdateSessionView, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    let installation = latest_provider_installation(&connection)?
        .ok_or_else(|| "no local game program installation is available".to_string())?;
    let target_id = validate_target(
        &connection,
        installation.id,
        target_kind,
        target_id,
    )?;

    let session = create_session(
        &connection,
        &installation,
        target_kind,
        &target_id,
    )?;
    let capability = capability(&installation.provider);
    let action = transition_session(
        &connection,
        session.id,
        ProviderUpdateState::ActionRequired,
        &capability.detail,
        None,
        None,
    )?;

    if !capability.supported {
        return Ok(action);
    }

    drop(connection);

    match open_provider(&installation.provider) {
        Ok(launch) => {
            let connection =
                storage::open(database_path).map_err(|error| error.to_string())?;
            let opened = transition_session(
                &connection,
                session.id,
                ProviderUpdateState::ProviderOpened,
                "The official provider client was opened. Sims Mod Health has not assumed that the update completed.",
                Some(&launch.executable_name),
                None,
            )?;

            transition_session(
                &connection,
                opened.id,
                ProviderUpdateState::AwaitingRescan,
                "Waiting for a local rescan to verify the resulting game and pack state.",
                None,
                None,
            )
        }
        Err(error) => {
            let connection =
                storage::open(database_path).map_err(|storage_error| storage_error.to_string())?;
            transition_session(
                &connection,
                session.id,
                ProviderUpdateState::Failed,
                "The provider client could not be opened automatically.",
                None,
                Some(&error.to_string()),
            )
        }
    }
}

pub(crate) fn get_pending_session(
    database_path: &Path,
) -> Result<Option<ProviderUpdateSessionView>, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    latest_pending_session(&connection)
}

pub(crate) fn get_session(
    database_path: &Path,
    session_id: i64,
) -> Result<ProviderUpdateSessionView, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    load_session(&connection, session_id)?
        .ok_or_else(|| "provider update session was not found".to_string())
}

pub(crate) fn begin_verification(
    database_path: &Path,
    session_id: i64,
) -> Result<ProviderUpdateSessionView, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    begin_verification_transition(&connection, session_id)
}

pub(crate) fn complete_verification(
    database_path: &Path,
    session_id: i64,
    health: &GameContentHealthSnapshot,
) -> Result<ProviderUpdateSessionView, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    let session = load_session(&connection, session_id)?
        .ok_or_else(|| "provider update session was not found".to_string())?;

    if session.state != ProviderUpdateState::AwaitingRescan {
        return Err(format!(
            "provider update session in state '{}' cannot complete verification",
            session.state.as_str()
        ));
    }

    let finding = match session.target_kind {
        ProviderUpdateTargetKind::Game => health.game.as_ref(),
        ProviderUpdateTargetKind::Pack => health
            .packs
            .iter()
            .find(|finding| finding.target_id.eq_ignore_ascii_case(&session.target_id)),
    };

    let (state, detail) = match finding.map(|finding| finding.state) {
        Some(GameContentHealthState::Current) => (
            ProviderUpdateState::Verified,
            "Local evidence verifies that the update target is current.",
        ),
        Some(GameContentHealthState::UpdateAvailable)
        | Some(GameContentHealthState::GameUpdateRequired) => (
            ProviderUpdateState::StillOutdated,
            "Local evidence still reports an update requirement after the provider handoff.",
        ),
        Some(GameContentHealthState::MetadataStale)
        | Some(GameContentHealthState::LocalIntegrityUncertain)
        | Some(GameContentHealthState::Unknown)
        | None => (
            ProviderUpdateState::Unknown,
            "Local evidence cannot yet verify that the provider update completed successfully.",
        ),
    };

    transition_session(
        &connection,
        session_id,
        state,
        detail,
        None,
        None,
    )
}

pub(crate) fn fail_verification(
    database_path: &Path,
    session_id: i64,
    error: &str,
) -> Result<ProviderUpdateSessionView, String> {
    let connection = storage::open(database_path).map_err(|storage_error| storage_error.to_string())?;
    let session = load_session(&connection, session_id)?
        .ok_or_else(|| "provider update session was not found".to_string())?;

    if session.state != ProviderUpdateState::AwaitingRescan {
        return Err(error.to_string());
    }

    transition_session(
        &connection,
        session_id,
        ProviderUpdateState::Failed,
        "Post-update local verification failed before a safe conclusion could be reached.",
        None,
        Some(error),
    )
}

pub(crate) use repository::{
    current_program_version_for_session, latest_mod_user_root,
    sync_latest_mod_game_version,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::content_health::{
        GameContentFindingKind, GameContentHealthFinding, ManifestState,
    };
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn database() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("temp");
        let path = temp.path().join("test.sqlite3");
        let mut connection = Connection::open(&path).expect("db");
        connection
            .execute_batch(include_str!("../../../migrations/0007_game_content_inventory.sql"))
            .expect("inventory schema");
        connection
            .execute_batch(include_str!("../../../migrations/0009_provider_update_sessions.sql"))
            .expect("provider schema");
        connection
            .execute(
                "INSERT INTO game_content_installations (
                    install_root, provider, provider_evidence, game_version,
                    version_evidence_kind, version_confidence, sentinel_json,
                    first_seen_at, last_seen_at
                 )
                 VALUES (
                    'C:/Games/The Sims 4', 'unknown', 'manual',
                    '1.1.1.1', 'default_ini', 'definitive', '[]', 'now', 'now'
                 )",
                [],
            )
            .expect("installation");
        drop(connection);
        (temp, path)
    }

    #[test]
    fn unknown_provider_stays_action_required_for_manual_update() {
        let (_temp, path) = database();
        let session = start_provider_update(
            &path,
            ProviderUpdateTargetKind::Game,
            "game",
        )
        .expect("session");

        assert_eq!(session.state, ProviderUpdateState::ActionRequired);

        let reloaded = get_session(&path, session.id).expect("reload");
        assert_eq!(reloaded.state, ProviderUpdateState::ActionRequired);
    }

    #[test]
    fn manual_update_can_enter_awaiting_rescan_after_restart() {
        let (_temp, path) = database();
        let session = start_provider_update(
            &path,
            ProviderUpdateTargetKind::Game,
            "game",
        )
        .expect("session");

        let waiting = begin_verification(&path, session.id).expect("verify");
        assert_eq!(waiting.state, ProviderUpdateState::AwaitingRescan);
    }

    #[test]
    fn current_health_completes_session_as_verified() {
        let (_temp, path) = database();
        let session = start_provider_update(
            &path,
            ProviderUpdateTargetKind::Game,
            "game",
        )
        .expect("session");
        begin_verification(&path, session.id).expect("begin verification");

        let health = GameContentHealthSnapshot {
            manifest_state: ManifestState::Fresh,
            manifest_version: Some("m1".to_string()),
            source_identity: Some("registry".to_string()),
            source_url: None,
            detail: "fixture".to_string(),
            game: Some(GameContentHealthFinding {
                kind: GameContentFindingKind::Game,
                target_id: "game".to_string(),
                state: GameContentHealthState::Current,
                disputed: false,
                manifest_stale: false,
                current_version: Some("1.2.0.0".to_string()),
                required_version: Some("1.2.0.0".to_string()),
                reason: "current".to_string(),
                evidence: Vec::new(),
            }),
            packs: Vec::new(),
        };

        let verified =
            complete_verification(&path, session.id, &health).expect("complete");
        assert_eq!(verified.state, ProviderUpdateState::Verified);
    }
}

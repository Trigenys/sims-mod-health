mod manifest;
mod repository;
mod resolver;

use std::{future::Future, path::Path, pin::Pin};

use crate::{
    registry::{
        configured_registry_url, RegistryClient, RegistryError, RegistryGameContentManifest,
    },
    storage,
};

use manifest::adapt_registry_manifest;
use repository::{load_cached_manifest, load_latest_local_state, save_cached_manifest};
pub(crate) use resolver::GameContentHealthSnapshot;
use resolver::{evaluate, missing_manifest, ManifestState};

pub(crate) trait ContentManifestSource {
    fn fetch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<RegistryGameContentManifest, RegistryError>> + Send + 'a>>;
}

struct RegistryManifestSource {
    client: RegistryClient,
}

impl ContentManifestSource for RegistryManifestSource {
    fn fetch<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<RegistryGameContentManifest, RegistryError>> + Send + 'a>>
    {
        Box::pin(self.client.fetch_game_content_manifest())
    }
}

pub(crate) async fn evaluate_game_content_health(
    database_path: &Path,
) -> Result<GameContentHealthSnapshot, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    let Some(local) = load_latest_local_state(&connection)? else {
        return Ok(GameContentHealthSnapshot {
            manifest_state: ManifestState::Missing,
            manifest_version: None,
            source_identity: None,
            source_url: None,
            detail: "No local The Sims 4 program installation has been inventoried yet."
                .to_string(),
            game: None,
            packs: Vec::new(),
        });
    };

    let registry_url = configured_registry_url(&connection);
    let cached = load_cached_manifest(&connection)?;
    drop(connection);

    let live_result = match RegistryClient::new(registry_url) {
        Ok(client) => RegistryManifestSource { client }.fetch().await,
        Err(error) => Err(error),
    };

    match live_result {
        Ok(wire) => {
            let manifest = adapt_registry_manifest(wire.clone())?;
            let connection = storage::open(database_path).map_err(|error| error.to_string())?;
            save_cached_manifest(&connection, &wire)?;
            Ok(evaluate(
                &local,
                &manifest,
                ManifestState::Fresh,
                "Game/DLC metadata is current from the Registry.".to_string(),
            ))
        }
        Err(error) => match cached {
            Some(cached) => {
                let manifest = adapt_registry_manifest(cached.manifest)?;
                Ok(evaluate(
                    &local,
                    &manifest,
                    ManifestState::CachedStale,
                    format!(
                        "Registry is unavailable; cached Game/DLC metadata from {} is being used. {error}",
                        cached.cached_at
                    ),
                ))
            }
            None => Ok(missing_manifest(
                &local,
                format!("Registry manifest is unavailable and no local cache exists. {error}"),
            )),
        },
    }
}

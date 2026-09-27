use std::path::Path;

use serde::Serialize;

use crate::{
    overview,
    registry::{DiscoveryRecommendation, RegistryClient},
};

const RECOMMENDATION_LIMIT: usize = 12;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoverySnapshot {
    pub(crate) patch_version: Option<String>,
    pub(crate) state: String,
    pub(crate) detail: String,
    pub(crate) recommendations: Vec<DiscoveryRecommendation>,
}

pub(crate) async fn load(database_path: &Path) -> DiscoverySnapshot {
    let seed = match overview::discovery_seed(database_path).await {
        Ok(Some(seed)) => seed,
        Ok(None) => {
            return DiscoverySnapshot {
                patch_version: None,
                state: "empty".to_string(),
                detail: "Scan a Sims 4 installation before requesting recommendations.".to_string(),
                recommendations: Vec::new(),
            };
        }
        Err(error) => {
            return DiscoverySnapshot {
                patch_version: None,
                state: "partial".to_string(),
                detail: error,
                recommendations: Vec::new(),
            };
        }
    };

    if seed.installed_release_ids.is_empty() {
        return DiscoverySnapshot {
            patch_version: Some(seed.patch_version),
            state: "empty".to_string(),
            detail: "No enabled installed artifact has a canonical Registry identity yet."
                .to_string(),
            recommendations: Vec::new(),
        };
    }

    let client = match RegistryClient::new(seed.registry_base_url) {
        Ok(client) => client,
        Err(error) => {
            return DiscoverySnapshot {
                patch_version: Some(seed.patch_version),
                state: "partial".to_string(),
                detail: error.to_string(),
                recommendations: Vec::new(),
            };
        }
    };

    match client
        .recommend_discovery(
            &seed.patch_version,
            &seed.installed_release_ids,
            RECOMMENDATION_LIMIT,
        )
        .await
    {
        Ok(recommendations) => DiscoverySnapshot {
            patch_version: Some(seed.patch_version),
            state: "ready".to_string(),
            detail: if recommendations.is_empty() {
                "No safe recommendation currently passes compatibility and known-conflict filters."
                    .to_string()
            } else {
                "Recommendations are filtered for current-patch compatibility before deterministic ranking."
                    .to_string()
            },
            recommendations,
        },
        Err(error) => DiscoverySnapshot {
            patch_version: Some(seed.patch_version),
            state: if error.is_transport() {
                "offline".to_string()
            } else {
                "partial".to_string()
            },
            detail: error.to_string(),
            recommendations: Vec::new(),
        },
    }
}

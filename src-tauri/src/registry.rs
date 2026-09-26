use std::{error::Error, fmt::{Display, Formatter}, time::Duration};

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const ARTIFACT_BATCH: usize = 100;
const HEALTH_BATCH: usize = 100;
const RELATIONSHIP_BATCH: usize = 500;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryFingerprintProbe {
    pub(crate) kind: String,
    pub(crate) value: String,
    pub(crate) algorithm_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryIdentityHints {
    pub(crate) creator: Option<String>,
    pub(crate) mod_name: Option<String>,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryArtifactProbe {
    pub(crate) client_ref: String,
    pub(crate) artifact_kind: Option<String>,
    pub(crate) filename: Option<String>,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) identity_hints: Option<RegistryIdentityHints>,
    pub(crate) fingerprints: Vec<RegistryFingerprintProbe>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ArtifactMatch {
    pub(crate) artifact_id: String,
    pub(crate) release_id: String,
    pub(crate) mod_id: String,
    pub(crate) confidence: String,
    pub(crate) deterministic: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ArtifactResolution {
    pub(crate) client_ref: String,
    pub(crate) status: String,
    pub(crate) selected_artifact_id: Option<String>,
    pub(crate) matches: Vec<ArtifactMatch>,
}

#[derive(Debug, Deserialize)]
struct ResolveResponse {
    artifacts: Vec<ArtifactResolution>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateAssessment {
    pub(crate) available: bool,
    pub(crate) target_version: Option<String>,
    pub(crate) target_compatibility_state: Option<String>,
    pub(crate) target_disputed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HealthAssessment {
    pub(crate) release_id: String,
    pub(crate) mod_id: String,
    pub(crate) state: String,
    pub(crate) compatibility_state: String,
    pub(crate) disputed: bool,
    pub(crate) reason: String,
    pub(crate) update: UpdateAssessment,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    items: Vec<HealthAssessment>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DependencyFinding {
    pub(crate) required_by_release_id: String,
    pub(crate) status: String,
    pub(crate) action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct KnownIncompatibilityFinding {
    pub(crate) left_release_id: String,
    pub(crate) right_release_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RelationshipResponse {
    pub(crate) dependency_findings: Vec<DependencyFinding>,
    pub(crate) known_incompatibilities: Vec<KnownIncompatibilityFinding>,
}

#[derive(Debug)]
pub(crate) enum RegistryError {
    Transport(reqwest::Error),
    Status { status: u16, body: String },
}

impl RegistryError {
    pub(crate) fn is_transport(&self) -> bool {
        matches!(self, Self::Transport(_))
    }
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(formatter, "registry transport error: {error}"),
            Self::Status { status, body } => {
                write!(formatter, "registry returned HTTP {status}: {body}")
            }
        }
    }
}

impl Error for RegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Status { .. } => None,
        }
    }
}

impl From<reqwest::Error> for RegistryError {
    fn from(value: reqwest::Error) -> Self {
        Self::Transport(value)
    }
}

#[derive(Clone)]
pub(crate) struct RegistryClient {
    base_url: String,
    client: Client,
}

impl RegistryClient {
    pub(crate) fn new(base_url: String) -> Result<Self, RegistryError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(7))
            .build()?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    pub(crate) async fn resolve_artifacts(
        &self,
        probes: &[RegistryArtifactProbe],
    ) -> Result<Vec<ArtifactResolution>, RegistryError> {
        let mut resolutions = Vec::new();

        for chunk in probes.chunks(ARTIFACT_BATCH) {
            #[derive(Serialize)]
            struct ResolveRequest<'a> {
                artifacts: &'a [RegistryArtifactProbe],
            }

            let response: ResolveResponse = self
                .post_json(
                    "/v1/artifacts/resolve",
                    &ResolveRequest { artifacts: chunk },
                )
                .await?;
            resolutions.extend(response.artifacts);
        }

        Ok(resolutions)
    }

    pub(crate) async fn evaluate_health(
        &self,
        patch_version: &str,
        release_ids: &[String],
    ) -> Result<Vec<HealthAssessment>, RegistryError> {
        let mut items = Vec::new();

        for chunk in release_ids.chunks(HEALTH_BATCH) {
            #[derive(Serialize)]
            struct HealthRequest<'a> {
                patch_version: &'a str,
                platform: &'static str,
                installed_release_ids: &'a [String],
            }

            let response: HealthResponse = self
                .post_json(
                    "/v1/health/evaluate",
                    &HealthRequest {
                        patch_version,
                        platform: "windows",
                        installed_release_ids: chunk,
                    },
                )
                .await?;
            items.extend(response.items);
        }

        Ok(items)
    }

    pub(crate) async fn evaluate_relationships(
        &self,
        release_ids: &[String],
    ) -> Result<RelationshipResponse, RegistryError> {
        let mut dependency_findings = Vec::new();
        let mut known_incompatibilities = Vec::new();

        for chunk in release_ids.chunks(RELATIONSHIP_BATCH) {
            #[derive(Serialize)]
            struct RelationshipRequest<'a> {
                installed_release_ids: &'a [String],
            }

            let response: RelationshipResponse = self
                .post_json(
                    "/v1/health/relationships/evaluate",
                    &RelationshipRequest {
                        installed_release_ids: chunk,
                    },
                )
                .await?;
            dependency_findings.extend(response.dependency_findings);
            known_incompatibilities.extend(response.known_incompatibilities);
        }

        Ok(RelationshipResponse {
            dependency_findings,
            known_incompatibilities,
        })
    }

    async fn post_json<Request, Response>(
        &self,
        path: &str,
        request: &Request,
    ) -> Result<Response, RegistryError>
    where
        Request: Serialize + ?Sized,
        Response: DeserializeOwned,
    {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RegistryError::Status {
                status: status.as_u16(),
                body,
            });
        }

        Ok(response.json::<Response>().await?)
    }
}

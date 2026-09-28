use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderUpdateTargetKind {
    Game,
    Pack,
}

impl ProviderUpdateTargetKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Pack => "pack",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderUpdateState {
    Detected,
    ActionRequired,
    ProviderOpened,
    AwaitingRescan,
    Verified,
    StillOutdated,
    Unknown,
    Failed,
}

impl ProviderUpdateState {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::ActionRequired => "action_required",
            Self::ProviderOpened => "provider_opened",
            Self::AwaitingRescan => "awaiting_rescan",
            Self::Verified => "verified",
            Self::StillOutdated => "still_outdated",
            Self::Unknown => "unknown",
            Self::Failed => "failed",
        }
    }

    pub(super) fn from_str(value: &str) -> Option<Self> {
        match value {
            "detected" => Some(Self::Detected),
            "action_required" => Some(Self::ActionRequired),
            "provider_opened" => Some(Self::ProviderOpened),
            "awaiting_rescan" => Some(Self::AwaitingRescan),
            "verified" => Some(Self::Verified),
            "still_outdated" => Some(Self::StillOutdated),
            "unknown" => Some(Self::Unknown),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub(super) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Verified | Self::StillOutdated | Self::Unknown | Self::Failed
        )
    }
}

pub(super) fn valid_transition(from: ProviderUpdateState, to: ProviderUpdateState) -> bool {
    match from {
        ProviderUpdateState::Detected => matches!(
            to,
            ProviderUpdateState::ActionRequired | ProviderUpdateState::Failed
        ),
        ProviderUpdateState::ActionRequired => matches!(
            to,
            ProviderUpdateState::ProviderOpened
                | ProviderUpdateState::AwaitingRescan
                | ProviderUpdateState::Failed
        ),
        ProviderUpdateState::ProviderOpened => matches!(
            to,
            ProviderUpdateState::AwaitingRescan | ProviderUpdateState::Failed
        ),
        ProviderUpdateState::AwaitingRescan => matches!(
            to,
            ProviderUpdateState::Verified
                | ProviderUpdateState::StillOutdated
                | ProviderUpdateState::Unknown
                | ProviderUpdateState::Failed
        ),
        ProviderUpdateState::Verified
        | ProviderUpdateState::StillOutdated
        | ProviderUpdateState::Unknown
        | ProviderUpdateState::Failed => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awaiting_rescan_can_reach_only_verification_outcomes() {
        assert!(valid_transition(
            ProviderUpdateState::AwaitingRescan,
            ProviderUpdateState::Verified
        ));
        assert!(valid_transition(
            ProviderUpdateState::AwaitingRescan,
            ProviderUpdateState::StillOutdated
        ));
        assert!(!valid_transition(
            ProviderUpdateState::AwaitingRescan,
            ProviderUpdateState::ProviderOpened
        ));
    }

    #[test]
    fn terminal_states_do_not_transition() {
        assert!(!valid_transition(
            ProviderUpdateState::Verified,
            ProviderUpdateState::ActionRequired
        ));
    }
}

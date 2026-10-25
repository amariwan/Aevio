//! Pure plugin identity and lifecycle invariants.

use aevio_contracts::plugin::PluginLifecycleState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIdentity {
    pub id: String,
    pub publisher_id: String,
}

impl PluginIdentity {
    pub fn new(
        id: impl Into<String>,
        publisher_id: impl Into<String>,
    ) -> Result<Self, PluginError> {
        let identity = Self {
            id: id.into(),
            publisher_id: publisher_id.into(),
        };
        if identity.id.is_empty() || identity.publisher_id.is_empty() {
            return Err(PluginError::InvalidIdentity);
        }
        Ok(identity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLifecycleEvent {
    RequestInstall,
    Resolve,
    Download,
    Verify,
    RequestPermissions,
    Stage,
    CommitInstall,
    Activate,
    Deactivate,
    Enable,
    Disable,
    UpdateAvailable,
    StartUpdate,
    StartUninstall,
    CompleteUninstall,
    Fail,
    Quarantine,
    Recover,
    MarkIncompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    InvalidIdentity,
    IllegalTransition {
        from: PluginLifecycleState,
        event: PluginLifecycleEvent,
    },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity => write!(formatter, "plugin identity is incomplete"),
            Self::IllegalTransition { from, event } => {
                write!(formatter, "cannot apply {event:?} to {from:?}")
            }
        }
    }
}

impl std::error::Error for PluginError {}

pub fn transition(
    state: PluginLifecycleState,
    event: PluginLifecycleEvent,
) -> Result<PluginLifecycleState, PluginError> {
    let next = match (state, event) {
        (PluginLifecycleState::Discovered, PluginLifecycleEvent::RequestInstall) => {
            PluginLifecycleState::InstallRequested
        }
        (PluginLifecycleState::InstallRequested, PluginLifecycleEvent::Resolve) => {
            PluginLifecycleState::Resolving
        }
        (PluginLifecycleState::Resolving, PluginLifecycleEvent::Download) => {
            PluginLifecycleState::Downloading
        }
        (PluginLifecycleState::Downloading, PluginLifecycleEvent::Verify) => {
            PluginLifecycleState::Verifying
        }
        (PluginLifecycleState::Verifying, PluginLifecycleEvent::RequestPermissions) => {
            PluginLifecycleState::AwaitingPermissions
        }
        (PluginLifecycleState::AwaitingPermissions, PluginLifecycleEvent::Stage) => {
            PluginLifecycleState::Staging
        }
        (PluginLifecycleState::Staging, PluginLifecycleEvent::CommitInstall) => {
            PluginLifecycleState::Installed
        }
        (PluginLifecycleState::Installed, PluginLifecycleEvent::Activate)
        | (PluginLifecycleState::Disabled, PluginLifecycleEvent::Activate) => {
            PluginLifecycleState::Activating
        }
        (PluginLifecycleState::Activating, PluginLifecycleEvent::Enable) => {
            PluginLifecycleState::Active
        }
        (PluginLifecycleState::Active, PluginLifecycleEvent::Deactivate) => {
            PluginLifecycleState::Deactivating
        }
        (PluginLifecycleState::Deactivating, PluginLifecycleEvent::Disable) => {
            PluginLifecycleState::Disabled
        }
        (PluginLifecycleState::Active, PluginLifecycleEvent::UpdateAvailable)
        | (PluginLifecycleState::Installed, PluginLifecycleEvent::UpdateAvailable) => {
            PluginLifecycleState::UpdateAvailable
        }
        (PluginLifecycleState::UpdateAvailable, PluginLifecycleEvent::StartUpdate)
        | (PluginLifecycleState::Active, PluginLifecycleEvent::StartUpdate) => {
            PluginLifecycleState::Updating
        }
        (PluginLifecycleState::Installed, PluginLifecycleEvent::StartUninstall)
        | (PluginLifecycleState::Disabled, PluginLifecycleEvent::StartUninstall)
        | (PluginLifecycleState::Failed, PluginLifecycleEvent::StartUninstall)
        | (PluginLifecycleState::Quarantined, PluginLifecycleEvent::StartUninstall)
        | (PluginLifecycleState::Incompatible, PluginLifecycleEvent::StartUninstall)
        | (PluginLifecycleState::RecoveryRequired, PluginLifecycleEvent::StartUninstall) => {
            PluginLifecycleState::Uninstalling
        }
        (PluginLifecycleState::Uninstalling, PluginLifecycleEvent::CompleteUninstall) => {
            PluginLifecycleState::Uninstalled
        }
        (PluginLifecycleState::Updating, PluginLifecycleEvent::CommitInstall) => {
            PluginLifecycleState::Installed
        }
        (PluginLifecycleState::Updating, PluginLifecycleEvent::Activate) => {
            PluginLifecycleState::Activating
        }
        (PluginLifecycleState::Failed, PluginLifecycleEvent::Recover)
        | (PluginLifecycleState::RecoveryRequired, PluginLifecycleEvent::Recover) => {
            PluginLifecycleState::Installed
        }
        (PluginLifecycleState::Verifying, PluginLifecycleEvent::MarkIncompatible)
        | (PluginLifecycleState::Resolving, PluginLifecycleEvent::MarkIncompatible) => {
            PluginLifecycleState::Incompatible
        }
        (_, PluginLifecycleEvent::Fail) => PluginLifecycleState::Failed,
        (_, PluginLifecycleEvent::Quarantine) => PluginLifecycleState::Quarantined,
        _ => return Err(PluginError::IllegalTransition { from: state, event }),
    };
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_contracts::plugin::PluginLifecycleState::*;

    #[test]
    fn install_lifecycle_is_explicit() {
        let events = [
            PluginLifecycleEvent::RequestInstall,
            PluginLifecycleEvent::Resolve,
            PluginLifecycleEvent::Download,
            PluginLifecycleEvent::Verify,
            PluginLifecycleEvent::RequestPermissions,
            PluginLifecycleEvent::Stage,
            PluginLifecycleEvent::CommitInstall,
            PluginLifecycleEvent::Activate,
            PluginLifecycleEvent::Enable,
        ];
        let mut state = Discovered;
        for event in events {
            state = transition(state, event).expect("legal transition");
        }
        assert_eq!(state, Active);
    }

    #[test]
    fn active_plugin_cannot_jump_to_uninstall() {
        assert!(transition(Active, PluginLifecycleEvent::StartUninstall).is_err());
    }

    #[test]
    fn uninstall_keeps_a_terminal_registration_state() {
        let state = transition(Disabled, PluginLifecycleEvent::StartUninstall)
            .and_then(|state| transition(state, PluginLifecycleEvent::CompleteUninstall))
            .expect("legal uninstall transition");
        assert_eq!(state, Uninstalled);
    }
}

use hub::{HubError, SendError};
use serde::Serialize;
use ts_rs::TS;

use crate::cli::CliError;
use crate::settings::SettingsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum UiErrorCode {
    Empty,
    TooLarge,
    NoTarget,
    TargetOffline,
    UnknownSession,
    SessionLive,
    InvalidSettings,
    CliConflict,
    PluginInstall,
    HubDown,
    Internal,
}

/// The one error shape every command returns to the UI.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct UiError {
    pub code: UiErrorCode,
    pub message: String,
}

impl UiError {
    pub fn new(code: UiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn hub_down() -> Self {
        Self::new(
            UiErrorCode::HubDown,
            "Voice Bridge isn't running its hub. Restart the app",
        )
    }

    pub fn internal(err: impl std::fmt::Display) -> Self {
        tracing::error!("internal error: {err}");
        Self::new(UiErrorCode::Internal, err.to_string())
    }
}

impl From<SendError> for UiError {
    fn from(err: SendError) -> Self {
        let code = match &err {
            SendError::Empty => UiErrorCode::Empty,
            SendError::TooLarge => UiErrorCode::TooLarge,
            SendError::NoTarget => UiErrorCode::NoTarget,
            SendError::TargetOffline { .. } => UiErrorCode::TargetOffline,
            SendError::Stopped => UiErrorCode::HubDown,
            SendError::Store(_) => return Self::internal(err),
        };
        Self::new(code, err.to_string())
    }
}

impl From<HubError> for UiError {
    fn from(err: HubError) -> Self {
        let code = match &err {
            HubError::UnknownSession => UiErrorCode::UnknownSession,
            HubError::SessionOffline => UiErrorCode::TargetOffline,
            HubError::SessionLive => UiErrorCode::SessionLive,
            HubError::Stopped => UiErrorCode::HubDown,
            HubError::Bind(_) | HubError::Store(_) => return Self::internal(err),
        };
        Self::new(code, err.to_string())
    }
}

impl From<CliError> for UiError {
    fn from(err: CliError) -> Self {
        match err {
            CliError::Conflict(_) => Self::new(UiErrorCode::CliConflict, err.to_string()),
            CliError::Plugin { .. } => Self::new(UiErrorCode::PluginInstall, err.to_string()),
            CliError::Io(_) => Self::internal(err),
        }
    }
}

impl From<SettingsError> for UiError {
    fn from(err: SettingsError) -> Self {
        match err {
            SettingsError::Invalid(message) => Self::new(UiErrorCode::InvalidSettings, message),
            SettingsError::Io(_) => Self::internal(err),
        }
    }
}

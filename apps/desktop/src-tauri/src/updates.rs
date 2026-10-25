//! Self-update of the stable app from the public GitHub releases (PLAN.md D56, D83–D85, §23.3).

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::events::{self, UPDATE_STATUS};
use crate::tray;

pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export)]
pub enum UpdateState {
    /// Not checked since launch.
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
        notes: String,
    },
    Installing {
        version: String,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateStatus {
    pub state: UpdateState,
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub checked_at: Option<OffsetDateTime>,
}

pub struct Updates {
    status: Mutex<UpdateStatus>,
    found: Mutex<Option<Update>>,
}

impl Updates {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus {
                state: UpdateState::Idle,
                checked_at: None,
            }),
            found: Mutex::new(None),
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().expect("update status lock").clone()
    }

    /// Changes the status, tells every window and keeps the menu bar's update item in step.
    fn set(&self, app: &AppHandle, change: impl FnOnce(&mut UpdateStatus)) -> UpdateStatus {
        let status = {
            let mut status = self.status.lock().expect("update status lock");
            change(&mut status);
            status.clone()
        };
        let available = match &status.state {
            UpdateState::Available { version, .. } => Some(version.as_str()),
            _ => None,
        };
        tray::show_update(app, available);
        events::emit(app, UPDATE_STATUS, status.clone());
        status
    }

    pub async fn check(&self, app: &AppHandle) -> UpdateStatus {
        if matches!(
            self.status().state,
            UpdateState::Checking | UpdateState::Installing { .. }
        ) {
            return self.status();
        }
        self.set(app, |s| s.state = UpdateState::Checking);
        let result = match app.updater() {
            Ok(updater) => updater.check().await,
            Err(err) => Err(err),
        };
        let state = match result {
            Ok(Some(update)) => {
                let state = UpdateState::Available {
                    version: update.version.clone(),
                    notes: update.body.clone().unwrap_or_default(),
                };
                self.found.lock().expect("found update lock").replace(update);
                state
            }
            Ok(None) => UpdateState::UpToDate,
            Err(err) => {
                tracing::warn!("update check failed: {err}");
                UpdateState::Failed {
                    message: err.to_string(),
                }
            }
        };
        self.set(app, |s| {
            s.state = state;
            s.checked_at = Some(OffsetDateTime::now_utc());
        })
    }

    /// Downloads, verifies and installs the update the last check found, then relaunches.
    pub async fn install(&self, app: &AppHandle) -> Result<(), String> {
        let update = self
            .found
            .lock()
            .expect("found update lock")
            .take()
            .ok_or("No update to install; check for updates first")?;
        let version = update.version.clone();
        self.set(app, |s| s.state = UpdateState::Installing { version });
        match update.download_and_install(|_, _| {}, || {}).await {
            Ok(()) => app.restart(),
            Err(err) => {
                let message = err.to_string();
                self.set(app, |s| {
                    s.state = UpdateState::Failed {
                        message: message.clone(),
                    }
                });
                Err(message)
            }
        }
    }
}

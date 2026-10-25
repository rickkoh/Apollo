//! Self-update of the stable app (PLAN.md D56, D57, §23.3): the feed and the download carry the GitLab deploy token
//! kept in the Keychain.

use std::sync::Mutex;
use std::time::Duration;

use security_framework::passwords;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::events::{self, UPDATE_STATUS};
use crate::tray;

pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// The request header GitLab reads a deploy token from (S7).
const TOKEN_HEADER: &str = "Deploy-Token";
const TOKEN_ACCOUNT: &str = "gitlab-deploy-token";
/// `errSecItemNotFound`.
const NOT_FOUND: i32 = -25300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export)]
pub enum UpdateState {
    /// Not checked since launch or since the token changed.
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
    /// A deploy token is in the Keychain; without one, updates are off (D57).
    pub token: bool,
    pub state: UpdateState,
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub checked_at: Option<OffsetDateTime>,
}

pub struct Updates {
    /// The Keychain service holding the token: the bundle identifier.
    service: String,
    status: Mutex<UpdateStatus>,
    found: Mutex<Option<Update>>,
}

impl Updates {
    pub fn new(service: String) -> Self {
        let token = read_token(&service).is_some();
        Self {
            service,
            status: Mutex::new(UpdateStatus {
                token,
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

    /// Stores the token, or removes it when `None`; a check follows separately.
    pub fn save_token(&self, app: &AppHandle, token: Option<&str>) -> Result<UpdateStatus, String> {
        match token.map(str::trim) {
            Some("") => return Err("The token is empty".into()),
            Some(token) => passwords::set_generic_password(&self.service, TOKEN_ACCOUNT, token.as_bytes())
                .map_err(|err| err.to_string())?,
            None => match passwords::delete_generic_password(&self.service, TOKEN_ACCOUNT) {
                Err(err) if err.code() != NOT_FOUND => return Err(err.to_string()),
                _ => {}
            },
        }
        self.found.lock().expect("found update lock").take();
        Ok(self.set(app, |s| {
            s.token = token.is_some();
            s.state = UpdateState::Idle;
            s.checked_at = None;
        }))
    }

    pub async fn check(&self, app: &AppHandle) -> UpdateStatus {
        let Some(token) = read_token(&self.service) else {
            return self.set(app, |s| {
                s.token = false;
                s.state = UpdateState::Idle;
            });
        };
        if matches!(
            self.status().state,
            UpdateState::Checking | UpdateState::Installing { .. }
        ) {
            return self.status();
        }
        self.set(app, |s| {
            s.token = true;
            s.state = UpdateState::Checking;
        });
        let result = match app.updater_builder().header(TOKEN_HEADER, token) {
            Ok(builder) => match builder.build() {
                Ok(updater) => updater.check().await,
                Err(err) => Err(err),
            },
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

fn read_token(service: &str) -> Option<String> {
    match passwords::get_generic_password(service, TOKEN_ACCOUNT) {
        Ok(bytes) => String::from_utf8(bytes).ok(),
        Err(err) => {
            if err.code() != NOT_FOUND {
                tracing::warn!("reading the deploy token from the Keychain failed: {err}");
            }
            None
        }
    }
}

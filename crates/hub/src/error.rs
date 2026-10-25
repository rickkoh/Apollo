use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored timestamp: {0}")]
    Time(String),
    #[error("stored value: {0}")]
    Corrupt(String),
    /// A newer app migrated the database; this one must not touch it (D59).
    #[error("the database is at schema version {found}, newer than this app's {known}")]
    NewerSchema { found: i64, known: i64 },
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("the message is empty")]
    Empty,
    #[error("the message is too long")]
    TooLarge,
    #[error("no session is selected")]
    NoTarget,
    #[error("{title} isn't connected")]
    TargetOffline { title: String },
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("the hub has stopped")]
    Stopped,
}

#[derive(Debug, Error)]
pub enum HubError {
    #[error("unknown session")]
    UnknownSession,
    #[error("the session isn't connected")]
    SessionOffline,
    #[error("the session is still connected")]
    SessionLive,
    #[error("cannot bind the hub: {0}")]
    Bind(#[from] std::io::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("the hub has stopped")]
    Stopped,
}

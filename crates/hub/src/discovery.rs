//! The discovery file through which channels find the hub (PLAN.md §5.8).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::Token;

pub const DISCOVERY_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Discovery {
    pub version: u32,
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

/// Writes the file with mode 0600 through a temp file and a rename, so readers never see half of it.
pub fn write(path: &Path, port: u16, token: &Token) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let discovery = Discovery {
        version: DISCOVERY_VERSION,
        port,
        token: token.expose().to_owned(),
        pid: std::process::id(),
    };
    let tmp = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(serde_json::to_string(&discovery)?.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)
}

pub fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

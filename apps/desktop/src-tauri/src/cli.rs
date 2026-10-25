//! The `cv` launcher the app installs into `~/.local/bin` (PLAN.md D18, §7.6).

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

pub const MARKER: &str = "# voice-bridge launcher";
const COMMAND: &str = "cv";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CliStatus {
    /// Our launcher is at `path`.
    pub installed: bool,
    /// It points at this app's plugin; false means a reinstall is due.
    pub up_to_date: bool,
    pub path: String,
    /// The folder is on the user's login-shell `PATH`.
    pub on_path: bool,
    /// A `cv` we didn't write is in the way.
    pub foreign: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0} already exists and wasn't written by Voice Bridge")]
    Conflict(String),
    #[error("cannot write the launcher: {0}")]
    Io(#[from] std::io::Error),
}

fn single_quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

pub fn script(plugin_dir: &Path) -> String {
    format!(
        "#!/bin/sh\n{MARKER}\nexec claude --plugin-dir {} --dangerously-load-development-channels plugin:voice@inline \"$@\"\n",
        single_quoted(plugin_dir)
    )
}

pub struct Launcher {
    pub bin_dir: PathBuf,
    pub plugin_dir: PathBuf,
}

impl Launcher {
    fn target(&self) -> PathBuf {
        self.bin_dir.join(COMMAND)
    }

    pub fn status(&self, path_env: Option<&OsStr>) -> CliStatus {
        let target = self.target();
        let existing = fs::read_to_string(&target).ok();
        let ours = existing
            .as_deref()
            .is_some_and(|text| text.lines().nth(1) == Some(MARKER));
        CliStatus {
            installed: ours,
            up_to_date: ours && existing.as_deref() == Some(script(&self.plugin_dir).as_str()),
            path: target.to_string_lossy().into_owned(),
            on_path: path_env.is_some_and(|paths| std::env::split_paths(paths).any(|p| p == self.bin_dir)),
            foreign: target.exists() && !ours,
        }
    }

    /// Writes or rewrites our launcher; never overwrites a file without the marker.
    pub fn install(&self, path_env: Option<&OsStr>) -> Result<CliStatus, CliError> {
        let target = self.target();
        if self.status(path_env).foreign {
            return Err(CliError::Conflict(target.to_string_lossy().into_owned()));
        }
        fs::create_dir_all(&self.bin_dir)?;
        let tmp = self.bin_dir.join(".cv.voice-bridge.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(&tmp)?;
        file.write_all(script(&self.plugin_dir).as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, &target)?;
        Ok(self.status(path_env))
    }
}

/// `name` as the user's login shell sets it; a GUI app's own environment lacks what shell profiles add.
pub fn login_shell_var(name: &str) -> Option<std::ffi::OsString> {
    const TAG: &str = "__VOICE_BRIDGE_VAR__";
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    let output = std::process::Command::new(shell)
        .args(["-ilc", &format!("printf '{TAG}%s' \"${name}\"")])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.rsplit_once(TAG)
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .map(Into::into)
        .or_else(|| std::env::var_os(name))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn launcher(dir: &tempfile::TempDir) -> Launcher {
        Launcher {
            bin_dir: dir.path().join("bin"),
            plugin_dir: PathBuf::from("/Apps/Voice Bridge.app/plugin/voice"),
        }
    }

    #[test]
    fn the_script_carries_the_marker_and_a_quoted_plugin_dir() {
        let text = script(Path::new("/a/it's here/voice"));
        assert!(text.starts_with("#!/bin/sh\n# voice-bridge launcher\n"));
        assert!(text.contains(
            r"--plugin-dir '/a/it'\''s here/voice' --dangerously-load-development-channels plugin:voice@inline"
        ));
    }

    #[test]
    fn install_writes_an_executable_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        assert_eq!(
            l.status(None),
            CliStatus {
                installed: false,
                up_to_date: false,
                path: dir.path().join("bin/cv").to_string_lossy().into_owned(),
                on_path: false,
                foreign: false,
            }
        );
        let status = l.install(None).unwrap();
        assert!(status.installed && status.up_to_date && !status.foreign);
        let mode = fs::metadata(dir.path().join("bin/cv")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn install_refuses_a_foreign_cv_and_leaves_it_alone() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        fs::create_dir_all(&l.bin_dir).unwrap();
        fs::write(l.bin_dir.join("cv"), "#!/bin/sh\necho mine\n").unwrap();
        assert!(l.status(None).foreign);
        assert!(matches!(l.install(None), Err(CliError::Conflict(_))));
        assert_eq!(
            fs::read_to_string(l.bin_dir.join("cv")).unwrap(),
            "#!/bin/sh\necho mine\n"
        );
    }

    #[test]
    fn an_outdated_launcher_of_ours_is_reinstalled() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        fs::create_dir_all(&l.bin_dir).unwrap();
        fs::write(l.bin_dir.join("cv"), script(Path::new("/old/place"))).unwrap();
        let before = l.status(None);
        assert!(before.installed && !before.up_to_date);
        assert!(l.install(None).unwrap().up_to_date);
    }

    #[test]
    fn on_path_reads_the_given_path() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        let with = std::env::join_paths([PathBuf::from("/usr/bin"), l.bin_dir.clone()]).unwrap();
        assert!(l.status(Some(&with)).on_path);
        assert!(!l.status(Some(OsStr::new("/usr/bin:/bin"))).on_path);
    }
}

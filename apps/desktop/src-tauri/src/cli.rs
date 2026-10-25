//! The `cv` (or Apollo Dev's `cv-dev`) launcher the app installs into `~/.local/bin` (PLAN.md D18, §7.6, §22.2, D75).

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

pub const MARKER: &str = "# apollo launcher";
/// The managed-settings drop-in that approves the voice channel (D45); the user writes it once with `sudo`.
pub const APPROVAL_PATH: &str = "/Library/Application Support/ClaudeCode/managed-settings.d/apollo.json";
pub const MARKETPLACE: &str = "apollo";
pub const PLUGIN_ID: &str = "voice@apollo";
const CHANNEL: &str = "plugin:voice@apollo";
/// Our `allowedChannelPlugins` entry exactly as the approval command writes it; the launcher greps for this text.
const APPROVAL_ENTRY: &str = r#"{"marketplace":"apollo","plugin":"voice"}"#;
/// The variable every launcher sets to its app's discovery file, unless it is already set (D52).
pub const DISCOVERY_VAR: &str = "APOLLO_DISCOVERY";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ApprovalState {
    Approved,
    Missing,
    /// The file lacks our entry as the approval command writes it, so `cv` keeps the dialog.
    WrongEntry,
    Unreadable,
    /// `managed-settings.json` sets its own `allowedChannelPlugins` without ours and may replace it.
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CliStatus {
    /// `cv`, or `cv-dev` for Apollo Dev.
    pub command: String,
    /// Our launcher is at `path`.
    pub installed: bool,
    /// It is this app's launcher script; false means a reinstall is due.
    pub up_to_date: bool,
    pub path: String,
    /// The folder is on the user's login-shell `PATH`.
    pub on_path: bool,
    /// A `cv` we didn't write is in the way.
    pub foreign: bool,
    pub approval: ApprovalState,
    /// The `sudo` command that writes the approval file.
    pub approval_command: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0} already exists and wasn't written by Apollo")]
    Conflict(String),
    #[error("cannot write the launcher: {0}")]
    Io(#[from] std::io::Error),
    #[error("`{step}` failed: {stderr}")]
    Plugin { step: String, stderr: String },
}

pub(crate) fn single_quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

/// A launcher's first lines: the marker, then its app's discovery file.
pub(crate) fn header(discovery: &Path) -> String {
    format!(
        "#!/bin/sh\n\
         {MARKER}\n\
         [ -n \"${DISCOVERY_VAR}\" ] || {DISCOVERY_VAR}={}\n\
         export {DISCOVERY_VAR}\n",
        single_quoted(discovery)
    )
}

/// The plugin a launcher starts claude with.
pub enum Route {
    /// The installed plugin (D44); only an approval file holding our entry skips the dialog.
    Installed,
    /// The working copy's `plugin/voice` with the development flag (D75).
    WorkingCopy { plugin_dir: PathBuf },
}

impl Route {
    fn command(&self) -> &'static str {
        match self {
            Route::Installed => "cv",
            Route::WorkingCopy { .. } => "cv-dev",
        }
    }
}

pub fn script(route: &Route, approval_path: &Path, discovery: &Path) -> String {
    let body = match route {
        Route::Installed => {
            let plugin = format!("--settings '{{\"enabledPlugins\":{{\"{PLUGIN_ID}\":true}}}}'");
            format!(
                "if grep -qF '{APPROVAL_ENTRY}' {approval} 2>/dev/null; then\n    \
                 exec claude {plugin} --channels {CHANNEL} \"$@\"\n\
                 fi\n\
                 exec claude {plugin} --dangerously-load-development-channels {CHANNEL} \"$@\"\n",
                approval = single_quoted(approval_path),
            )
        }
        Route::WorkingCopy { plugin_dir } => format!(
            "exec claude --plugin-dir {} --dangerously-load-development-channels plugin:voice@inline \"$@\"\n",
            single_quoted(plugin_dir)
        ),
    };
    header(discovery) + &body
}

fn approval_json() -> String {
    format!(r#"{{"allowedChannelPlugins":[{APPROVAL_ENTRY}]}}"#)
}

fn approval_command(approval_path: &Path) -> String {
    format!(
        "sudo mkdir -p {} && echo '{}' | sudo tee {} > /dev/null",
        single_quoted(approval_path.parent().unwrap_or(approval_path)),
        approval_json(),
        single_quoted(approval_path)
    )
}

fn allows_voice(settings: &Value) -> Option<bool> {
    let entries = settings.get("allowedChannelPlugins")?.as_array()?;
    Some(
        entries
            .iter()
            .any(|e| e["marketplace"] == MARKETPLACE && e["plugin"] == "voice"),
    )
}

pub fn approval_state(approval_path: &Path) -> ApprovalState {
    let text = match fs::read_to_string(approval_path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return ApprovalState::Missing,
        Err(_) => return ApprovalState::Unreadable,
    };
    let Ok(settings) = serde_json::from_str::<Value>(&text) else {
        return ApprovalState::Unreadable;
    };
    if allows_voice(&settings) != Some(true) || !text.contains(APPROVAL_ENTRY) {
        return ApprovalState::WrongEntry;
    }
    let managed = approval_path
        .parent()
        .and_then(Path::parent)
        .map(|dir| dir.join("managed-settings.json"));
    let overriding = managed
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|settings| allows_voice(&settings));
    if overriding == Some(false) {
        return ApprovalState::Conflict;
    }
    ApprovalState::Approved
}

/// A launcher file of ours: recognised by the marker on line 2, written atomically, never over someone else's file.
pub(crate) struct OurScript {
    pub target: PathBuf,
}

pub(crate) struct ScriptState {
    pub installed: bool,
    pub up_to_date: bool,
    pub foreign: bool,
}

impl OurScript {
    pub fn state(&self, expected: &str) -> ScriptState {
        let existing = fs::read_to_string(&self.target).ok();
        let ours = existing
            .as_deref()
            .is_some_and(|text| text.lines().nth(1) == Some(MARKER));
        ScriptState {
            installed: ours,
            up_to_date: ours && existing.as_deref() == Some(expected),
            foreign: self.target.exists() && !ours,
        }
    }

    pub fn write(&self, text: &str) -> Result<(), CliError> {
        if self.state(text).foreign {
            return Err(CliError::Conflict(self.target.to_string_lossy().into_owned()));
        }
        let dir = self.target.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(dir)?;
        let name = self.target.file_name().unwrap_or_default().to_string_lossy();
        let tmp = dir.join(format!(".{name}.apollo.tmp"));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, &self.target)?;
        Ok(())
    }
}

pub struct Launcher {
    pub bin_dir: PathBuf,
    pub approval_path: PathBuf,
    pub discovery: PathBuf,
    pub route: Route,
}

impl Launcher {
    fn script(&self) -> String {
        script(&self.route, &self.approval_path, &self.discovery)
    }

    fn file(&self) -> OurScript {
        OurScript {
            target: self.bin_dir.join(self.route.command()),
        }
    }

    pub fn status(&self, path_env: Option<&OsStr>) -> CliStatus {
        let file = self.file();
        let state = file.state(&self.script());
        CliStatus {
            command: self.route.command().into(),
            installed: state.installed,
            up_to_date: state.up_to_date,
            path: file.target.to_string_lossy().into_owned(),
            on_path: path_env.is_some_and(|paths| std::env::split_paths(paths).any(|p| p == self.bin_dir)),
            foreign: state.foreign,
            approval: approval_state(&self.approval_path),
            approval_command: approval_command(&self.approval_path),
        }
    }

    /// Writes or rewrites our launcher; never overwrites a file without the marker.
    pub fn install(&self, path_env: Option<&OsStr>) -> Result<CliStatus, CliError> {
        self.file().write(&self.script())?;
        Ok(self.status(path_env))
    }
}

/// This checkout's `plugin/voice`, which Apollo Dev's `cv-dev` loads (D75).
pub fn working_copy_plugin() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../plugin/voice");
    dir.canonicalize().unwrap_or(dir)
}

/// `name` as the user's login shell sets it; a GUI app's own environment lacks what shell profiles add.
pub fn login_shell_var(name: &str) -> Option<std::ffi::OsString> {
    const TAG: &str = "__APOLLO_VAR__";
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

    struct Fixture {
        dir: tempfile::TempDir,
        launcher: Launcher,
    }

    fn fixture_with(route: Route) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let launcher = Launcher {
            bin_dir: dir.path().join("bin"),
            approval_path: dir.path().join("ClaudeCode/managed-settings.d/apollo.json"),
            discovery: dir.path().join("data/hub.json"),
            route,
        };
        Fixture { dir, launcher }
    }

    fn fixture() -> Fixture {
        fixture_with(Route::Installed)
    }

    impl Fixture {
        fn approve(&self, text: &str) {
            fs::create_dir_all(self.launcher.approval_path.parent().unwrap()).unwrap();
            fs::write(&self.launcher.approval_path, text).unwrap();
        }

        fn managed(&self, text: &str) {
            fs::write(self.dir.path().join("ClaudeCode/managed-settings.json"), text).unwrap();
        }

        fn approval(&self) -> ApprovalState {
            approval_state(&self.launcher.approval_path)
        }

        /// Runs the installed launcher with a stub `claude` that records the discovery variable, then its arguments.
        fn run(&self, discovery: Option<&str>) -> Vec<String> {
            let stub = self.dir.path().join("stub");
            fs::create_dir_all(&stub).unwrap();
            let claude = stub.join("claude");
            let record = format!("#!/bin/sh\nprintf '%s\\n' \"${DISCOVERY_VAR}\" \"$@\" > \"$ARGV_OUT\"\n");
            fs::write(&claude, record).unwrap();
            fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
            let out = self.dir.path().join("argv");
            let mut command = std::process::Command::new(self.launcher.bin_dir.join(self.launcher.route.command()));
            command
                .args(["--resume", "it's"])
                .env("PATH", format!("{}:/usr/bin:/bin", stub.display()))
                .env("ARGV_OUT", &out)
                .env_remove(DISCOVERY_VAR);
            if let Some(discovery) = discovery {
                command.env(DISCOVERY_VAR, discovery);
            }
            assert!(command.status().unwrap().success());
            fs::read_to_string(out).unwrap().lines().map(str::to_owned).collect()
        }

        fn run_cv(&self) -> Vec<String> {
            self.run(None).split_off(1)
        }
    }

    fn argv(route: &str) -> Vec<String> {
        [
            "--settings",
            r#"{"enabledPlugins":{"voice@apollo":true}}"#,
            route,
            "plugin:voice@apollo",
            "--resume",
            "it's",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn the_script_carries_the_marker_and_quoted_paths() {
        let text = script(
            &Route::Installed,
            Path::new("/a/it's here/apollo.json"),
            Path::new("/d/it's/hub.json"),
        );
        assert!(text.starts_with("#!/bin/sh\n# apollo launcher\n"));
        assert!(text.contains(r"'/a/it'\''s here/apollo.json'"));
        assert!(text.contains(r"'/d/it'\''s/hub.json'"));
    }

    #[test]
    fn a_launcher_points_its_session_at_its_apps_hub_unless_one_is_set() {
        let f = fixture();
        f.launcher.install(None).unwrap();
        assert_eq!(f.run(None)[0], f.launcher.discovery.to_string_lossy());
        assert_eq!(f.run(Some("/tmp/vb/hub.json"))[0], "/tmp/vb/hub.json");
    }

    #[test]
    fn cv_dev_runs_the_working_copy_with_the_development_flag() {
        let f = fixture_with(Route::WorkingCopy {
            plugin_dir: PathBuf::from("/repo/plugin/voice"),
        });
        let status = f.launcher.install(None).unwrap();
        assert_eq!(status.command, "cv-dev");
        assert!(status.installed && status.path.ends_with("bin/cv-dev"));
        assert!(!f.dir.path().join("bin/cv").exists());
        assert_eq!(
            f.run(None),
            [
                &f.launcher.discovery.to_string_lossy(),
                "--plugin-dir",
                "/repo/plugin/voice",
                "--dangerously-load-development-channels",
                "plugin:voice@inline",
                "--resume",
                "it's",
            ]
        );
    }

    #[test]
    fn cv_takes_the_approved_route_only_with_our_entry() {
        let f = fixture();
        f.launcher.install(None).unwrap();
        assert_eq!(f.run_cv(), argv("--dangerously-load-development-channels"), "no file");
        f.approve(r#"{"allowedChannelPlugins":[{"marketplace":"inline","plugin":"voice"}]}"#);
        assert_eq!(
            f.run_cv(),
            argv("--dangerously-load-development-channels"),
            "wrong entry"
        );
        f.approve(&approval_json());
        assert_eq!(f.run_cv(), argv("--channels"), "approved");
    }

    #[test]
    fn approval_states() {
        let f = fixture();
        assert_eq!(f.approval(), ApprovalState::Missing);
        f.approve("{ not json");
        assert_eq!(f.approval(), ApprovalState::Unreadable);
        f.approve(r#"{"allowedChannelPlugins":[{"marketplace":"apollo","plugin":"other"}]}"#);
        assert_eq!(f.approval(), ApprovalState::WrongEntry);
        f.approve(r#"{ "allowedChannelPlugins": [{ "marketplace": "apollo", "plugin": "voice" }] }"#);
        assert_eq!(
            f.approval(),
            ApprovalState::WrongEntry,
            "hand-formatted: the launcher's grep misses it"
        );
        f.approve(&approval_json());
        assert_eq!(f.approval(), ApprovalState::Approved);
        f.managed(r#"{"allowedChannelPlugins":[{"marketplace":"acme","plugin":"chat"}]}"#);
        assert_eq!(f.approval(), ApprovalState::Conflict);
        f.managed(r#"{"allowedChannelPlugins":[{"marketplace":"apollo","plugin":"voice"}]}"#);
        assert_eq!(f.approval(), ApprovalState::Approved);
        f.managed(r#"{"permissions":{}}"#);
        assert_eq!(f.approval(), ApprovalState::Approved);
    }

    #[test]
    fn the_approval_command_writes_the_path_cv_checks() {
        let command = approval_command(Path::new(APPROVAL_PATH));
        assert_eq!(
            command,
            "sudo mkdir -p '/Library/Application Support/ClaudeCode/managed-settings.d' && echo \
             '{\"allowedChannelPlugins\":[{\"marketplace\":\"apollo\",\"plugin\":\"voice\"}]}' | sudo tee \
             '/Library/Application Support/ClaudeCode/managed-settings.d/apollo.json' > /dev/null"
        );
    }

    #[test]
    fn install_writes_an_executable_launcher() {
        let f = fixture();
        let before = f.launcher.status(None);
        assert!(!before.installed && !before.up_to_date && !before.foreign && !before.on_path);
        assert_eq!(before.path, f.dir.path().join("bin/cv").to_string_lossy());
        let status = f.launcher.install(None).unwrap();
        assert!(status.installed && status.up_to_date && !status.foreign);
        let mode = fs::metadata(f.dir.path().join("bin/cv")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn install_refuses_a_foreign_cv_and_leaves_it_alone() {
        let f = fixture();
        let l = &f.launcher;
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
        let f = fixture();
        let l = &f.launcher;
        fs::create_dir_all(&l.bin_dir).unwrap();
        let inline = format!("#!/bin/sh\n{MARKER}\nexec claude --plugin-dir '/old' plugin:voice@inline \"$@\"\n");
        fs::write(l.bin_dir.join("cv"), inline).unwrap();
        let before = l.status(None);
        assert!(before.installed && !before.up_to_date);
        assert!(l.install(None).unwrap().up_to_date);
    }

    #[test]
    fn on_path_reads_the_given_path() {
        let f = fixture();
        let l = &f.launcher;
        let with = std::env::join_paths([PathBuf::from("/usr/bin"), l.bin_dir.clone()]).unwrap();
        assert!(l.status(Some(&with)).on_path);
        assert!(!l.status(Some(OsStr::new("/usr/bin:/bin"))).on_path);
    }
}

//! `cv-panel`, the launcher an editor's Claude Code panel runs through `claudeCode.claudeProcessWrapper` (PLAN.md §24).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::cli::{CliError, OurScript, header, single_quoted};

pub const SETTING: &str = "claudeCode.claudeProcessWrapper";
const COMMAND: &str = "cv-panel";
/// Editors whose Claude Code extension reads `SETTING`, with their user settings folder in Application Support.
const EDITORS: [(&str, &str); 2] = [("Cursor", "Cursor/User"), ("VS Code", "Code/User")];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PanelWrapper {
    Ours,
    Unset,
    /// Another wrapper is set; the app leaves it alone.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditorPanel {
    pub name: String,
    pub settings_path: String,
    pub wrapper: PanelWrapper,
    pub current: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IdePanelStatus {
    pub installed: bool,
    pub up_to_date: bool,
    pub path: String,
    pub foreign: bool,
    /// The settings line that points an editor's panel at `cv-panel`.
    pub setting: String,
    /// Editors installed on this Mac.
    pub editors: Vec<EditorPanel>,
}

/// Sessions (their arguments carry `--input-format`) go through the bridge; every other call, and a session the
/// bridge can't start, runs unchanged.
pub fn script(bridge: &Path, discovery: &Path) -> String {
    let bridge = single_quoted(bridge);
    header(discovery)
        + &format!(
            "case \" $* \" in\n    \
         *\" --input-format \"*|*\" --input-format=\"*)\n        \
         if [ ! -f {bridge} ]; then echo \"apollo: voice off, the panel bridge is missing\" >&2\n        \
         elif ! command -v node >/dev/null 2>&1; then echo \"apollo: voice off, node is not on PATH\" >&2\n        \
         else exec node {bridge} \"$@\"; fi ;;\n\
         esac\n\
         exec \"$@\"\n"
        )
}

/// The wrapper an editor's `settings.json` sets, skipping `//` comment lines; settings files are JSON with comments.
fn configured_wrapper(settings: &str) -> Option<String> {
    let key = format!("\"{SETTING}\"");
    settings
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .find_map(|line| {
            let rest = line.split_once(&key)?.1.trim_start().strip_prefix(':')?;
            serde_json::Deserializer::from_str(rest)
                .into_iter::<String>()
                .next()?
                .ok()
        })
}

pub struct IdePanelLauncher {
    pub bin_dir: PathBuf,
    /// The bundled `panel-bridge.js`.
    pub bridge: PathBuf,
    pub discovery: PathBuf,
    /// `~/Library/Application Support`, where the editors keep their settings.
    pub app_support: PathBuf,
}

impl IdePanelLauncher {
    fn file(&self) -> OurScript {
        OurScript {
            target: self.bin_dir.join(COMMAND),
        }
    }

    fn editor(&self, name: &str, folder: &str, ours: &str) -> Option<EditorPanel> {
        let dir = self.app_support.join(folder);
        if !dir.is_dir() {
            return None;
        }
        let settings_path = dir.join("settings.json");
        let current = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|text| configured_wrapper(&text));
        let wrapper = match current.as_deref() {
            None => PanelWrapper::Unset,
            Some(path) if path == ours => PanelWrapper::Ours,
            Some(_) => PanelWrapper::Other,
        };
        Some(EditorPanel {
            name: name.into(),
            settings_path: settings_path.to_string_lossy().into_owned(),
            wrapper,
            current,
        })
    }

    pub fn status(&self) -> IdePanelStatus {
        let file = self.file();
        let state = file.state(&script(&self.bridge, &self.discovery));
        let path = file.target.to_string_lossy().into_owned();
        IdePanelStatus {
            installed: state.installed,
            up_to_date: state.up_to_date,
            foreign: state.foreign,
            setting: format!("\"{SETTING}\": {}", serde_json::to_string(&path).unwrap_or_default()),
            editors: EDITORS
                .iter()
                .filter_map(|(name, folder)| self.editor(name, folder, &path))
                .collect(),
            path,
        }
    }

    pub fn install(&self) -> Result<IdePanelStatus, CliError> {
        self.file().write(&script(&self.bridge, &self.discovery))?;
        Ok(self.status())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn launcher(dir: &tempfile::TempDir) -> IdePanelLauncher {
        let bridge = dir.path().join("Apollo.app/plugin/voice/dist/panel-bridge.js");
        fs::create_dir_all(bridge.parent().unwrap()).unwrap();
        fs::write(&bridge, "").unwrap();
        IdePanelLauncher {
            bin_dir: dir.path().join("bin"),
            bridge,
            discovery: dir.path().join("data/hub.json"),
            app_support: dir.path().join("Application Support"),
        }
    }

    #[test]
    fn the_launcher_points_sessions_at_its_apps_hub() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        assert!(script(&l.bridge, &l.discovery).starts_with(&header(&l.discovery)));
    }

    fn executable(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Runs `cv-panel <claude> <args>` with stubs for `claude` and, given `node`, `node` that record their arguments.
    fn run(dir: &tempfile::TempDir, node: bool, args: &[&str]) -> (String, Vec<String>) {
        let stub = dir.path().join("stub");
        let record = "#!/bin/sh\necho \"$0\" > \"$OUT\"\nprintf '%s\\n' \"$@\" >> \"$OUT\"\n";
        executable(&stub.join("claude"), record);
        if node {
            executable(&stub.join("node"), record);
        } else {
            let _ = fs::remove_file(stub.join("node"));
        }
        let out = dir.path().join("argv");
        let status = std::process::Command::new(dir.path().join("bin/cv-panel"))
            .arg(stub.join("claude"))
            .args(args)
            .env("PATH", format!("{}:/usr/bin:/bin", stub.display()))
            .env("OUT", &out)
            .status()
            .unwrap();
        assert!(status.success());
        let text = fs::read_to_string(out).unwrap();
        let mut lines = text.lines().map(str::to_owned);
        let program = lines.next().unwrap();
        (program.rsplit('/').next().unwrap().to_owned(), lines.collect())
    }

    #[test]
    fn a_session_goes_through_the_bridge_and_anything_else_runs_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        l.install().unwrap();
        let claude = dir.path().join("stub/claude").to_string_lossy().into_owned();
        let (program, argv) = run(
            &dir,
            true,
            &["--output-format", "stream-json", "--input-format", "stream-json"],
        );
        assert_eq!(program, "node");
        assert_eq!(
            argv,
            [
                l.bridge.to_string_lossy().as_ref(),
                &claude,
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json"
            ]
        );
        let (program, argv) = run(&dir, true, &["plugin", "list", "--json"]);
        assert_eq!(
            (program.as_str(), argv),
            ("claude", vec!["plugin".into(), "list".into(), "--json".into()])
        );
    }

    #[test]
    fn a_session_the_bridge_cant_start_runs_claude_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        l.install().unwrap();
        let args = ["--input-format", "stream-json"];
        let unchanged = ("claude".to_owned(), args.map(String::from).to_vec());
        assert_eq!(run(&dir, false, &args), unchanged, "no node");
        fs::remove_file(&l.bridge).unwrap();
        assert_eq!(run(&dir, true, &args), unchanged, "no bridge");
    }

    #[test]
    fn install_writes_an_up_to_date_launcher_and_refuses_a_foreign_one() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        assert!(!l.status().installed);
        let status = l.install().unwrap();
        assert!(status.installed && status.up_to_date && !status.foreign);
        fs::write(l.bin_dir.join("cv-panel"), "#!/bin/sh\necho mine\n").unwrap();
        assert!(l.status().foreign);
        assert!(matches!(l.install(), Err(CliError::Conflict(_))));
    }

    #[test]
    fn each_installed_editor_reports_its_wrapper() {
        let dir = tempfile::tempdir().unwrap();
        let l = launcher(&dir);
        let ours = l.status().path;
        let cursor = l.app_support.join("Cursor/User");
        fs::create_dir_all(&cursor).unwrap();
        let editors = |l: &IdePanelLauncher| {
            l.status()
                .editors
                .iter()
                .map(|e| (e.name.clone(), e.wrapper))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            editors(&l),
            [("Cursor".to_owned(), PanelWrapper::Unset)],
            "VS Code isn't installed"
        );

        let commented = format!("{{\n  // \"{SETTING}\": \"{ours}\",\n  \"editor.fontSize\": 13,\n}}\n");
        fs::write(cursor.join("settings.json"), commented).unwrap();
        assert_eq!(
            editors(&l)[0].1,
            PanelWrapper::Unset,
            "a commented-out line doesn't count"
        );

        fs::write(
            cursor.join("settings.json"),
            format!("{{\n  {},\n}}\n", l.status().setting),
        )
        .unwrap();
        assert_eq!(editors(&l)[0].1, PanelWrapper::Ours);

        fs::write(
            cursor.join("settings.json"),
            format!("{{ \"{SETTING}\" : \"/usr/local/bin/mine\" }}"),
        )
        .unwrap();
        let status = l.status();
        assert_eq!(
            (status.editors[0].wrapper, status.editors[0].current.as_deref()),
            (PanelWrapper::Other, Some("/usr/local/bin/mine"))
        );
    }

    #[test]
    fn the_setting_line_is_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let status = launcher(&dir).status();
        let parsed: serde_json::Value = serde_json::from_str(&format!("{{{}}}", status.setting)).unwrap();
        assert_eq!(parsed[SETTING], status.path);
    }
}

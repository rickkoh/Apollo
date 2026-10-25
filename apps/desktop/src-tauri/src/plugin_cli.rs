//! The voice plugin, installed from the app's bundled marketplace and disabled user-wide so only `cv` enables it.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cli::{CliError, MARKETPLACE, PLUGIN_ID};

const TIMEOUT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);
/// What `cv` runs from the installed copy; the rest of the plugin folder is metadata.
const RUNTIME_FILES: [&str; 2] = ["dist/channel.js", ".mcp.json"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub id: String,
    pub enabled: bool,
    pub install_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Marketplace {
    pub name: String,
    /// Directory marketplaces only.
    pub path: Option<PathBuf>,
}

/// Claude Code's plugin commands, behind a port so the installer can be tested without them.
pub trait PluginCli {
    fn plugins(&self) -> Result<Vec<InstalledPlugin>, CliError>;
    fn marketplaces(&self) -> Result<Vec<Marketplace>, CliError>;
    fn run(&self, args: &[&str]) -> Result<(), CliError>;
}

/// The user's `claude`, found on their login-shell `PATH` (D42).
pub struct ClaudeCli {
    path_env: Option<OsString>,
}

impl ClaudeCli {
    pub fn new(path_env: Option<OsString>) -> Self {
        Self { path_env }
    }

    fn output(&self, args: &[&str]) -> Result<String, CliError> {
        let failed = |stderr: String| CliError::Plugin {
            step: format!("claude {}", args.join(" ")),
            stderr,
        };
        let mut command = Command::new("claude");
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = &self.path_env {
            command.env("PATH", path);
        }
        let mut child = command
            .spawn()
            .map_err(|err| failed(format!("cannot run claude: {err}")))?;
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(failed(format!("no answer within {} s", TIMEOUT.as_secs())));
                }
                Err(err) => return Err(failed(err.to_string())),
            }
        };
        let stdout = stdout.join().unwrap_or_default();
        let stderr = stderr.join().unwrap_or_default();
        if status.success() {
            Ok(stdout)
        } else {
            Err(failed(
                if stderr.trim().is_empty() { stdout } else { stderr }.trim().to_owned(),
            ))
        }
    }

    fn json<T: for<'de> Deserialize<'de>>(&self, args: &[&str]) -> Result<T, CliError> {
        let text = self.output(args)?;
        serde_json::from_str(&text).map_err(|err| CliError::Plugin {
            step: format!("claude {}", args.join(" ")),
            stderr: format!("unexpected output: {err}"),
        })
    }
}

/// Reads a pipe to the end on its own thread, so a full pipe never blocks the child.
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    })
}

impl PluginCli for ClaudeCli {
    fn plugins(&self) -> Result<Vec<InstalledPlugin>, CliError> {
        self.json(&["plugin", "list", "--json"])
    }

    fn marketplaces(&self) -> Result<Vec<Marketplace>, CliError> {
        self.json(&["plugin", "marketplace", "list", "--json"])
    }

    fn run(&self, args: &[&str]) -> Result<(), CliError> {
        self.output(args).map(drop)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginStatus {
    pub installed: bool,
    /// From this app's marketplace, with its files, and disabled user-wide; false means a reinstall is due.
    pub up_to_date: bool,
}

/// The voice plugin as this copy of the app bundles it.
pub struct BundledPlugin {
    /// `<resource dir>/plugin`: a marketplace whose `voice/` is the plugin.
    pub marketplace: PathBuf,
}

fn same_path(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

fn same_contents(a: &Path, b: &Path) -> bool {
    matches!((std::fs::read(a), std::fs::read(b)), (Ok(a), Ok(b)) if a == b)
}

impl BundledPlugin {
    fn installed(cli: &dyn PluginCli) -> Result<Option<InstalledPlugin>, CliError> {
        Ok(cli.plugins()?.into_iter().find(|p| p.id == PLUGIN_ID))
    }

    fn marketplace_is_ours(&self, cli: &dyn PluginCli) -> Result<Option<bool>, CliError> {
        Ok(cli
            .marketplaces()?
            .into_iter()
            .find(|m| m.name == MARKETPLACE)
            .map(|m| m.path.is_some_and(|path| same_path(&path, &self.marketplace))))
    }

    fn has_our_files(&self, plugin: &InstalledPlugin) -> bool {
        let bundled = self.marketplace.join("voice");
        RUNTIME_FILES
            .iter()
            .all(|file| same_contents(&bundled.join(file), &plugin.install_path.join(file)))
    }

    pub fn status(&self, cli: &dyn PluginCli) -> Result<PluginStatus, CliError> {
        let ours = self.marketplace_is_ours(cli)? == Some(true);
        let plugin = Self::installed(cli)?;
        Ok(PluginStatus {
            installed: plugin.is_some(),
            up_to_date: ours && plugin.is_some_and(|p| !p.enabled && self.has_our_files(&p)),
        })
    }

    /// Brings the marketplace and plugin in line with this app; each step runs only when needed.
    pub fn install(&self, cli: &dyn PluginCli) -> Result<PluginStatus, CliError> {
        let root = self.marketplace.to_string_lossy();
        match self.marketplace_is_ours(cli)? {
            Some(true) => {}
            Some(false) => {
                cli.run(&["plugin", "marketplace", "remove", MARKETPLACE])?;
                cli.run(&["plugin", "marketplace", "add", &root])?;
            }
            None => cli.run(&["plugin", "marketplace", "add", &root])?,
        }
        // Removing a marketplace uninstalls its plugins, so read them only now.
        match Self::installed(cli)? {
            Some(plugin) if self.has_our_files(&plugin) => {}
            Some(_) => {
                cli.run(&["plugin", "uninstall", PLUGIN_ID, "--scope", "user"])?;
                cli.run(&["plugin", "install", PLUGIN_ID, "--scope", "user"])?;
            }
            None => cli.run(&["plugin", "install", PLUGIN_ID, "--scope", "user"])?,
        }
        if Self::installed(cli)?.is_some_and(|p| p.enabled) {
            cli.run(&["plugin", "disable", PLUGIN_ID, "--scope", "user"])?;
        }
        self.status(cli)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;

    use super::*;

    /// Claude Code's plugin state as the real commands change it, recording every write.
    struct FakeCli {
        cache: PathBuf,
        marketplace: RefCell<Option<PathBuf>>,
        plugin: RefCell<Option<InstalledPlugin>>,
        calls: RefCell<Vec<String>>,
        failing: Option<&'static str>,
    }

    impl FakeCli {
        fn new(dir: &Path) -> Self {
            Self {
                cache: dir.join("cache"),
                marketplace: RefCell::new(None),
                plugin: RefCell::new(None),
                calls: RefCell::new(Vec::new()),
                failing: None,
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.take()
        }
    }

    impl PluginCli for FakeCli {
        fn plugins(&self) -> Result<Vec<InstalledPlugin>, CliError> {
            Ok(self.plugin.borrow().iter().cloned().collect())
        }

        fn marketplaces(&self) -> Result<Vec<Marketplace>, CliError> {
            Ok(self
                .marketplace
                .borrow()
                .iter()
                .map(|path| Marketplace {
                    name: MARKETPLACE.into(),
                    path: Some(path.clone()),
                })
                .collect())
        }

        fn run(&self, args: &[&str]) -> Result<(), CliError> {
            let call = args.join(" ");
            self.calls.borrow_mut().push(call.clone());
            if self.failing.is_some_and(|f| call.contains(f)) {
                return Err(CliError::Plugin {
                    step: format!("claude {call}"),
                    stderr: "boom".into(),
                });
            }
            match args {
                ["plugin", "marketplace", "add", path] => *self.marketplace.borrow_mut() = Some(path.into()),
                ["plugin", "marketplace", "remove", _] => {
                    *self.marketplace.borrow_mut() = None;
                    *self.plugin.borrow_mut() = None;
                }
                ["plugin", "install", ..] => {
                    let source = self.marketplace.borrow().clone().unwrap().join("voice");
                    for file in RUNTIME_FILES {
                        fs::create_dir_all(self.cache.join(file).parent().unwrap()).unwrap();
                        fs::copy(source.join(file), self.cache.join(file)).unwrap();
                    }
                    *self.plugin.borrow_mut() = Some(InstalledPlugin {
                        id: PLUGIN_ID.into(),
                        enabled: true,
                        install_path: self.cache.clone(),
                    });
                }
                ["plugin", "uninstall", ..] => *self.plugin.borrow_mut() = None,
                ["plugin", "disable", ..] => self.plugin.borrow_mut().as_mut().unwrap().enabled = false,
                _ => panic!("unexpected call {call}"),
            }
            Ok(())
        }
    }

    fn bundle(dir: &Path, channel: &str) -> BundledPlugin {
        let marketplace = dir.join("Resources/plugin");
        fs::create_dir_all(marketplace.join("voice/dist")).unwrap();
        fs::write(marketplace.join("voice/dist/channel.js"), channel).unwrap();
        fs::write(marketplace.join("voice/.mcp.json"), "{}").unwrap();
        BundledPlugin { marketplace }
    }

    fn ready() -> PluginStatus {
        PluginStatus {
            installed: true,
            up_to_date: true,
        }
    }

    #[test]
    fn a_first_install_adds_the_marketplace_installs_and_disables() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = bundle(dir.path(), "v1");
        let cli = FakeCli::new(dir.path());
        assert_eq!(
            plugin.status(&cli).unwrap(),
            PluginStatus {
                installed: false,
                up_to_date: false
            }
        );
        assert_eq!(plugin.install(&cli).unwrap(), ready());
        let root = plugin.marketplace.to_string_lossy();
        assert_eq!(
            cli.calls(),
            [
                format!("plugin marketplace add {root}"),
                "plugin install voice@voice-bridge --scope user".into(),
                "plugin disable voice@voice-bridge --scope user".into(),
            ]
        );
    }

    #[test]
    fn a_second_install_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = bundle(dir.path(), "v1");
        let cli = FakeCli::new(dir.path());
        plugin.install(&cli).unwrap();
        cli.calls();
        assert_eq!(plugin.install(&cli).unwrap(), ready());
        assert!(cli.calls().is_empty());
    }

    #[test]
    fn a_marketplace_elsewhere_is_moved_to_this_app() {
        let dir = tempfile::tempdir().unwrap();
        let old = bundle(&dir.path().join("old"), "v1");
        let cli = FakeCli::new(dir.path());
        old.install(&cli).unwrap();
        let plugin = bundle(dir.path(), "v1");
        assert!(!plugin.status(&cli).unwrap().up_to_date);
        cli.calls();
        assert_eq!(plugin.install(&cli).unwrap(), ready());
        let root = plugin.marketplace.to_string_lossy();
        assert_eq!(
            cli.calls(),
            [
                "plugin marketplace remove voice-bridge".into(),
                format!("plugin marketplace add {root}"),
                "plugin install voice@voice-bridge --scope user".into(),
                "plugin disable voice@voice-bridge --scope user".into(),
            ]
        );
    }

    #[test]
    fn a_rebuilt_plugin_is_reinstalled() {
        let dir = tempfile::tempdir().unwrap();
        let cli = FakeCli::new(dir.path());
        bundle(dir.path(), "v1").install(&cli).unwrap();
        let plugin = bundle(dir.path(), "v2");
        assert_eq!(
            plugin.status(&cli).unwrap(),
            PluginStatus {
                installed: true,
                up_to_date: false
            }
        );
        cli.calls();
        assert_eq!(plugin.install(&cli).unwrap(), ready());
        assert_eq!(
            cli.calls(),
            [
                "plugin uninstall voice@voice-bridge --scope user",
                "plugin install voice@voice-bridge --scope user",
                "plugin disable voice@voice-bridge --scope user",
            ]
        );
    }

    #[test]
    fn a_failing_step_stops_the_install_with_its_output() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = bundle(dir.path(), "v1");
        let mut cli = FakeCli::new(dir.path());
        cli.failing = Some("install");
        let Err(CliError::Plugin { step, stderr }) = plugin.install(&cli) else {
            panic!("expected a plugin error")
        };
        assert_eq!(
            (step.as_str(), stderr.as_str()),
            ("claude plugin install voice@voice-bridge --scope user", "boom")
        );
        assert!(!cli.calls().iter().any(|c| c.contains("disable")));
    }
}

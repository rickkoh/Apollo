const COMMANDS: &[&str] = &[
    "hub_status",
    "list_sessions",
    "set_target",
    "send",
    "history",
    "mark_read",
    "rename_session",
    "forget_session",
    "clear_history",
    "get_settings",
    "update_settings",
    "overlay_resize",
    "overlay_visible",
    "set_overlay_visible",
    "open_panel",
    "recent_messages",
    "cli_status",
    "install_cli",
    "plugin_status",
    "panel_status",
    "about",
    "reveal_data_folder",
    "update_status",
    "check_for_updates",
    "install_update",
    "open_at_login",
    "set_open_at_login",
];

fn git(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The commit About shows; rebuilt when `HEAD` or the branch it names moves.
fn commit() -> String {
    let refs = ["HEAD".to_owned()]
        .into_iter()
        .chain(git(&["symbolic-ref", "-q", "HEAD"]));
    for name in refs {
        if let Some(path) = git(&["rev-parse", "--path-format=absolute", "--git-path", &name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into())
}

fn main() {
    println!("cargo:rustc-env=GIT_COMMIT={}", commit());
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}

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
    "about",
    "reveal_data_folder",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}

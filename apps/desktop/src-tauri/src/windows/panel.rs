use tauri::{
    ActivationPolicy, AppHandle, Emitter, Manager, TitleBarStyle, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

pub const LABEL: &str = "panel";
pub const NAVIGATE: &str = "panel-navigate";

fn create(app: &AppHandle, route: Option<&str>) -> tauri::Result<WebviewWindow> {
    let url = format!("panel.html#{}", route.unwrap_or("/"));
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url.into()))
        .title(&app.package_info().name)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .inner_size(960.0, 640.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .visible(false)
        .build()?;
    let app = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(err) = hide(&app) {
                tracing::error!("hiding the control panel failed: {err}");
            }
        }
    });
    Ok(window)
}

/// Shows the control panel (creating it on first use) with a Dock icon while it is open.
pub fn open(app: &AppHandle, route: Option<&str>) -> tauri::Result<()> {
    let window = match app.get_webview_window(LABEL) {
        Some(window) => {
            if let Some(route) = route {
                window.emit(NAVIGATE, route)?;
            }
            window
        }
        None => create(app, route)?,
    };
    app.set_activation_policy(ActivationPolicy::Regular)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.hide()?;
    }
    app.set_activation_policy(ActivationPolicy::Accessory)
}

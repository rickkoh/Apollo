use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
                .title("Voice Bridge Overlay")
                .inner_size(420.0, 120.0)
                .build()?;
            WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
                .title("Voice Bridge")
                .inner_size(960.0, 640.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Voice Bridge");
}

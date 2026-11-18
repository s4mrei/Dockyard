mod appearance;
#[cfg_attr(windows, path = "windows/applications.rs")]
mod applications;
#[cfg_attr(windows, path = "windows/local_items.rs")]
mod local_items;
#[cfg_attr(windows, path = "windows/private_data.rs")]
mod private_data;
#[cfg(windows)]
#[path = "windows/icon.rs"]
mod windows_icon;
#[cfg(windows)]
#[path = "windows/shell.rs"]
mod windows_shell;
mod workspaces;

#[cfg(target_os = "linux")]
use gtk::prelude::IconThemeExt;
#[cfg(target_os = "linux")]
use tauri::Emitter;
use tauri::{webview::NewWindowResponse, Manager, Url, WebviewWindowBuilder};

fn allowed_navigation(url: &Url) -> bool {
    if !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/" | "/index.html")
        || url.query().is_some()
    {
        return false;
    }
    if cfg!(dev) {
        return url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(1420);
    }
    let (scheme, host) = if cfg!(windows) {
        ("http", "tauri.localhost")
    } else {
        ("tauri", "localhost")
    };
    url.scheme() == scheme && url.host_str() == Some(host) && url.port().is_none()
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            app.manage(workspaces::Store::open(&directory)?);
            #[cfg(target_os = "linux")]
            if let Some(theme) = gtk::IconTheme::default() {
                let handle = app.handle().clone();
                theme.connect_changed(move |_| {
                    let _ = handle.emit("application-icons-changed", ());
                });
            }
            WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .on_navigation(allowed_navigation)
                .on_new_window(|_, _| NewWindowResponse::Deny)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            appearance::load_appearance,
            appearance::save_appearance,
            applications::list_applications,
            applications::launch_application,
            workspaces::list_workspaces,
            workspaces::save_workspace,
            workspaces::delete_workspace,
            workspaces::add_workspace_application,
            workspaces::add_workspace_local_item,
            workspaces::open_workspace_local_item,
            workspaces::remove_workspace_item,
        ])
        .run(tauri::generate_context!())
        .expect("Не удалось открыть Dockyard");
}

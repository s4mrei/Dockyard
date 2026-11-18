use base64::{engine::general_purpose::STANDARD, Engine as _};
use gio::prelude::*;
use gtk::prelude::IconThemeExt;
use serde::Serialize;

#[derive(Serialize)]
pub struct Application {
    id: String,
    name: String,
    icon: Option<String>,
}

fn visible_applications() -> impl Iterator<Item = gio::DesktopAppInfo> {
    gio::AppInfo::all()
        .into_iter()
        .filter_map(|app| app.downcast::<gio::DesktopAppInfo>().ok())
        .filter(|app| app.should_show())
}

fn application_icon(app: &gio::DesktopAppInfo, theme: &gtk::IconTheme) -> Option<String> {
    let icon = app.icon()?;
    let pixbuf = theme
        .lookup_by_gicon(&icon, 48, gtk::IconLookupFlags::FORCE_SIZE)?
        .load_icon()
        .ok()?;
    // Send a small PNG so the webview does not need access to local files.
    let png = pixbuf.save_to_bufferv("png", &[]).ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

#[tauri::command]
pub fn list_applications() -> Vec<Application> {
    let theme = gtk::IconTheme::default();
    visible_applications()
        .filter_map(|app| {
            Some(Application {
                id: app.id()?.to_string(),
                name: app.display_name().to_string(),
                icon: theme
                    .as_ref()
                    .and_then(|theme| application_icon(&app, theme)),
            })
        })
        .collect()
}

fn find_application(id: &str) -> Result<gio::DesktopAppInfo, String> {
    visible_applications()
        .find(|app| app.id().as_deref() == Some(id))
        .ok_or_else(|| "Приложение больше недоступно. Возможно, его удалили или скрыли.".to_owned())
}

pub fn application_name(id: &str) -> Result<String, String> {
    Ok(find_application(id)?.display_name().to_string())
}

#[tauri::command]
pub fn launch_application(id: String) -> Result<(), String> {
    let app = find_application(&id)?;
    let context = gtk::gdk::Display::default().and_then(|display| display.app_launch_context());

    app.launch(
        &[],
        context
            .as_ref()
            .map(|context| context.upcast_ref::<gio::AppLaunchContext>()),
    )
    .map_err(|error| format!("Не удалось запустить «{}». {error}", app.display_name()))
}

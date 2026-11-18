use gio::prelude::*;
use gtk::prelude::{FileChooserExt, NativeDialogExt};
use std::io::ErrorKind;
use tauri::WebviewWindow;

pub fn choose(window: &WebviewWindow, folder: bool) -> Result<Option<(String, String)>, String> {
    let parent = window
        .gtk_window()
        .map_err(|error| format!("Не удалось открыть диалог выбора. {error}"))?;
    let (title, action) = if folder {
        ("Выберите папку", gtk::FileChooserAction::SelectFolder)
    } else {
        ("Выберите файл", gtk::FileChooserAction::Open)
    };
    let dialog = gtk::FileChooserNative::new(
        Some(title),
        Some(&parent),
        action,
        Some("Добавить"),
        Some("Отмена"),
    );
    dialog.set_modal(true);
    // The native dialog runs on GTK's main thread. No database lock is held while it is open.
    if dialog.run() != gtk::ResponseType::Accept {
        return Ok(None);
    }
    let file = dialog.file().ok_or("Не удалось получить выбранный путь.")?;
    let path = file.path().ok_or("Выберите локальный файл или папку.")?;
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    Ok(Some((file.uri().to_string(), name)))
}

pub fn open(uri: &str) -> Result<(), String> {
    let file = gio::File::for_uri(uri);
    let path = file
        .path()
        .ok_or("Ссылка не указывает на локальный файл или папку.")?;
    std::fs::metadata(&path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!(
                "«{}» больше не найден. Возможно, файл или папку переместили.",
                path.display()
            )
        } else {
            format!("Нет доступа к «{}». {error}", path.display())
        }
    })?;

    gio::AppInfo::launch_default_for_uri(uri, gio::AppLaunchContext::NONE)
        .map_err(|error| format!("Не удалось открыть «{}». {error}", path.display()))
}

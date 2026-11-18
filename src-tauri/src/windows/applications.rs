use std::{
    ffi::OsString,
    fs,
    os::windows::{ffi::OsStringExt, fs::MetadataExt},
    path::{Path, PathBuf},
};

use serde::Serialize;
use windows::{
    core::GUID,
    Win32::{
        Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT,
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_CommonPrograms, FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        },
    },
};

use crate::windows_shell::{self, Apartment};

#[derive(Serialize)]
pub struct Application {
    id: String,
    name: String,
    icon: Option<String>,
}

struct Shortcut {
    id: String,
    name: String,
    path: PathBuf,
}

fn known_folder(folder: &GUID) -> Option<PathBuf> {
    let buffer = unsafe { SHGetKnownFolderPath(folder, KF_FLAG_DEFAULT, None).ok()? };
    let path = unsafe { PathBuf::from(OsString::from_wide(buffer.as_wide())) };
    unsafe { CoTaskMemFree(Some(buffer.0.cast())) };
    Some(path)
}

fn shortcuts() -> Vec<Shortcut> {
    let mut entries = Vec::new();
    for (scope, folder) in [
        ("user", FOLDERID_Programs),
        ("common", FOLDERID_CommonPrograms),
    ] {
        let Some(root) = known_folder(&folder) else {
            continue;
        };
        let mut directories = vec![root.clone()];
        while let Some(directory) = directories.pop() {
            let Ok(children) = fs::read_dir(directory) else {
                continue;
            };
            for entry in children.flatten() {
                let path = entry.path();
                let Ok(metadata) = fs::symlink_metadata(&path) else {
                    continue;
                };
                // Do not traverse junctions or accept reparse-point shortcuts outside the catalog.
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
                    continue;
                }
                if metadata.is_dir() {
                    directories.push(path);
                    continue;
                }
                if let Some(shortcut) = shortcut(&root, scope, path, &metadata) {
                    entries.push(shortcut);
                }
            }
        }
    }
    entries
}

fn shortcut(root: &Path, scope: &str, path: PathBuf, metadata: &fs::Metadata) -> Option<Shortcut> {
    if !metadata.is_file() || !path.extension()?.to_str()?.eq_ignore_ascii_case("lnk") {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?.to_str()?.replace('\\', "/");
    let id = format!("windows:{scope}:{}", relative.to_lowercase());
    let name = path.file_stem()?.to_string_lossy().into_owned();
    Some(Shortcut { id, name, path })
}

fn find_application(id: &str) -> Result<Shortcut, String> {
    // IPC IDs are looked up in a newly enumerated catalog. They never become paths.
    shortcuts()
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "Ярлык приложения больше не найден в меню «Пуск».".to_owned())
}

pub fn application_name(id: &str) -> Result<String, String> {
    Ok(find_application(id)?.name)
}

#[tauri::command]
pub fn list_applications() -> Vec<Application> {
    let apartment = Apartment::enter().ok();
    shortcuts()
        .into_iter()
        .map(|entry| Application {
            icon: apartment
                .as_ref()
                .and_then(|_| crate::windows_icon::read(&entry.path)),
            id: entry.id,
            name: entry.name,
        })
        .collect()
}

#[tauri::command]
pub fn launch_application(id: String) -> Result<(), String> {
    windows_shell::open(&find_application(&id)?.path)
}

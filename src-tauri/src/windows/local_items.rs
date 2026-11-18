use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::{Component, Path, PathBuf, Prefix},
};
use tauri::{Url, WebviewWindow};
use windows::{
    core::w,
    Win32::{
        Foundation::ERROR_CANCELLED,
        System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER},
        UI::Shell::{
            FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
            FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
        },
    },
};

use crate::windows_shell::{self, Apartment};

pub fn choose(window: &WebviewWindow, folder: bool) -> Result<Option<(String, String)>, String> {
    let owner = window.hwnd().map_err(|error| error.to_string())?;
    let Some(path) = choose_path(owner, folder)
        .map_err(|error| format!("Не удалось открыть диалог выбора. {error}"))?
    else {
        return Ok(None);
    };
    if !local_drive(&path) {
        return Err("Выберите файл или папку на локальном диске.".into());
    }
    let uri = Url::from_file_path(&path).map_err(|()| "Не удалось сохранить путь.".to_owned())?;
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    Ok(Some((uri.to_string(), name)))
}

fn choose_path(
    owner: windows::Win32::Foundation::HWND,
    folder: bool,
) -> windows::core::Result<Option<PathBuf>> {
    let _apartment = Apartment::enter()?;
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        let selection = if folder {
            FOS_PICKFOLDERS
        } else {
            FOS_FILEMUSTEXIST
        };
        dialog.SetOptions(
            dialog.GetOptions()?
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR
                | selection,
        )?;
        dialog.SetTitle(if folder {
            w!("Выберите папку")
        } else {
            w!("Выберите файл")
        })?;
        match dialog.Show(Some(owner)) {
            Ok(()) => {}
            Err(error) if error.code() == ERROR_CANCELLED.to_hresult() => return Ok(None),
            Err(error) => return Err(error),
        }
        let buffer = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = PathBuf::from(OsString::from_wide(buffer.as_wide()));
        CoTaskMemFree(Some(buffer.0.cast()));
        Ok(Some(path))
    }
}

fn local_drive(path: &Path) -> bool {
    path.is_absolute()
        && matches!(path.components().next(),
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
}

pub fn open(uri: &str) -> Result<(), String> {
    let path = Url::parse(uri)
        .ok()
        .filter(|url| {
            url.scheme() == "file"
                && url.host_str().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
        })
        .and_then(|url| url.to_file_path().ok())
        .filter(|path| local_drive(path))
        .ok_or_else(|| "Ссылка не указывает на локальный файл или папку.".to_owned())?;
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Не удалось открыть «{}». {error}", path.display()))?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err("Ссылка не указывает на обычный файл или папку.".into());
    }
    windows_shell::open(&path)
}

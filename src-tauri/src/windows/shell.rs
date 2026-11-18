use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        },
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
};

pub struct Apartment;

impl Apartment {
    pub fn enter() -> windows::core::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok()? };
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

pub fn open(path: &Path) -> Result<(), String> {
    let _apartment = Apartment::enter().map_err(|error| error.to_string())?;
    let name = wide(path.as_os_str());
    // Pass one pathname to the shell. The shell handles .lnk metadata and file associations.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(name.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err(format!(
            "Не удалось открыть «{}» (код Windows {}).",
            path.display(),
            result.0 as isize
        ))
    } else {
        Ok(())
    }
}

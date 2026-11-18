use std::{
    fs, io,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

pub fn prepare_database(directory: &Path) -> io::Result<PathBuf> {
    // The per-user Tauri data directory inherits the user's Windows ACL.
    fs::create_dir_all(directory)?;
    check(directory, true, false)?;
    let database = directory.join("workspaces.sqlite3");
    for name in [
        "workspaces.sqlite3",
        "workspaces.sqlite3-wal",
        "workspaces.sqlite3-shm",
        "workspaces.sqlite3-journal",
    ] {
        check(&directory.join(name), false, true)?;
    }
    Ok(database)
}

fn check(path: &Path, directory: bool, allow_missing: bool) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0
                && if directory {
                    metadata.is_dir()
                } else {
                    metadata.is_file()
                } =>
        {
            Ok(())
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Небезопасный путь данных Dockyard.",
        )),
        Err(error) if allow_missing && error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

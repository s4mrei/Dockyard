use std::{
    fs::{self, DirBuilder, OpenOptions, Permissions},
    io::{self, ErrorKind},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const DATABASE: &str = "workspaces.sqlite3";
const SIDECARS: &[&str] = &[
    "workspaces.sqlite3-journal",
    "workspaces.sqlite3-wal",
    "workspaces.sqlite3-shm",
];

fn unsafe_path(path: &Path) -> io::Error {
    io::Error::new(
        ErrorKind::PermissionDenied,
        format!("Небезопасный путь данных: {}", path.display()),
    )
}

pub fn prepare_database(directory: &Path) -> io::Result<PathBuf> {
    let parent = directory.parent().ok_or_else(|| unsafe_path(directory))?;
    fs::create_dir_all(parent)?;
    // The parent must not let another local account replace the application directory.
    let owner = unsafe { libc::geteuid() };
    let parent = parent.canonicalize()?;
    for ancestor in parent.ancestors() {
        let metadata = fs::metadata(ancestor)?;
        let trusted_owner = metadata.uid() == owner || metadata.uid() == 0;
        let writable_by_others = metadata.mode() & 0o022 != 0;
        let sticky = metadata.mode() & libc::S_ISVTX != 0;
        if !trusted_owner || (writable_by_others && !sticky) {
            return Err(unsafe_path(ancestor));
        }
    }
    let directory = parent.join(
        directory
            .file_name()
            .ok_or_else(|| unsafe_path(directory))?,
    );
    match DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let handle = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(&directory)?;
    if handle.metadata()?.uid() != owner {
        return Err(unsafe_path(&directory));
    }
    handle.set_permissions(Permissions::from_mode(0o700))?;

    let db = directory.join(DATABASE);
    secure_file(&db, true, owner)?;
    for name in SIDECARS {
        secure_file(&directory.join(name), false, owner)?;
    }
    Ok(db)
}

fn secure_file(path: &Path, create: bool, owner: u32) -> io::Result<()> {
    let file = match OpenOptions::new()
        .read(true)
        .write(create)
        .create(create)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if !create && error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.uid() != owner || metadata.nlink() != 1 {
        return Err(unsafe_path(path));
    }
    file.set_permissions(Permissions::from_mode(0o600))
}

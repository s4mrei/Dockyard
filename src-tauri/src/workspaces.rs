use std::{path::Path, sync::Mutex, time::Duration};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use tauri::{State, WebviewWindow};

use crate::{applications, local_items, private_data};

pub struct Store(Mutex<Connection>);

#[derive(Serialize)]
pub struct Workspace {
    id: i64,
    name: String,
    items: Vec<Item>,
}

#[derive(Serialize)]
struct Item {
    id: i64,
    kind: String,
    target: String,
    name: String,
}

impl Store {
    pub fn open(directory: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = private_data::prepare_database(directory)?;
        let db = Connection::open_with_flags(
            path,
            OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        db.busy_timeout(Duration::from_secs(3))?;
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS appearance (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                settings TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                kind TEXT NOT NULL CHECK (kind IN ('application', 'file', 'folder')),
                target TEXT NOT NULL,
                name TEXT NOT NULL,
                UNIQUE (workspace_id, kind, target)
             );",
        )?;
        Ok(Self(Mutex::new(db)))
    }

    pub(crate) fn query<T>(
        &self,
        query: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, String> {
        let db = self
            .0
            .lock()
            .map_err(|_| "Хранилище вкладок недоступно.".to_owned())?;
        query(&db).map_err(|error| format!("Ошибка хранилища вкладок. {error}"))
    }

    fn add(&self, workspace_id: i64, kind: &str, target: &str, name: &str) -> Result<(), String> {
        self.query(|db| {
            db.execute(
                "INSERT INTO items (workspace_id, kind, target, name) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (workspace_id, kind, target) DO NOTHING",
                params![workspace_id, kind, target, name],
            )
        })?;
        Ok(())
    }
}

#[tauri::command]
pub fn list_workspaces(store: State<'_, Store>) -> Result<Vec<Workspace>, String> {
    store.query(|db| {
        let tx = db.unchecked_transaction()?;
        let workspaces = {
            let mut tabs = tx.prepare("SELECT id, name FROM workspaces ORDER BY id")?;
            let mut items = tx.prepare(
                "SELECT id, kind, target, name FROM items WHERE workspace_id = ?1 ORDER BY id",
            )?;
            let rows = tabs.query_map([], |row| {
                let id = row.get::<_, i64>(0)?;
                let items = items
                    .query_map([id], |row| {
                        Ok(Item {
                            id: row.get(0)?,
                            kind: row.get(1)?,
                            target: row.get(2)?,
                            name: row.get(3)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(Workspace {
                    id,
                    name: row.get(1)?,
                    items,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        tx.commit()?;
        Ok(workspaces)
    })
}

#[tauri::command]
pub fn save_workspace(
    store: State<'_, Store>,
    id: Option<i64>,
    name: String,
) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Введите название вкладки длиной от 1 до 80 символов.".to_owned());
    }
    match id {
        Some(id) => {
            let changed = store.query(|db| {
                db.execute(
                    "UPDATE workspaces SET name = ?1 WHERE id = ?2",
                    params![name, id],
                )
            })?;
            if changed == 0 {
                return Err("Вкладка больше не существует.".to_owned());
            }
            Ok(id)
        }
        None => store.query(|db| {
            db.execute("INSERT INTO workspaces (name) VALUES (?1)", [name])?;
            Ok(db.last_insert_rowid())
        }),
    }
}

#[tauri::command]
pub fn delete_workspace(store: State<'_, Store>, id: i64) -> Result<(), String> {
    store.query(|db| db.execute("DELETE FROM workspaces WHERE id = ?1", [id]))?;
    Ok(())
}

#[tauri::command]
pub fn add_workspace_application(
    store: State<'_, Store>,
    workspace_id: i64,
    desktop_id: String,
) -> Result<(), String> {
    let name = applications::application_name(&desktop_id)?;
    store.add(workspace_id, "application", &desktop_id, &name)
}

#[tauri::command]
pub fn add_workspace_local_item(
    store: State<'_, Store>,
    window: WebviewWindow,
    workspace_id: i64,
    folder: bool,
) -> Result<(), String> {
    if let Some((uri, name)) = local_items::choose(&window, folder)? {
        store.add(
            workspace_id,
            if folder { "folder" } else { "file" },
            &uri,
            &name,
        )?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_workspace_local_item(store: State<'_, Store>, id: i64) -> Result<(), String> {
    let uri: String = store
        .query(|db| {
            db.query_row(
                "SELECT target FROM items WHERE id = ?1 AND kind IN ('file', 'folder')",
                [id],
                |row| row.get(0),
            )
            .optional()
        })?
        .ok_or_else(|| "Ссылка больше не существует.".to_owned())?;
    local_items::open(&uri)
}

#[tauri::command]
pub fn remove_workspace_item(store: State<'_, Store>, id: i64) -> Result<(), String> {
    store.query(|db| db.execute("DELETE FROM items WHERE id = ?1", [id]))?;
    Ok(())
}

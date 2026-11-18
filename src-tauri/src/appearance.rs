use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::workspaces::Store;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum AccentMode {
    #[default]
    Icons,
    Single,
}

#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    mode: AccentMode,
    accent: String,
    glow: bool,
    glow_color: String,
    overrides: BTreeMap<String, String>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: AccentMode::Icons,
            accent: "#497FBE".into(),
            glow: true,
            glow_color: "#497FBE".into(),
            overrides: BTreeMap::new(),
        }
    }
}

fn is_hex(color: &str) -> bool {
    color
        .strip_prefix('#')
        .is_some_and(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[tauri::command]
pub fn load_appearance(store: State<'_, Store>) -> Result<Appearance, String> {
    let json: Option<String> = store.query(|db| {
        db.query_row("SELECT settings FROM appearance WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()
    })?;
    json.map_or_else(
        || Ok(Appearance::default()),
        |json| {
            serde_json::from_str(&json).map_err(|e| format!("Не удалось прочитать оформление. {e}"))
        },
    )
}

#[tauri::command]
pub fn save_appearance(store: State<'_, Store>, settings: Appearance) -> Result<(), String> {
    if !is_hex(&settings.accent)
        || !is_hex(&settings.glow_color)
        || !settings.overrides.values().all(|color| is_hex(color))
    {
        return Err("Введите цвет в формате #RRGGBB.".into());
    }
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    store.query(|db| {
        db.execute(
            "INSERT INTO appearance (id, settings) VALUES (1, ?1)
         ON CONFLICT (id) DO UPDATE SET settings = excluded.settings",
            [json],
        )
    })?;
    Ok(())
}

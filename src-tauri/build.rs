fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "load_appearance",
            "save_appearance",
            "list_applications",
            "launch_application",
            "list_workspaces",
            "save_workspace",
            "delete_workspace",
            "add_workspace_application",
            "add_workspace_local_item",
            "open_workspace_local_item",
            "remove_workspace_item",
        ]),
    ))
    .expect("Не удалось собрать разрешения Dockyard");
}

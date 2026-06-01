mod bridge;

use bridge::{ApiBridge, BridgeResponse};

#[tauri::command]
fn server_status() -> BridgeResponse {
    ApiBridge::default().server_status()
}

#[tauri::command]
fn workspace_list() -> BridgeResponse {
    ApiBridge::default().workspace_list()
}

#[tauri::command]
fn workspace_focus(workspace_id: String) -> BridgeResponse {
    ApiBridge::default().workspace_focus(workspace_id)
}

#[tauri::command]
fn workspace_create(cwd: Option<String>, label: Option<String>, focus: bool) -> BridgeResponse {
    ApiBridge::default().workspace_create(cwd, label, focus)
}

#[tauri::command]
fn workspace_rename(workspace_id: String, label: String) -> BridgeResponse {
    ApiBridge::default().workspace_rename(workspace_id, label)
}

#[tauri::command]
fn tab_list(workspace_id: Option<String>) -> BridgeResponse {
    ApiBridge::default().tab_list(workspace_id)
}

#[tauri::command]
fn tab_focus(tab_id: String) -> BridgeResponse {
    ApiBridge::default().tab_focus(tab_id)
}

#[tauri::command]
fn pane_list(workspace_id: Option<String>) -> BridgeResponse {
    ApiBridge::default().pane_list(workspace_id)
}

#[tauri::command]
fn agent_list() -> BridgeResponse {
    ApiBridge::default().agent_list()
}

#[tauri::command]
fn agent_focus(target: String) -> BridgeResponse {
    ApiBridge::default().agent_focus(target)
}

#[tauri::command]
fn agent_send(target: String, text: String) -> BridgeResponse {
    ApiBridge::default().agent_send(target, text)
}

#[tauri::command]
fn kanban_list(status: Option<String>) -> BridgeResponse {
    ApiBridge::default().kanban_list(status)
}

#[tauri::command]
fn kanban_add(
    title: String,
    description: Option<String>,
    status: Option<String>,
) -> BridgeResponse {
    ApiBridge::default().kanban_add(title, description, status)
}

#[tauri::command]
fn kanban_update(
    uuid: String,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
) -> BridgeResponse {
    ApiBridge::default().kanban_update(uuid, title, description, status)
}

#[tauri::command]
fn kanban_delete(uuid: String) -> BridgeResponse {
    ApiBridge::default().kanban_delete(uuid)
}

#[tauri::command]
fn pane_read(pane_id: String) -> BridgeResponse {
    ApiBridge::default().pane_read(pane_id)
}

pub fn run() {
    let builder = tauri::Builder::default().invoke_handler(tauri::generate_handler![
        server_status,
        workspace_list,
        workspace_focus,
        workspace_create,
        workspace_rename,
        tab_list,
        tab_focus,
        pane_list,
        agent_list,
        agent_focus,
        agent_send,
        kanban_list,
        kanban_add,
        kanban_update,
        kanban_delete,
        pane_read,
    ]);

    if let Err(error) = builder.run(tauri::generate_context!()) {
        eprintln!("failed to run Orbit: {error}");
    }
}

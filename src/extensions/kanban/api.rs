use crate::api::schema::{
    EventData, EventEnvelope, EventKind, KanbanAddParams, KanbanDeleteParams, KanbanItem,
    KanbanListParams, KanbanUpdateParams, Method, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

const KANBAN_CARD_RESOURCE_KIND: &str = "application/vnd.herdr.kanban-card+json";

pub(crate) fn handle_api_request(
    app: &mut App,
    request_id: String,
    method: &Method,
) -> Option<String> {
    match method {
        Method::KanbanAdd(params) => Some(handle_kanban_add(app, request_id, params.clone())),
        Method::KanbanList(params) => Some(handle_kanban_list(app, request_id, params.clone())),
        Method::KanbanUpdate(params) => Some(handle_kanban_update(app, request_id, params.clone())),
        Method::KanbanDelete(params) => Some(handle_kanban_delete(app, request_id, params.clone())),
        _ => None,
    }
}

fn emit_kanban_event(app: &mut App, event: EventKind, data: EventData) {
    app.emit_event(EventEnvelope { event, data });
}

fn emit_kanban_added(app: &mut App, item: KanbanItem) {
    emit_kanban_event(app, EventKind::KanbanAdded, EventData::KanbanAdded { item });
}

fn emit_kanban_updated(app: &mut App, item: KanbanItem) {
    emit_kanban_event(
        app,
        EventKind::KanbanUpdated,
        EventData::KanbanUpdated { item },
    );
}

fn emit_kanban_deleted(app: &mut App, item: KanbanItem) {
    emit_kanban_event(
        app,
        EventKind::KanbanDeleted,
        EventData::KanbanDeleted { item },
    );
}

fn mirror_kanban_card_resource(app: &mut App, item: &KanbanItem) {
    let Ok(value) = serde_json::to_value(item) else {
        tracing::warn!(uuid = %item.uuid, "failed to serialize kanban card for plugin resource mirror");
        return;
    };
    app.mirror_plugin_resource_kind_put(KANBAN_CARD_RESOURCE_KIND, &item.uuid, value);
}

fn delete_kanban_card_resource_mirror(app: &mut App, item: &KanbanItem) {
    app.mirror_plugin_resource_kind_delete(KANBAN_CARD_RESOURCE_KIND, &item.uuid);
}

fn active_pane_terminal_ids(app: &App) -> std::collections::HashSet<String> {
    let mut active = std::collections::HashSet::new();
    for ws in &app.state.workspaces {
        for tab in &ws.tabs {
            for pane in tab.panes.values() {
                active.insert(pane.attached_terminal_id.to_string());
            }
        }
    }
    active
}

fn handle_kanban_add(app: &mut App, id: String, params: KanbanAddParams) -> String {
    if let Some(ref path_str) = params.description {
        if !path_str.is_empty() {
            let path = std::path::Path::new(path_str);
            if std::fs::File::open(path).is_err() {
                return encode_error(
                    id,
                    "kanban_description_not_accessible",
                    format!("Description path {} is not accessible", path_str),
                );
            }
        }
    }
    let terminal_id = params.terminal_id.map(|tid| {
        if let Ok(resolved) = app.resolve_terminal_target(&tid) {
            resolved.terminal_id
        } else {
            tid
        }
    });
    let active_tids = active_pane_terminal_ids(app);
    if app
        .state
        .extensions
        .kanban
        .clear_dead_terminals(|tid| active_tids.contains(tid))
    {
        app.state.mark_session_dirty();
        app.schedule_session_save();
    }

    let item = app.state.extensions.kanban.add_item(
        params.title,
        params.description,
        params.status,
        terminal_id,
    );
    app.state.mark_session_dirty();
    app.schedule_session_save();
    mirror_kanban_card_resource(app, &item);
    emit_kanban_added(app, item.clone());
    encode_success(id, ResponseResult::KanbanItem { item })
}

fn handle_kanban_list(app: &mut App, id: String, params: KanbanListParams) -> String {
    let target_terminal_id = params.terminal_id.map(|tid| {
        if let Ok(resolved) = app.resolve_terminal_target(&tid) {
            resolved.terminal_id
        } else {
            tid
        }
    });

    let active_tids = active_pane_terminal_ids(app);
    if app
        .state
        .extensions
        .kanban
        .clear_dead_terminals(|tid| active_tids.contains(tid))
    {
        app.state.mark_session_dirty();
        app.schedule_session_save();
    }

    let items = app
        .state
        .extensions
        .kanban
        .items
        .iter()
        .filter(|item| {
            if let Some(ref status) = params.status {
                if item.status != *status {
                    return false;
                }
            }
            if let Some(ref target_tid) = target_terminal_id {
                if item.terminal_id.as_ref() != Some(target_tid) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    encode_success(id, ResponseResult::KanbanList { items })
}

fn handle_kanban_update(app: &mut App, id: String, params: KanbanUpdateParams) -> String {
    if let Some(ref path_str) = params.description {
        if !path_str.is_empty() {
            let path = std::path::Path::new(path_str);
            if std::fs::File::open(path).is_err() {
                return encode_error(
                    id,
                    "kanban_description_not_accessible",
                    format!("Description path {} is not accessible", path_str),
                );
            }
        }
    }
    let terminal_id = params.terminal_id.map(|tid| {
        if let Ok(resolved) = app.resolve_terminal_target(&tid) {
            resolved.terminal_id
        } else {
            tid
        }
    });
    let active_tids = active_pane_terminal_ids(app);
    if app
        .state
        .extensions
        .kanban
        .clear_dead_terminals(|tid| active_tids.contains(tid))
    {
        app.state.mark_session_dirty();
        app.schedule_session_save();
    }

    match app.state.extensions.kanban.update_item(
        &params.uuid,
        params.title,
        params.description,
        params.status,
        terminal_id,
        params.clear_terminal_id,
    ) {
        Some(item) => {
            app.state.mark_session_dirty();
            app.schedule_session_save();
            mirror_kanban_card_resource(app, &item);
            emit_kanban_updated(app, item.clone());
            encode_success(id, ResponseResult::KanbanItem { item })
        }
        None => encode_error(
            id,
            "kanban_item_not_found",
            format!("Kanban item with uuid {} not found", params.uuid),
        ),
    }
}

fn handle_kanban_delete(app: &mut App, id: String, params: KanbanDeleteParams) -> String {
    let active_tids = active_pane_terminal_ids(app);
    if app
        .state
        .extensions
        .kanban
        .clear_dead_terminals(|tid| active_tids.contains(tid))
    {
        app.state.mark_session_dirty();
        app.schedule_session_save();
    }

    match app.state.extensions.kanban.delete_item(&params.uuid) {
        Some(item) => {
            app.state.mark_session_dirty();
            app.schedule_session_save();
            delete_kanban_card_resource_mirror(app, &item);
            emit_kanban_deleted(app, item.clone());
            encode_success(id, ResponseResult::KanbanItem { item })
        }
        None => encode_error(
            id,
            "kanban_item_not_found",
            format!("Kanban item with uuid {} not found", params.uuid),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{
        Method, PluginLinkParams, PluginResourceGetParams, Request, SuccessResponse,
    };
    use crate::config::Config;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn kanban_api_request_dispatch_handles_kanban_methods() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );

        let response = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Dispatched card".into(),
                description: None,
                status: None,
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled by kanban extension API");

        let resp: SuccessResponse = serde_json::from_str(&response).unwrap();
        let item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };
        assert_eq!(item.title, "Dispatched card");
    }

    #[test]
    fn kanban_mutations_mirror_to_plugin_card_resource() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("kanban-resource-mirror");
        let xdg_home = unique_temp_path("kanban-resource-mirror-xdg");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_manifest_content(
            &root,
            r#"
id = "example.board"
name = "Board"
version = "0.1.0"
api_version = 2
capabilities = ["resources", "storage"]
min_herdr_version = "0.6.10"
platforms = ["linux", "macos", "windows"]

[[resources]]
id = "cards"
title = "Cards"
kind = "application/vnd.herdr.kanban-card+json"
storage_prefix = "resources/cards/"
"#,
        );

        let link = app.handle_api_request(Request {
            id: "link".into(),
            method: Method::PluginLink(PluginLinkParams {
                path: root.display().to_string(),
                enabled: true,
                source: None,
            }),
        });
        assert!(link.contains("plugin_linked"), "link failed: {link}");

        let add = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Plugin card".into(),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Todo),
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled");
        let item = kanban_item_from_response(&add);
        let mirrored = plugin_resource_value(&mut app, &item.uuid).expect("card should mirror");
        assert_eq!(mirrored["uuid"], item.uuid);
        assert_eq!(mirrored["title"], "Plugin card");
        assert_eq!(mirrored["status"], "todo");

        let update = handle_api_request(
            &mut app,
            "update".into(),
            &Method::KanbanUpdate(KanbanUpdateParams {
                uuid: item.uuid.clone(),
                title: Some("Plugin card updated".into()),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Reviewing),
                terminal_id: None,
                clear_terminal_id: None,
            }),
        )
        .expect("kanban.update should be handled");
        let updated = kanban_item_from_response(&update);
        let mirrored = plugin_resource_value(&mut app, &updated.uuid).expect("card should update");
        assert_eq!(mirrored["title"], "Plugin card updated");
        assert_eq!(mirrored["status"], "reviewing");

        let delete = handle_api_request(
            &mut app,
            "delete".into(),
            &Method::KanbanDelete(KanbanDeleteParams {
                uuid: updated.uuid.clone(),
            }),
        )
        .expect("kanban.delete should be handled");
        let deleted = kanban_item_from_response(&delete);
        assert_eq!(deleted.uuid, updated.uuid);
        assert!(plugin_resource_value(&mut app, &updated.uuid).is_none());

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        match old_config_home {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_state_home {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
    }

    #[test]
    fn test_kanban_api_handlers() {
        let temp_dir = std::env::temp_dir();
        let plan_file = temp_dir.join(format!("herdr-test-plan-{}.md", uuid::Uuid::new_v4()));
        std::fs::write(&plan_file, "API test plan content").unwrap();
        let plan_path = plan_file.to_string_lossy().to_string();

        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );

        // Add
        let add_res = handle_kanban_add(
            &mut app,
            "1".into(),
            KanbanAddParams {
                title: "Test Kanban".into(),
                description: Some(plan_path.clone()),
                status: None,
                terminal_id: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&add_res).unwrap();
        let item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };
        assert_eq!(item.title, "Test Kanban");

        // List
        let list_res = handle_kanban_list(
            &mut app,
            "2".into(),
            KanbanListParams {
                status: None,
                terminal_id: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 1);

        // List with terminal_id: Some("other-terminal") (should return 0 items)
        let list_res = handle_kanban_list(
            &mut app,
            "2a".into(),
            KanbanListParams {
                status: None,
                terminal_id: Some("other-terminal".into()),
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 0);

        let terminal_id = crate::terminal::TerminalId::alloc();
        let term_id_str = terminal_id.to_string();

        // Add a fake workspace/pane so "my-terminal" is considered alive
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane = crate::pane::PaneState::new(terminal_id);
        ws.tabs[0]
            .panes
            .insert(crate::layout::PaneId::from_raw(1), pane);
        app.state.workspaces.push(ws);

        // Update item with terminal_id: Some("my-terminal")
        let update_res = handle_kanban_update(
            &mut app,
            "2b".into(),
            KanbanUpdateParams {
                uuid: item.uuid.clone(),
                title: None,
                description: None,
                status: None,
                terminal_id: Some(term_id_str.clone()),
                clear_terminal_id: None,
            },
        );
        assert!(update_res.contains("\"type\":\"kanban_item\""));

        // List with terminal_id: Some("my-terminal") (should return 1 item)
        let list_res = handle_kanban_list(
            &mut app,
            "2c".into(),
            KanbanListParams {
                status: None,
                terminal_id: Some(term_id_str.clone()),
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].terminal_id, Some(term_id_str));

        // Update
        let update_res = handle_kanban_update(
            &mut app,
            "3".into(),
            KanbanUpdateParams {
                uuid: item.uuid.clone(),
                title: Some("Updated Title".into()),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Ongoing),
                terminal_id: None,
                clear_terminal_id: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&update_res).unwrap();
        let updated_item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };
        assert_eq!(updated_item.title, "Updated Title");
        assert_eq!(
            updated_item.status,
            crate::api::schema::KanbanStatus::Ongoing
        );

        // Delete
        let delete_res = handle_kanban_delete(
            &mut app,
            "4".into(),
            KanbanDeleteParams {
                uuid: item.uuid.clone(),
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&delete_res).unwrap();
        assert!(matches!(resp.result, ResponseResult::KanbanItem { .. }));

        // List after delete should be empty
        let list_res = handle_kanban_list(
            &mut app,
            "5".into(),
            KanbanListParams {
                status: None,
                terminal_id: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 0);

        let _ = std::fs::remove_file(plan_file);
    }

    #[test]
    fn kanban_api_handlers_emit_mutation_events() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let event_hub = crate::api::EventHub::default();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());

        let add_res = handle_kanban_add(
            &mut app,
            "add".into(),
            KanbanAddParams {
                title: "Streamed card".into(),
                description: None,
                status: None,
                terminal_id: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&add_res).unwrap();
        let item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };

        handle_kanban_update(
            &mut app,
            "update".into(),
            KanbanUpdateParams {
                uuid: item.uuid.clone(),
                title: Some("Updated streamed card".into()),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Reviewing),
                terminal_id: None,
                clear_terminal_id: None,
            },
        );
        handle_kanban_delete(
            &mut app,
            "delete".into(),
            KanbanDeleteParams {
                uuid: item.uuid.clone(),
            },
        );

        let events = event_hub.events_after(0);
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[0].1.data,
            crate::api::schema::EventData::KanbanAdded { item: added }
                if events[0].1.event == crate::api::schema::EventKind::KanbanAdded
                    && added.uuid == item.uuid
                    && added.title == "Streamed card"
        ));
        assert!(matches!(
            &events[1].1.data,
            crate::api::schema::EventData::KanbanUpdated { item: updated }
                if events[1].1.event == crate::api::schema::EventKind::KanbanUpdated
                    && updated.uuid == item.uuid
                    && updated.title == "Updated streamed card"
                    && updated.status == crate::api::schema::KanbanStatus::Reviewing
        ));
        assert!(matches!(
            &events[2].1.data,
            crate::api::schema::EventData::KanbanDeleted { item: deleted }
                if events[2].1.event == crate::api::schema::EventKind::KanbanDeleted
                    && deleted.uuid == item.uuid
        ));
    }

    #[test]
    fn test_kanban_api_validation_and_rendering() {
        let temp_dir = std::env::temp_dir();
        let plan_file = temp_dir.join(format!("herdr-test-validation-{}.md", uuid::Uuid::new_v4()));
        let plan_path = plan_file.to_string_lossy().to_string();

        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );

        // 1. Validation fails when file does not exist
        let add_res = handle_kanban_add(
            &mut app,
            "1".into(),
            KanbanAddParams {
                title: "Test Plan Validation".into(),
                description: Some(plan_path.clone()),
                status: None,
                terminal_id: None,
            },
        );
        assert!(add_res.contains("kanban_description_not_accessible"));

        // 2. Validation succeeds once file is written
        std::fs::write(&plan_file, "API validation check").unwrap();
        let add_res2 = handle_kanban_add(
            &mut app,
            "2".into(),
            KanbanAddParams {
                title: "Test Plan Validation".into(),
                description: Some(plan_path.clone()),
                status: None,
                terminal_id: None,
            },
        );
        assert!(add_res2.contains("\"type\":\"kanban_item\""));

        // 3. UI Helper behaves correctly
        let (text, is_err) = crate::extensions::kanban::ui::get_description_text(&plan_path);
        assert_eq!(text, "API validation check");
        assert!(!is_err);

        // 4. UI Helper handles deleted file correctly
        std::fs::remove_file(&plan_file).unwrap();
        let (text2, is_err2) = crate::extensions::kanban::ui::get_description_text(&plan_path);
        assert_eq!(text2, "NO DESCRIPTION FOUND");
        assert!(is_err2);
    }

    fn kanban_item_from_response(response: &str) -> KanbanItem {
        let resp: SuccessResponse = serde_json::from_str(response).unwrap();
        match resp.result {
            ResponseResult::KanbanItem { item } => item,
            other => panic!("expected kanban item response, got {other:?}"),
        }
    }

    fn plugin_resource_value(app: &mut App, item_id: &str) -> Option<serde_json::Value> {
        let response = app.handle_api_request(Request {
            id: "resource-get".into(),
            method: Method::PluginResourceGet(PluginResourceGetParams {
                plugin_id: "example.board".into(),
                resource_id: "cards".into(),
                item_id: item_id.into(),
            }),
        });
        let resp: SuccessResponse = serde_json::from_str(&response).unwrap();
        match resp.result {
            ResponseResult::PluginResourceValue { value, .. } => value,
            other => panic!("expected plugin resource value response, got {other:?}"),
        }
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("herdr-{name}-{}-{nanos}", std::process::id()))
    }

    fn write_manifest_content(root: &std::path::Path, content: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("herdr-plugin.toml"), content).unwrap();
    }
}

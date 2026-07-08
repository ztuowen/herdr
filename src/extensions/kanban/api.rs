use crate::api::schema::{
    KanbanAddParams, KanbanDeleteParams, KanbanListParams, KanbanUpdateParams, Method,
    ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

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

    let item = crate::extensions::kanban::resources::create_card(
        app,
        params.title,
        params.description,
        params.status,
        terminal_id,
    );
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

    match crate::extensions::kanban::resources::update_card(
        app,
        &params.uuid,
        params.title,
        params.description,
        params.status,
        terminal_id,
        params.clear_terminal_id,
    ) {
        Some(item) => encode_success(id, ResponseResult::KanbanItem { item }),
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

    match crate::extensions::kanban::resources::delete_card(app, &params.uuid) {
        Some(item) => encode_success(id, ResponseResult::KanbanItem { item }),
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
        KanbanItem, Method, PluginLinkParams, PluginResourceDeleteParams, PluginResourceGetParams,
        PluginSetEnabledParams, Request, SuccessResponse,
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
        write_board_manifest(&root);

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
        let events = app.event_hub.events_after(0);
        assert_plugin_resource_events_include(
            &events,
            crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID,
            "resource.put",
            crate::extensions::kanban::resources::CARD_RESOURCE_ID,
            &item.uuid,
            Some("Plugin card"),
        );
        assert_plugin_resource_events_include(
            &events,
            "example.board",
            "resource.put",
            "cards",
            &item.uuid,
            Some("Plugin card"),
        );
        assert_kanban_event_includes_added(&events, &item.uuid);

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
        let events = app.event_hub.events_after(0);
        assert_plugin_resource_events_include(
            &events,
            crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID,
            "resource.put",
            crate::extensions::kanban::resources::CARD_RESOURCE_ID,
            &updated.uuid,
            Some("Plugin card updated"),
        );
        assert_plugin_resource_events_include(
            &events,
            "example.board",
            "resource.put",
            "cards",
            &updated.uuid,
            Some("Plugin card updated"),
        );
        assert_kanban_event_includes_updated(&events, &updated.uuid);

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
        let events = app.event_hub.events_after(0);
        assert_plugin_resource_events_include(
            &events,
            crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID,
            "resource.delete",
            crate::extensions::kanban::resources::CARD_RESOURCE_ID,
            &updated.uuid,
            None,
        );
        assert_plugin_resource_events_include(
            &events,
            "example.board",
            "resource.delete",
            "cards",
            &updated.uuid,
            None,
        );
        assert_kanban_event_includes_deleted(&events, &updated.uuid);

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
    fn linking_board_plugin_backfills_existing_kanban_cards() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("kanban-resource-link-backfill");
        let xdg_home = unique_temp_path("kanban-resource-link-backfill-xdg");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_board_manifest(&root);

        let add = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Existing card".into(),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Blocked),
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled");
        let item = kanban_item_from_response(&add);

        link_board_plugin(&mut app, &root, true);

        let mirrored = plugin_resource_value(&mut app, &item.uuid).expect("card should backfill");
        assert_eq!(mirrored["uuid"], item.uuid);
        assert_eq!(mirrored["title"], "Existing card");
        assert_eq!(mirrored["status"], "blocked");

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        restore_xdg_home(old_config_home, old_state_home);
    }

    #[test]
    fn enabling_board_plugin_backfills_existing_kanban_cards() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("kanban-resource-enable-backfill");
        let xdg_home = unique_temp_path("kanban-resource-enable-backfill-xdg");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_board_manifest(&root);

        let add = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Disabled board card".into(),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Ongoing),
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled");
        let item = kanban_item_from_response(&add);

        link_board_plugin(&mut app, &root, false);
        assert!(
            plugin_resource_value(&mut app, &item.uuid).is_none(),
            "disabled plugin should not receive backfill"
        );

        let enable = app.handle_api_request(Request {
            id: "enable".into(),
            method: Method::PluginEnable(PluginSetEnabledParams {
                plugin_id: "example.board".into(),
            }),
        });
        assert!(enable.contains("plugin_enabled"), "enable failed: {enable}");

        let mirrored = plugin_resource_value(&mut app, &item.uuid).expect("card should backfill");
        assert_eq!(mirrored["uuid"], item.uuid);
        assert_eq!(mirrored["title"], "Disabled board card");
        assert_eq!(mirrored["status"], "ongoing");

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        restore_xdg_home(old_config_home, old_state_home);
    }

    #[test]
    fn kanban_cards_are_available_through_builtin_plugin_resources() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );

        let add = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Builtin resource card".into(),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Ongoing),
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled");
        let item = kanban_item_from_response(&add);

        let list = app.handle_api_request(Request {
            id: "list".into(),
            method: Method::PluginResourceList(crate::api::schema::PluginResourceListParams {
                plugin_id: crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID.into(),
                resource_id: crate::extensions::kanban::resources::CARD_RESOURCE_ID.into(),
            }),
        });
        let ResponseResult::PluginResourceList { items, .. } = response_result(&list) else {
            panic!("expected plugin resource list: {list}");
        };
        assert_eq!(items[&item.uuid]["title"], "Builtin resource card");
        assert_eq!(items[&item.uuid]["status"], "ongoing");

        let get = app.handle_api_request(Request {
            id: "get".into(),
            method: Method::PluginResourceGet(PluginResourceGetParams {
                plugin_id: crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID.into(),
                resource_id: crate::extensions::kanban::resources::CARD_RESOURCE_ID.into(),
                item_id: item.uuid.clone(),
            }),
        });
        let ResponseResult::PluginResourceValue { value, .. } = response_result(&get) else {
            panic!("expected plugin resource value: {get}");
        };
        let value = value.expect("card should exist");
        assert_eq!(value["uuid"], item.uuid);
        assert_eq!(value["title"], "Builtin resource card");

        let events = app.event_hub.events_after(0);
        assert!(events.iter().any(|(_, envelope)| {
            matches!(
                &envelope.data,
                crate::api::schema::EventData::PluginEvent {
                    plugin_id,
                    event,
                    payload,
                } if plugin_id == crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID
                    && event == "resource.put"
                    && payload["resource_id"] == crate::extensions::kanban::resources::CARD_RESOURCE_ID
                    && payload["item_id"] == item.uuid
            )
        }));
    }

    #[test]
    fn builtin_plugin_resources_update_kanban_compatibility_projection() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );

        let put = app.handle_api_request(Request {
            id: "put".into(),
            method: Method::PluginResourcePut(crate::api::schema::PluginResourcePutParams {
                plugin_id: crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID.into(),
                resource_id: crate::extensions::kanban::resources::CARD_RESOURCE_ID.into(),
                item_id: "card-1".into(),
                value: serde_json::json!({
                    "uuid": "card-1",
                    "title": "V2 card",
                    "description": "Created through plugin.resource.put",
                    "status": "todo",
                    "terminal_id": null,
                }),
            }),
        });
        let ResponseResult::PluginResourcePut { value, .. } = response_result(&put) else {
            panic!("expected plugin resource put: {put}");
        };
        assert_eq!(value["title"], "V2 card");

        let list = handle_api_request(
            &mut app,
            "kanban-list".into(),
            &Method::KanbanList(KanbanListParams {
                status: None,
                terminal_id: None,
            }),
        )
        .expect("kanban.list should be handled");
        let ResponseResult::KanbanList { items } = response_result(&list) else {
            panic!("expected kanban list: {list}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].uuid, "card-1");
        assert_eq!(items[0].title, "V2 card");

        let delete = app.handle_api_request(Request {
            id: "delete".into(),
            method: Method::PluginResourceDelete(PluginResourceDeleteParams {
                plugin_id: crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID.into(),
                resource_id: crate::extensions::kanban::resources::CARD_RESOURCE_ID.into(),
                item_id: "card-1".into(),
            }),
        });
        let ResponseResult::PluginResourceDeleted { existed, .. } = response_result(&delete) else {
            panic!("expected plugin resource delete: {delete}");
        };
        assert!(existed);
        assert!(app.state.extensions.kanban.items.is_empty());
    }

    #[test]
    fn kanban_delete_mirror_emits_resource_event_only_when_resource_existed() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("kanban-resource-delete-missing");
        let xdg_home = unique_temp_path("kanban-resource-delete-missing-xdg");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_board_manifest(&root);

        link_board_plugin(&mut app, &root, true);

        let add = handle_api_request(
            &mut app,
            "add".into(),
            &Method::KanbanAdd(KanbanAddParams {
                title: "Missing mirror card".into(),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::Todo),
                terminal_id: None,
            }),
        )
        .expect("kanban.add should be handled");
        let item = kanban_item_from_response(&add);

        let direct_delete = app.handle_api_request(Request {
            id: "resource-delete".into(),
            method: Method::PluginResourceDelete(PluginResourceDeleteParams {
                plugin_id: "example.board".into(),
                resource_id: "cards".into(),
                item_id: item.uuid.clone(),
            }),
        });
        let ResponseResult::PluginResourceDeleted { existed, .. } = response_result(&direct_delete)
        else {
            panic!("expected plugin resource delete: {direct_delete}");
        };
        assert!(existed);
        let event_cursor = app.event_hub.current_sequence();

        let delete = handle_api_request(
            &mut app,
            "delete".into(),
            &Method::KanbanDelete(KanbanDeleteParams {
                uuid: item.uuid.clone(),
            }),
        )
        .expect("kanban.delete should be handled");
        let deleted = kanban_item_from_response(&delete);
        assert_eq!(deleted.uuid, item.uuid);

        let events = app.event_hub.events_after(event_cursor);
        assert_eq!(events.len(), 2);
        assert_plugin_resource_events_include(
            &events,
            crate::extensions::kanban::resources::BUILTIN_PLUGIN_ID,
            "resource.delete",
            crate::extensions::kanban::resources::CARD_RESOURCE_ID,
            &item.uuid,
            None,
        );
        assert!(
            !plugin_resource_events_include(
                &events,
                "example.board",
                "resource.delete",
                "cards",
                &item.uuid,
                None,
            ),
            "deleting an already removed external mirror should not emit an external resource event"
        );
        assert_kanban_event_includes_deleted(&events, &item.uuid);

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        restore_xdg_home(old_config_home, old_state_home);
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
        let kanban_events = events
            .iter()
            .filter(|(_, event)| {
                matches!(
                    event.event,
                    crate::api::schema::EventKind::KanbanAdded
                        | crate::api::schema::EventKind::KanbanUpdated
                        | crate::api::schema::EventKind::KanbanDeleted
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(kanban_events.len(), 3);
        assert!(matches!(
            &kanban_events[0].1.data,
            crate::api::schema::EventData::KanbanAdded { item: added }
                if kanban_events[0].1.event == crate::api::schema::EventKind::KanbanAdded
                    && added.uuid == item.uuid
                    && added.title == "Streamed card"
        ));
        assert!(matches!(
            &kanban_events[1].1.data,
            crate::api::schema::EventData::KanbanUpdated { item: updated }
                if kanban_events[1].1.event == crate::api::schema::EventKind::KanbanUpdated
                    && updated.uuid == item.uuid
                    && updated.title == "Updated streamed card"
                    && updated.status == crate::api::schema::KanbanStatus::Reviewing
        ));
        assert!(matches!(
            &kanban_events[2].1.data,
            crate::api::schema::EventData::KanbanDeleted { item: deleted }
                if kanban_events[2].1.event == crate::api::schema::EventKind::KanbanDeleted
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

    fn response_result(response: &str) -> ResponseResult {
        serde_json::from_str::<SuccessResponse>(response)
            .unwrap()
            .result
    }

    fn assert_plugin_resource_events_include(
        events: &[(u64, crate::api::schema::EventEnvelope)],
        expected_plugin_id: &str,
        expected_event: &str,
        expected_resource_id: &str,
        expected_item_id: &str,
        expected_title: Option<&str>,
    ) {
        assert!(
            plugin_resource_events_include(
                events,
                expected_plugin_id,
                expected_event,
                expected_resource_id,
                expected_item_id,
                expected_title,
            ),
            "expected {expected_plugin_id} {expected_event} for {expected_resource_id}/{expected_item_id}"
        );
    }

    fn plugin_resource_events_include(
        events: &[(u64, crate::api::schema::EventEnvelope)],
        expected_plugin_id: &str,
        expected_event: &str,
        expected_resource_id: &str,
        expected_item_id: &str,
        expected_title: Option<&str>,
    ) -> bool {
        events.iter().any(|(_, event)| {
            plugin_resource_event_matches(
                event,
                expected_plugin_id,
                expected_event,
                expected_resource_id,
                expected_item_id,
                expected_title,
            )
        })
    }

    fn plugin_resource_event_matches(
        event: &crate::api::schema::EventEnvelope,
        expected_plugin_id: &str,
        expected_event: &str,
        expected_resource_id: &str,
        expected_item_id: &str,
        expected_title: Option<&str>,
    ) -> bool {
        if event.event != crate::api::schema::EventKind::PluginEvent {
            return false;
        }
        let crate::api::schema::EventData::PluginEvent {
            plugin_id,
            event,
            payload,
        } = &event.data
        else {
            return false;
        };
        if plugin_id != expected_plugin_id
            || event != expected_event
            || payload["resource_id"] != expected_resource_id
            || payload["item_id"] != expected_item_id
        {
            return false;
        }
        if let Some(expected_title) = expected_title {
            payload["value"]["title"] == expected_title
        } else {
            payload.get("value").is_none()
        }
    }

    fn assert_kanban_event_includes_added(
        events: &[(u64, crate::api::schema::EventEnvelope)],
        uuid: &str,
    ) {
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                crate::api::schema::EventData::KanbanAdded { item }
                    if event.event == crate::api::schema::EventKind::KanbanAdded
                        && item.uuid == uuid
            )
        }));
    }

    fn assert_kanban_event_includes_updated(
        events: &[(u64, crate::api::schema::EventEnvelope)],
        uuid: &str,
    ) {
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                crate::api::schema::EventData::KanbanUpdated { item }
                    if event.event == crate::api::schema::EventKind::KanbanUpdated
                        && item.uuid == uuid
            )
        }));
    }

    fn assert_kanban_event_includes_deleted(
        events: &[(u64, crate::api::schema::EventEnvelope)],
        uuid: &str,
    ) {
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                crate::api::schema::EventData::KanbanDeleted { item }
                    if event.event == crate::api::schema::EventKind::KanbanDeleted
                        && item.uuid == uuid
            )
        }));
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

    fn link_board_plugin(app: &mut App, root: &std::path::Path, enabled: bool) {
        let response = app.handle_api_request(Request {
            id: "link".into(),
            method: Method::PluginLink(PluginLinkParams {
                path: root.display().to_string(),
                enabled,
                source: None,
            }),
        });
        assert!(
            response.contains("plugin_linked"),
            "link failed: {response}"
        );
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("herdr-{name}-{}-{nanos}", std::process::id()))
    }

    fn write_board_manifest(root: &std::path::Path) {
        write_manifest_content(
            root,
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
    }

    fn write_manifest_content(root: &std::path::Path, content: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("herdr-plugin.toml"), content).unwrap();
    }

    fn restore_xdg_home(
        old_config_home: Option<std::ffi::OsString>,
        old_state_home: Option<std::ffi::OsString>,
    ) {
        match old_config_home {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_state_home {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
    }
}

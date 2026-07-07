// Allow dead code in app/api/kanban.rs when the kanban feature is disabled.
#![allow(dead_code)]

use crate::api::schema::{
    EventData, EventEnvelope, EventKind, KanbanAddParams, KanbanDeleteParams, KanbanItem,
    KanbanListParams, KanbanUpdateParams, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

impl App {
    fn emit_kanban_event(&mut self, event: EventKind, data: EventData) {
        self.emit_event(EventEnvelope { event, data });
    }

    fn emit_kanban_added(&mut self, item: KanbanItem) {
        self.emit_kanban_event(EventKind::KanbanAdded, EventData::KanbanAdded { item });
    }

    fn emit_kanban_updated(&mut self, item: KanbanItem) {
        self.emit_kanban_event(EventKind::KanbanUpdated, EventData::KanbanUpdated { item });
    }

    fn emit_kanban_deleted(&mut self, item: KanbanItem) {
        self.emit_kanban_event(EventKind::KanbanDeleted, EventData::KanbanDeleted { item });
    }

    fn active_pane_terminal_ids(&self) -> std::collections::HashSet<String> {
        let mut active = std::collections::HashSet::new();
        for ws in &self.state.workspaces {
            for tab in &ws.tabs {
                for pane in tab.panes.values() {
                    active.insert(pane.attached_terminal_id.to_string());
                }
            }
        }
        active
    }
    pub(crate) fn handle_kanban_add(&mut self, id: String, params: KanbanAddParams) -> String {
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
            if let Ok(resolved) = self.resolve_terminal_target(&tid) {
                resolved.terminal_id
            } else {
                tid
            }
        });
        let active_tids = self.active_pane_terminal_ids();
        if self
            .state
            .extensions
            .kanban
            .clear_dead_terminals(|tid| active_tids.contains(tid))
        {
            self.state.mark_session_dirty();
            self.schedule_session_save();
        }

        let item = self.state.extensions.kanban.add_item(
            params.title,
            params.description,
            params.status,
            terminal_id,
        );
        self.state.mark_session_dirty();
        self.schedule_session_save();
        self.emit_kanban_added(item.clone());
        encode_success(id, ResponseResult::KanbanItem { item })
    }

    pub(crate) fn handle_kanban_list(&mut self, id: String, params: KanbanListParams) -> String {
        let target_terminal_id = params.terminal_id.map(|tid| {
            if let Ok(resolved) = self.resolve_terminal_target(&tid) {
                resolved.terminal_id
            } else {
                tid
            }
        });

        let active_tids = self.active_pane_terminal_ids();
        if self
            .state
            .extensions
            .kanban
            .clear_dead_terminals(|tid| active_tids.contains(tid))
        {
            self.state.mark_session_dirty();
            self.schedule_session_save();
        }

        let items = self
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

    pub(crate) fn handle_kanban_update(
        &mut self,
        id: String,
        params: KanbanUpdateParams,
    ) -> String {
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
            if let Ok(resolved) = self.resolve_terminal_target(&tid) {
                resolved.terminal_id
            } else {
                tid
            }
        });
        let active_tids = self.active_pane_terminal_ids();
        if self
            .state
            .extensions
            .kanban
            .clear_dead_terminals(|tid| active_tids.contains(tid))
        {
            self.state.mark_session_dirty();
            self.schedule_session_save();
        }

        match self.state.extensions.kanban.update_item(
            &params.uuid,
            params.title,
            params.description,
            params.status,
            terminal_id,
            params.clear_terminal_id,
        ) {
            Some(item) => {
                self.state.mark_session_dirty();
                self.schedule_session_save();
                self.emit_kanban_updated(item.clone());
                encode_success(id, ResponseResult::KanbanItem { item })
            }
            None => encode_error(
                id,
                "kanban_item_not_found",
                format!("Kanban item with uuid {} not found", params.uuid),
            ),
        }
    }

    pub(crate) fn handle_kanban_delete(
        &mut self,
        id: String,
        params: KanbanDeleteParams,
    ) -> String {
        let active_tids = self.active_pane_terminal_ids();
        if self
            .state
            .extensions
            .kanban
            .clear_dead_terminals(|tid| active_tids.contains(tid))
        {
            self.state.mark_session_dirty();
            self.schedule_session_save();
        }

        match self.state.extensions.kanban.delete_item(&params.uuid) {
            Some(item) => {
                self.state.mark_session_dirty();
                self.schedule_session_save();
                self.emit_kanban_deleted(item.clone());
                encode_success(id, ResponseResult::KanbanItem { item })
            }
            None => encode_error(
                id,
                "kanban_item_not_found",
                format!("Kanban item with uuid {} not found", params.uuid),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::SuccessResponse;
    use crate::config::Config;

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
        let add_res = app.handle_kanban_add(
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
        let list_res = app.handle_kanban_list(
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
        let list_res = app.handle_kanban_list(
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
        let update_res = app.handle_kanban_update(
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
        let list_res = app.handle_kanban_list(
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
        let update_res = app.handle_kanban_update(
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
        let delete_res = app.handle_kanban_delete(
            "4".into(),
            KanbanDeleteParams {
                uuid: item.uuid.clone(),
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&delete_res).unwrap();
        assert!(matches!(resp.result, ResponseResult::KanbanItem { .. }));

        // List after delete should be empty
        let list_res = app.handle_kanban_list(
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

        let add_res = app.handle_kanban_add(
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

        app.handle_kanban_update(
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
        app.handle_kanban_delete(
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
        let add_res = app.handle_kanban_add(
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
        let add_res2 = app.handle_kanban_add(
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
}

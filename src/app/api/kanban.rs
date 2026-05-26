use super::responses::{encode_error, encode_success};
use crate::api::schema::{
    KanbanAddParams, KanbanDeleteParams, KanbanListParams, KanbanUpdateParams, ResponseResult,
};
use crate::app::App;

impl App {
    pub(super) fn handle_kanban_add(&mut self, id: String, params: KanbanAddParams) -> String {
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
        if self.state.clear_dead_kanban_terminals() {
            self.schedule_session_save();
        }

        let item = self.state.add_kanban_item(
            params.title,
            params.description,
            params.status,
            terminal_id,
        );
        self.schedule_session_save();
        encode_success(id, ResponseResult::KanbanItem { item })
    }

    pub(super) fn handle_kanban_list(&mut self, id: String, params: KanbanListParams) -> String {
        let target_terminal_id = params.terminal_id.map(|tid| {
            if let Ok(resolved) = self.resolve_terminal_target(&tid) {
                resolved.terminal_id
            } else {
                tid
            }
        });

        if self.state.clear_dead_kanban_terminals() {
            self.schedule_session_save();
        }

        let items = self
            .state
            .kanban_items
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

    pub(super) fn handle_kanban_update(
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
        if self.state.clear_dead_kanban_terminals() {
            self.schedule_session_save();
        }

        match self.state.update_kanban_item(
            &params.uuid,
            params.title,
            params.description,
            params.status,
            terminal_id,
            params.clear_terminal_id,
        ) {
            Some(item) => {
                self.schedule_session_save();
                encode_success(id, ResponseResult::KanbanItem { item })
            }
            None => encode_error(
                id,
                "kanban_item_not_found",
                format!("Kanban item with uuid {} not found", params.uuid),
            ),
        }
    }

    pub(super) fn handle_kanban_delete(
        &mut self,
        id: String,
        params: KanbanDeleteParams,
    ) -> String {
        if self.state.clear_dead_kanban_terminals() {
            self.schedule_session_save();
        }

        match self.state.delete_kanban_item(&params.uuid) {
            Some(item) => {
                self.schedule_session_save();
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
                status: Some(crate::api::schema::KanbanStatus::InProgress),
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
            crate::api::schema::KanbanStatus::InProgress
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
        let (text, is_err) = crate::ui::get_description_text(&plan_path);
        assert_eq!(text, "API validation check");
        assert!(!is_err);

        // 4. UI Helper handles deleted file correctly
        std::fs::remove_file(&plan_file).unwrap();
        let (text2, is_err2) = crate::ui::get_description_text(&plan_path);
        assert_eq!(text2, "NO DESCRIPTION FOUND");
        assert!(is_err2);
    }
}

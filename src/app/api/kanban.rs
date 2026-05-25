use crate::api::schema::{
    KanbanAddParams, KanbanDeleteParams, KanbanListParams, KanbanUpdateParams, ResponseResult,
};
use crate::app::App;
use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_kanban_add(&mut self, id: String, params: KanbanAddParams) -> String {
        let item = self.state.add_kanban_item(params.title, params.description, params.status);
        self.schedule_session_save();
        encode_success(id, ResponseResult::KanbanItem { item })
    }

    pub(super) fn handle_kanban_list(&mut self, id: String, params: KanbanListParams) -> String {
        let items = if let Some(status) = params.status {
            self.state
                .kanban_items
                .iter()
                .filter(|item| item.status == status)
                .cloned()
                .collect()
        } else {
            self.state.kanban_items.clone()
        };
        encode_success(id, ResponseResult::KanbanList { items })
    }

    pub(super) fn handle_kanban_update(&mut self, id: String, params: KanbanUpdateParams) -> String {
        match self.state.update_kanban_item(
            &params.uuid,
            params.title,
            params.description,
            params.status,
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

    pub(super) fn handle_kanban_delete(&mut self, id: String, params: KanbanDeleteParams) -> String {
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
    use crate::config::Config;
    use crate::api::schema::SuccessResponse;

    #[test]
    fn test_kanban_api_handlers() {
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
                description: Some("Description".into()),
                status: None,
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&add_res).unwrap();
        let item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };
        assert_eq!(item.title, "Test Kanban");

        // List
        let list_res = app.handle_kanban_list("2".into(), KanbanListParams { status: None });
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 1);

        // Update
        let update_res = app.handle_kanban_update(
            "3".into(),
            KanbanUpdateParams {
                uuid: item.uuid.clone(),
                title: Some("Updated Title".into()),
                description: None,
                status: Some(crate::api::schema::KanbanStatus::InProgress),
            },
        );
        let resp: SuccessResponse = serde_json::from_str(&update_res).unwrap();
        let updated_item = match resp.result {
            ResponseResult::KanbanItem { item } => item,
            _ => panic!("Expected ResponseResult::KanbanItem"),
        };
        assert_eq!(updated_item.title, "Updated Title");
        assert_eq!(updated_item.status, crate::api::schema::KanbanStatus::InProgress);

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
        let list_res = app.handle_kanban_list("5".into(), KanbanListParams { status: None });
        let resp: SuccessResponse = serde_json::from_str(&list_res).unwrap();
        let items = match resp.result {
            ResponseResult::KanbanList { items } => items,
            _ => panic!("Expected ResponseResult::KanbanList"),
        };
        assert_eq!(items.len(), 0);
    }
}

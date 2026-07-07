use crate::api::schema::EventData;

pub(crate) fn plugin_invocation_source(event_data: &EventData) -> Option<String> {
    match event_data {
        EventData::KanbanAdded { item }
        | EventData::KanbanUpdated { item }
        | EventData::KanbanDeleted { item } => Some(format!("kanban:{}", item.uuid)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_item(uuid: &str) -> crate::api::schema::KanbanItem {
        crate::api::schema::KanbanItem {
            uuid: uuid.into(),
            title: "Card".into(),
            description: String::new(),
            status: crate::api::schema::KanbanStatus::Todo,
            terminal_id: None,
        }
    }

    #[test]
    fn kanban_events_provide_plugin_invocation_source() {
        assert_eq!(
            plugin_invocation_source(&EventData::KanbanAdded {
                item: test_item("card-1")
            })
            .as_deref(),
            Some("kanban:card-1")
        );
    }
}

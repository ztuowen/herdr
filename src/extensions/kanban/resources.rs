use crate::api::schema::{EventData, EventEnvelope, EventKind, KanbanItem, KanbanStatus};
use crate::app::App;

pub(crate) const CARD_RESOURCE_KIND: &str = "application/vnd.herdr.kanban-card+json";

pub(crate) fn create_card(
    app: &mut App,
    title: String,
    description: Option<String>,
    status: Option<KanbanStatus>,
    terminal_id: Option<String>,
) -> KanbanItem {
    let item = app
        .state
        .extensions
        .kanban
        .add_item(title, description, status, terminal_id);
    mark_cards_changed(app);
    mirror_card(app, &item);
    emit_card_event(
        app,
        EventKind::KanbanAdded,
        EventData::KanbanAdded { item: item.clone() },
    );
    item
}

pub(crate) fn update_card(
    app: &mut App,
    uuid: &str,
    title: Option<String>,
    description: Option<String>,
    status: Option<KanbanStatus>,
    terminal_id: Option<String>,
    clear_terminal_id: Option<bool>,
) -> Option<KanbanItem> {
    let item = app.state.extensions.kanban.update_item(
        uuid,
        title,
        description,
        status,
        terminal_id,
        clear_terminal_id,
    )?;
    mark_cards_changed(app);
    mirror_card(app, &item);
    emit_card_event(
        app,
        EventKind::KanbanUpdated,
        EventData::KanbanUpdated { item: item.clone() },
    );
    Some(item)
}

pub(crate) fn delete_card(app: &mut App, uuid: &str) -> Option<KanbanItem> {
    let item = app.state.extensions.kanban.delete_item(uuid)?;
    mark_cards_changed(app);
    delete_card_mirror(app, &item);
    emit_card_event(
        app,
        EventKind::KanbanDeleted,
        EventData::KanbanDeleted { item: item.clone() },
    );
    Some(item)
}

pub(crate) fn mirror_card(app: &mut App, item: &KanbanItem) {
    let Ok(value) = serde_json::to_value(item) else {
        tracing::warn!(uuid = %item.uuid, "failed to serialize kanban card for plugin resource mirror");
        return;
    };
    app.mirror_plugin_resource_kind_put(CARD_RESOURCE_KIND, &item.uuid, value);
}

pub(crate) fn delete_card_mirror(app: &mut App, item: &KanbanItem) {
    app.mirror_plugin_resource_kind_delete(CARD_RESOURCE_KIND, &item.uuid);
}

pub(crate) fn backfill_existing_cards(app: &mut App) {
    let items = app.state.extensions.kanban.items.clone();
    for item in items {
        mirror_card(app, &item);
    }
}

fn mark_cards_changed(app: &mut App) {
    app.state.mark_session_dirty();
    app.schedule_session_save();
}

fn emit_card_event(app: &mut App, event: EventKind, data: EventData) {
    app.emit_event(EventEnvelope { event, data });
}

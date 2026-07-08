use crate::api::schema::{
    EventData, EventEnvelope, EventKind, KanbanItem, KanbanStatus, PluginResourceItems,
};
use crate::app::App;

pub(crate) const BUILTIN_PLUGIN_ID: &str = "herdr.kanban";
pub(crate) const CARD_RESOURCE_ID: &str = "cards";
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
    emit_builtin_resource_put(app, &item);
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
    emit_builtin_resource_put(app, &item);
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
    emit_builtin_resource_delete(app, &item.uuid);
    delete_card_mirror(app, &item);
    emit_card_event(
        app,
        EventKind::KanbanDeleted,
        EventData::KanbanDeleted { item: item.clone() },
    );
    Some(item)
}

pub(crate) fn is_builtin_plugin(plugin_id: &str) -> bool {
    plugin_id == BUILTIN_PLUGIN_ID
}

pub(crate) fn is_builtin_card_resource(plugin_id: &str, resource_id: &str) -> bool {
    is_builtin_plugin(plugin_id) && resource_id == CARD_RESOURCE_ID
}

pub(crate) fn list_card_resources(app: &App) -> PluginResourceItems {
    app.state
        .extensions
        .kanban
        .items
        .iter()
        .filter_map(|item| {
            serde_json::to_value(item)
                .map(|value| (item.uuid.clone(), value))
                .map_err(|err| {
                    tracing::warn!(
                        uuid = %item.uuid,
                        error = %err,
                        "failed to serialize kanban card resource"
                    );
                })
                .ok()
        })
        .collect()
}

pub(crate) fn get_card_resource(app: &App, item_id: &str) -> Option<serde_json::Value> {
    let item = app
        .state
        .extensions
        .kanban
        .items
        .iter()
        .find(|item| item.uuid == item_id)?;
    serde_json::to_value(item)
        .map_err(|err| {
            tracing::warn!(
                uuid = %item.uuid,
                error = %err,
                "failed to serialize kanban card resource"
            );
        })
        .ok()
}

pub(crate) fn put_card_resource(
    app: &mut App,
    item_id: &str,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let item = serde_json::from_value::<KanbanItem>(value).map_err(|err| {
        format!("kanban card resource value must match the KanbanItem schema: {err}")
    })?;
    if item.uuid != item_id {
        return Err("kanban card resource value uuid must match item_id".to_string());
    }

    let mut event = EventKind::KanbanAdded;
    let mut data = EventData::KanbanAdded { item: item.clone() };
    if let Some(existing) = app
        .state
        .extensions
        .kanban
        .items
        .iter_mut()
        .find(|existing| existing.uuid == item_id)
    {
        *existing = item.clone();
        event = EventKind::KanbanUpdated;
        data = EventData::KanbanUpdated { item: item.clone() };
    } else {
        app.state.extensions.kanban.items.push(item.clone());
    }

    mark_cards_changed(app);
    emit_builtin_resource_put(app, &item);
    mirror_card(app, &item);
    emit_card_event(app, event, data);
    serde_json::to_value(item)
        .map_err(|err| format!("failed to serialize kanban card resource after storing it: {err}"))
}

pub(crate) fn delete_card_resource(app: &mut App, item_id: &str) -> bool {
    let Some(item) = app.state.extensions.kanban.delete_item(item_id) else {
        return false;
    };
    mark_cards_changed(app);
    emit_builtin_resource_delete(app, &item.uuid);
    delete_card_mirror(app, &item);
    emit_card_event(
        app,
        EventKind::KanbanDeleted,
        EventData::KanbanDeleted { item },
    );
    true
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

fn emit_builtin_resource_put(app: &mut App, item: &KanbanItem) {
    let Ok(value) = serde_json::to_value(item) else {
        tracing::warn!(uuid = %item.uuid, "failed to serialize kanban card resource event");
        return;
    };
    emit_builtin_resource_event(
        app,
        "resource.put",
        serde_json::json!({
            "resource_id": CARD_RESOURCE_ID,
            "item_id": item.uuid,
            "value": value,
        }),
    );
}

fn emit_builtin_resource_delete(app: &mut App, item_id: &str) {
    emit_builtin_resource_event(
        app,
        "resource.delete",
        serde_json::json!({
            "resource_id": CARD_RESOURCE_ID,
            "item_id": item_id,
        }),
    );
}

fn emit_builtin_resource_event(app: &mut App, event: &str, payload: serde_json::Value) {
    app.emit_event(EventEnvelope {
        event: EventKind::PluginEvent,
        data: EventData::PluginEvent {
            plugin_id: BUILTIN_PLUGIN_ID.to_string(),
            event: event.to_string(),
            payload,
        },
    });
}

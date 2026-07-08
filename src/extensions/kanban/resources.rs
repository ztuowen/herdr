use crate::api::schema::KanbanItem;
use crate::app::App;

pub(crate) const CARD_RESOURCE_KIND: &str = "application/vnd.herdr.kanban-card+json";

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

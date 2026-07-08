use crate::api::schema::{
    EventData, EventEnvelope, EventKind, InstalledPluginInfo, PluginCapability,
    PluginManifestResource, PluginResourceDeleteParams, PluginResourceGetParams,
    PluginResourceItems, PluginResourceListParams, PluginResourcePutParams, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

use super::storage;

impl App {
    pub(crate) fn mirror_plugin_resource_kind_put(
        &mut self,
        kind: &str,
        item_id: &str,
        value: serde_json::Value,
    ) -> usize {
        let targets = self.plugin_resource_kind_targets(kind);
        let mut mirrored = 0;
        for (plugin, resource) in targets {
            match write_plugin_resource_item(&plugin, &resource, item_id, value.clone()) {
                Ok(()) => {
                    mirrored += 1;
                    self.emit_plugin_resource_put_event(
                        &plugin.plugin_id,
                        &resource.id,
                        item_id,
                        value.clone(),
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        plugin_id = %plugin.plugin_id,
                        resource_id = %resource.id,
                        item_id,
                        error = %err,
                        "failed to mirror plugin resource item"
                    );
                }
            }
        }
        mirrored
    }

    pub(crate) fn mirror_plugin_resource_kind_delete(
        &mut self,
        kind: &str,
        item_id: &str,
    ) -> usize {
        let targets = self.plugin_resource_kind_targets(kind);
        let mut mirrored = 0;
        for (plugin, resource) in targets {
            match delete_plugin_resource_item(&plugin, &resource, item_id) {
                Ok(existed) => {
                    if existed {
                        mirrored += 1;
                        self.emit_plugin_resource_delete_event(
                            &plugin.plugin_id,
                            &resource.id,
                            item_id,
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        plugin_id = %plugin.plugin_id,
                        resource_id = %resource.id,
                        item_id,
                        error = %err,
                        "failed to mirror plugin resource delete"
                    );
                }
            }
        }
        mirrored
    }

    fn plugin_resource_kind_targets(
        &self,
        kind: &str,
    ) -> Vec<(InstalledPluginInfo, PluginManifestResource)> {
        self.state
            .installed_plugins
            .values()
            .filter(|plugin| {
                plugin.enabled
                    && super::manifest::plugin_has_capability(plugin, PluginCapability::Resources)
            })
            .flat_map(|plugin| {
                plugin
                    .resources
                    .iter()
                    .filter(move |resource| resource.kind == kind)
                    .cloned()
                    .map(|resource| (plugin.clone(), resource))
            })
            .collect()
    }

    pub(in crate::app::api) fn handle_plugin_resource_list(
        &mut self,
        id: String,
        params: PluginResourceListParams,
    ) -> String {
        if let Some((plugin_id, resource_id)) = match normalize_builtin_kanban_card_resource(
            &id,
            params.plugin_id.clone(),
            params.resource_id.clone(),
        ) {
            Ok(resource) => resource,
            Err(response) => return response,
        } {
            return encode_success(
                id,
                ResponseResult::PluginResourceList {
                    plugin_id,
                    resource_id,
                    items: crate::extensions::kanban::resources::list_card_resources(self),
                },
            );
        }

        let (plugin_id, resource) =
            match self.normalize_plugin_resource(&id, params.plugin_id, params.resource_id) {
                Ok(request) => request,
                Err(response) => return response,
            };
        let document = match storage::read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let items = document
            .into_iter()
            .filter_map(|(key, value)| {
                let item_id = key.strip_prefix(&resource.storage_prefix)?;
                (super::manifest::normalize_action_id(item_id).as_deref() == Some(item_id))
                    .then(|| (item_id.to_string(), value))
            })
            .collect::<PluginResourceItems>();
        encode_success(
            id,
            ResponseResult::PluginResourceList {
                plugin_id,
                resource_id: resource.id,
                items,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_resource_get(
        &mut self,
        id: String,
        params: PluginResourceGetParams,
    ) -> String {
        if let Some((plugin_id, resource_id, item_id)) =
            match normalize_builtin_kanban_card_resource_item(
                &id,
                params.plugin_id.clone(),
                params.resource_id.clone(),
                params.item_id.clone(),
            ) {
                Ok(resource) => resource,
                Err(response) => return response,
            }
        {
            return encode_success(
                id,
                ResponseResult::PluginResourceValue {
                    plugin_id,
                    resource_id,
                    item_id: item_id.clone(),
                    value: crate::extensions::kanban::resources::get_card_resource(self, &item_id),
                },
            );
        }

        let (plugin_id, resource, item_id, storage_key) = match self.normalize_plugin_resource_item(
            &id,
            params.plugin_id,
            params.resource_id,
            params.item_id,
        ) {
            Ok(request) => request,
            Err(response) => return response,
        };
        let document = match storage::read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let value = document.get(&storage_key).cloned();
        encode_success(
            id,
            ResponseResult::PluginResourceValue {
                plugin_id,
                resource_id: resource.id,
                item_id,
                value,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_resource_put(
        &mut self,
        id: String,
        params: PluginResourcePutParams,
    ) -> String {
        if let Some((plugin_id, resource_id, item_id)) =
            match normalize_builtin_kanban_card_resource_item(
                &id,
                params.plugin_id.clone(),
                params.resource_id.clone(),
                params.item_id.clone(),
            ) {
                Ok(resource) => resource,
                Err(response) => return response,
            }
        {
            if let Err(response) = storage::validate_storage_value(&id, &params.value) {
                return response;
            }
            let value = match crate::extensions::kanban::resources::put_card_resource(
                self,
                &item_id,
                params.value,
            ) {
                Ok(value) => value,
                Err(message) => {
                    return encode_error(id, "invalid_plugin_resource_value", message);
                }
            };
            return encode_success(
                id,
                ResponseResult::PluginResourcePut {
                    plugin_id,
                    resource_id,
                    item_id,
                    value,
                },
            );
        }

        let (plugin_id, resource, item_id, storage_key) = match self.normalize_plugin_resource_item(
            &id,
            params.plugin_id,
            params.resource_id,
            params.item_id,
        ) {
            Ok(request) => request,
            Err(response) => return response,
        };
        if let Err(response) = storage::validate_storage_value(&id, &params.value) {
            return response;
        }
        let mut document = match storage::read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        if let Err(response) = storage::validate_storage_entry_count(&id, &document, &storage_key) {
            return response;
        }
        document.insert(storage_key, params.value.clone());
        if let Err(response) = storage::write_storage_document(&id, &plugin_id, &document) {
            return response;
        }
        self.emit_plugin_resource_put_event(
            &plugin_id,
            &resource.id,
            &item_id,
            params.value.clone(),
        );
        encode_success(
            id,
            ResponseResult::PluginResourcePut {
                plugin_id,
                resource_id: resource.id,
                item_id,
                value: params.value,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_resource_delete(
        &mut self,
        id: String,
        params: PluginResourceDeleteParams,
    ) -> String {
        if let Some((plugin_id, resource_id, item_id)) =
            match normalize_builtin_kanban_card_resource_item(
                &id,
                params.plugin_id.clone(),
                params.resource_id.clone(),
                params.item_id.clone(),
            ) {
                Ok(resource) => resource,
                Err(response) => return response,
            }
        {
            let existed =
                crate::extensions::kanban::resources::delete_card_resource(self, &item_id);
            return encode_success(
                id,
                ResponseResult::PluginResourceDeleted {
                    plugin_id,
                    resource_id,
                    item_id,
                    existed,
                },
            );
        }

        let (plugin_id, resource, item_id, storage_key) = match self.normalize_plugin_resource_item(
            &id,
            params.plugin_id,
            params.resource_id,
            params.item_id,
        ) {
            Ok(request) => request,
            Err(response) => return response,
        };
        let mut document = match storage::read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let existed = document.remove(&storage_key).is_some();
        if let Err(response) = storage::write_storage_document(&id, &plugin_id, &document) {
            return response;
        }
        if existed {
            self.emit_plugin_resource_delete_event(&plugin_id, &resource.id, &item_id);
        }
        encode_success(
            id,
            ResponseResult::PluginResourceDeleted {
                plugin_id,
                resource_id: resource.id,
                item_id,
                existed,
            },
        )
    }

    fn normalize_plugin_resource(
        &self,
        id: &str,
        plugin_id: String,
        resource_id: String,
    ) -> Result<(String, PluginManifestResource), String> {
        let plugin_id = storage::normalize_plugin_storage_id(id, plugin_id)?;
        let resource_id = super::manifest::normalize_action_id(&resource_id).ok_or_else(|| {
            encode_error(
                id.to_string(),
                "invalid_plugin_resource_id",
                "invalid resource id",
            )
        })?;
        let Some(plugin) = self.state.installed_plugins.get(&plugin_id) else {
            return Err(encode_error(
                id.to_string(),
                "plugin_not_found",
                format!("plugin {plugin_id} is not installed"),
            ));
        };
        if !super::manifest::plugin_has_capability(plugin, PluginCapability::Resources) {
            return Err(encode_error(
                id.to_string(),
                "plugin_capability_required",
                "plugin resources require capability 'resources'",
            ));
        }
        let Some(resource) = plugin
            .resources
            .iter()
            .find(|resource| resource.id == resource_id)
            .cloned()
        else {
            return Err(encode_error(
                id.to_string(),
                "plugin_resource_not_found",
                format!("plugin resource '{resource_id}' not found"),
            ));
        };
        super::env::ensure_plugin_user_dirs(plugin).map_err(|err| {
            encode_error(
                id.to_string(),
                "plugin_user_dir_create_failed",
                err.to_string(),
            )
        })?;
        Ok((plugin_id, resource))
    }

    fn normalize_plugin_resource_item(
        &self,
        id: &str,
        plugin_id: String,
        resource_id: String,
        item_id: String,
    ) -> Result<(String, PluginManifestResource, String, String), String> {
        let (plugin_id, resource) = self.normalize_plugin_resource(id, plugin_id, resource_id)?;
        let item_id = super::manifest::normalize_action_id(&item_id).ok_or_else(|| {
            encode_error(
                id.to_string(),
                "invalid_plugin_resource_item_id",
                "invalid resource item id",
            )
        })?;
        let storage_key = format!("{}{}", resource.storage_prefix, item_id);
        storage::validate_storage_key(id, &storage_key)?;
        Ok((plugin_id, resource, item_id, storage_key))
    }

    fn emit_plugin_resource_put_event(
        &mut self,
        plugin_id: &str,
        resource_id: &str,
        item_id: &str,
        value: serde_json::Value,
    ) {
        self.emit_event(EventEnvelope {
            event: EventKind::PluginEvent,
            data: EventData::PluginEvent {
                plugin_id: plugin_id.to_string(),
                event: "resource.put".to_string(),
                payload: serde_json::json!({
                    "resource_id": resource_id,
                    "item_id": item_id,
                    "value": value,
                }),
            },
        });
    }

    fn emit_plugin_resource_delete_event(
        &mut self,
        plugin_id: &str,
        resource_id: &str,
        item_id: &str,
    ) {
        self.emit_event(EventEnvelope {
            event: EventKind::PluginEvent,
            data: EventData::PluginEvent {
                plugin_id: plugin_id.to_string(),
                event: "resource.delete".to_string(),
                payload: serde_json::json!({
                    "resource_id": resource_id,
                    "item_id": item_id,
                }),
            },
        });
    }
}

fn normalize_builtin_kanban_card_resource(
    id: &str,
    plugin_id: String,
    resource_id: String,
) -> Result<Option<(String, String)>, String> {
    let plugin_id = storage::normalize_plugin_storage_id(id, plugin_id)?;
    let resource_id = super::manifest::normalize_action_id(&resource_id).ok_or_else(|| {
        encode_error(
            id.to_string(),
            "invalid_plugin_resource_id",
            "invalid resource id",
        )
    })?;
    if crate::extensions::kanban::resources::is_builtin_card_resource(&plugin_id, &resource_id) {
        return Ok(Some((plugin_id, resource_id)));
    }
    if crate::extensions::kanban::resources::is_builtin_plugin(&plugin_id) {
        return Err(encode_error(
            id.to_string(),
            "plugin_resource_not_found",
            format!("plugin resource '{resource_id}' not found"),
        ));
    }
    Ok(None)
}

fn normalize_builtin_kanban_card_resource_item(
    id: &str,
    plugin_id: String,
    resource_id: String,
    item_id: String,
) -> Result<Option<(String, String, String)>, String> {
    let Some((plugin_id, resource_id)) =
        normalize_builtin_kanban_card_resource(id, plugin_id, resource_id)?
    else {
        return Ok(None);
    };
    let item_id = super::manifest::normalize_action_id(&item_id).ok_or_else(|| {
        encode_error(
            id.to_string(),
            "invalid_plugin_resource_item_id",
            "invalid resource item id",
        )
    })?;
    Ok(Some((plugin_id, resource_id, item_id)))
}

fn write_plugin_resource_item(
    plugin: &InstalledPluginInfo,
    resource: &PluginManifestResource,
    item_id: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    super::env::ensure_plugin_user_dirs(plugin).map_err(|err| err.to_string())?;
    let item_id = super::manifest::normalize_action_id(item_id)
        .ok_or_else(|| "invalid resource item id".to_string())?;
    let storage_key = format!("{}{}", resource.storage_prefix, item_id);
    storage::validate_storage_key("plugin_resource_mirror", &storage_key)?;
    storage::validate_storage_value("plugin_resource_mirror", &value)?;
    let mut document = storage::read_storage_document("plugin_resource_mirror", &plugin.plugin_id)?;
    storage::validate_storage_entry_count("plugin_resource_mirror", &document, &storage_key)?;
    document.insert(storage_key, value);
    storage::write_storage_document("plugin_resource_mirror", &plugin.plugin_id, &document)
}

fn delete_plugin_resource_item(
    plugin: &InstalledPluginInfo,
    resource: &PluginManifestResource,
    item_id: &str,
) -> Result<bool, String> {
    super::env::ensure_plugin_user_dirs(plugin).map_err(|err| err.to_string())?;
    let item_id = super::manifest::normalize_action_id(item_id)
        .ok_or_else(|| "invalid resource item id".to_string())?;
    let storage_key = format!("{}{}", resource.storage_prefix, item_id);
    storage::validate_storage_key("plugin_resource_mirror", &storage_key)?;
    let mut document = storage::read_storage_document("plugin_resource_mirror", &plugin.plugin_id)?;
    let existed = document.remove(&storage_key).is_some();
    storage::write_storage_document("plugin_resource_mirror", &plugin.plugin_id, &document)?;
    Ok(existed)
}

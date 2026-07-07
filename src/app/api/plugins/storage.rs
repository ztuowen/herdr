use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::api::schema::{
    PluginStorageDeleteParams, PluginStorageEntries, PluginStorageGetParams,
    PluginStorageListParams, PluginStorageSetParams, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

const STORAGE_FILE: &str = "storage.json";
const MAX_STORAGE_KEY_LEN: usize = 256;

type StorageDocument = BTreeMap<String, serde_json::Value>;

impl App {
    pub(in crate::app::api) fn handle_plugin_storage_get(
        &mut self,
        id: String,
        params: PluginStorageGetParams,
    ) -> String {
        let (plugin_id, key) = match normalize_storage_request(&id, params.plugin_id, params.key) {
            Ok(request) => request,
            Err(response) => return response,
        };
        if let Err(response) = self.ensure_plugin_storage_available(&id, &plugin_id) {
            return response;
        }
        let document = match read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let value = document.get(&key).cloned();
        encode_success(
            id,
            ResponseResult::PluginStorageValue {
                plugin_id,
                key,
                value,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_storage_set(
        &mut self,
        id: String,
        params: PluginStorageSetParams,
    ) -> String {
        let (plugin_id, key) = match normalize_storage_request(&id, params.plugin_id, params.key) {
            Ok(request) => request,
            Err(response) => return response,
        };
        if let Err(response) = self.ensure_plugin_storage_available(&id, &plugin_id) {
            return response;
        }
        let mut document = match read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        document.insert(key.clone(), params.value.clone());
        if let Err(response) = write_storage_document(&id, &plugin_id, &document) {
            return response;
        }
        encode_success(
            id,
            ResponseResult::PluginStorageSet {
                plugin_id,
                key,
                value: params.value,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_storage_delete(
        &mut self,
        id: String,
        params: PluginStorageDeleteParams,
    ) -> String {
        let (plugin_id, key) = match normalize_storage_request(&id, params.plugin_id, params.key) {
            Ok(request) => request,
            Err(response) => return response,
        };
        if let Err(response) = self.ensure_plugin_storage_available(&id, &plugin_id) {
            return response;
        }
        let mut document = match read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let existed = document.remove(&key).is_some();
        if let Err(response) = write_storage_document(&id, &plugin_id, &document) {
            return response;
        }
        encode_success(
            id,
            ResponseResult::PluginStorageDeleted {
                plugin_id,
                key,
                existed,
            },
        )
    }

    pub(in crate::app::api) fn handle_plugin_storage_list(
        &mut self,
        id: String,
        params: PluginStorageListParams,
    ) -> String {
        let plugin_id = match normalize_plugin_storage_id(&id, params.plugin_id) {
            Ok(plugin_id) => plugin_id,
            Err(response) => return response,
        };
        if let Err(response) = self.ensure_plugin_storage_available(&id, &plugin_id) {
            return response;
        }
        let document = match read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        let entries = document
            .into_iter()
            .filter(|(key, _)| {
                params
                    .prefix
                    .as_deref()
                    .is_none_or(|prefix| key.starts_with(prefix))
            })
            .collect::<PluginStorageEntries>();
        encode_success(id, ResponseResult::PluginStorageList { plugin_id, entries })
    }

    fn ensure_plugin_storage_available(&self, id: &str, plugin_id: &str) -> Result<(), String> {
        let Some(plugin) = self.state.installed_plugins.get(plugin_id) else {
            return Err(encode_error(
                id.to_string(),
                "plugin_not_found",
                format!("plugin {plugin_id} is not installed"),
            ));
        };
        super::env::ensure_plugin_user_dirs(plugin).map_err(|err| {
            encode_error(
                id.to_string(),
                "plugin_user_dir_create_failed",
                err.to_string(),
            )
        })
    }
}

fn normalize_storage_request(
    id: &str,
    plugin_id: String,
    key: String,
) -> Result<(String, String), String> {
    let plugin_id = normalize_plugin_storage_id(id, plugin_id)?;
    validate_storage_key(id, &key)?;
    Ok((plugin_id, key))
}

fn normalize_plugin_storage_id(id: &str, plugin_id: String) -> Result<String, String> {
    super::manifest::normalize_plugin_id(&plugin_id)
        .ok_or_else(|| invalid_storage_request(id, "invalid plugin id"))
}

fn validate_storage_key(id: &str, key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err(invalid_storage_request(
            id,
            "plugin storage key cannot be empty",
        ));
    }
    if key.chars().count() > MAX_STORAGE_KEY_LEN {
        return Err(invalid_storage_request(
            id,
            format!("plugin storage key cannot exceed {MAX_STORAGE_KEY_LEN} characters"),
        ));
    }
    if key.chars().any(char::is_control) {
        return Err(invalid_storage_request(
            id,
            "plugin storage key cannot contain control characters",
        ));
    }
    Ok(())
}

fn invalid_storage_request(id: &str, message: impl Into<String>) -> String {
    encode_error(id.to_string(), "invalid_plugin_storage_request", message)
}

fn storage_file(plugin_id: &str) -> PathBuf {
    super::env::plugin_state_dir(plugin_id).join(STORAGE_FILE)
}

fn read_storage_document(id: &str, plugin_id: &str) -> Result<StorageDocument, String> {
    let path = storage_file(plugin_id);
    if !path.exists() {
        return Ok(StorageDocument::new());
    }
    let bytes = std::fs::read(&path).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_read_failed",
            err.to_string(),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_decode_failed",
            err.to_string(),
        )
    })
}

fn write_storage_document(
    id: &str,
    plugin_id: &str,
    document: &StorageDocument,
) -> Result<(), String> {
    let path = storage_file(plugin_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            encode_error(
                id.to_string(),
                "plugin_storage_write_failed",
                err.to_string(),
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(document).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_encode_failed",
            err.to_string(),
        )
    })?;
    std::fs::write(path, bytes).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_write_failed",
            err.to_string(),
        )
    })
}

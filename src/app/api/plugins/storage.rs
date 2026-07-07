use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::api::schema::{
    PluginCapability, PluginStorageDeleteParams, PluginStorageEntries, PluginStorageGetParams,
    PluginStorageListParams, PluginStorageSetParams, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

const STORAGE_FILE: &str = "storage.json";
const MAX_STORAGE_KEY_LEN: usize = 256;
const MAX_STORAGE_KEYS: usize = 1024;
const MAX_STORAGE_VALUE_BYTES: usize = 256 * 1024;
const MAX_STORAGE_DOCUMENT_BYTES: usize = 1024 * 1024;

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
        if let Err(response) = validate_storage_value(&id, &params.value) {
            return response;
        }
        let mut document = match read_storage_document(&id, &plugin_id) {
            Ok(document) => document,
            Err(response) => return response,
        };
        if let Err(response) = validate_storage_entry_count(&id, &document, &key) {
            return response;
        }
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
        if !super::manifest::plugin_has_capability(plugin, PluginCapability::Storage) {
            return Err(encode_error(
                id.to_string(),
                "plugin_capability_required",
                "plugin storage requires capability 'storage'",
            ));
        }
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

fn validate_storage_value(id: &str, value: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_encode_failed",
            err.to_string(),
        )
    })?;
    if bytes.len() > MAX_STORAGE_VALUE_BYTES {
        return Err(encode_error(
            id.to_string(),
            "plugin_storage_value_too_large",
            format!("plugin storage values cannot exceed {MAX_STORAGE_VALUE_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn validate_storage_entry_count(
    id: &str,
    document: &StorageDocument,
    key: &str,
) -> Result<(), String> {
    if !document.contains_key(key) && document.len() >= MAX_STORAGE_KEYS {
        return Err(encode_error(
            id.to_string(),
            "plugin_storage_entry_limit_exceeded",
            format!("plugin storage cannot exceed {MAX_STORAGE_KEYS} keys"),
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
    if let Ok(metadata) = std::fs::metadata(&path) {
        if metadata.len() > MAX_STORAGE_DOCUMENT_BYTES as u64 {
            return Err(encode_error(
                id.to_string(),
                "plugin_storage_document_too_large",
                format!("plugin storage document cannot exceed {MAX_STORAGE_DOCUMENT_BYTES} bytes"),
            ));
        }
    }
    let bytes = std::fs::read(&path).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_read_failed",
            err.to_string(),
        )
    })?;
    if bytes.len() > MAX_STORAGE_DOCUMENT_BYTES {
        return Err(encode_error(
            id.to_string(),
            "plugin_storage_document_too_large",
            format!("plugin storage document cannot exceed {MAX_STORAGE_DOCUMENT_BYTES} bytes"),
        ));
    }
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
    let bytes = serde_json::to_vec_pretty(document).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_encode_failed",
            err.to_string(),
        )
    })?;
    if bytes.len() > MAX_STORAGE_DOCUMENT_BYTES {
        return Err(encode_error(
            id.to_string(),
            "plugin_storage_document_too_large",
            format!("plugin storage document cannot exceed {MAX_STORAGE_DOCUMENT_BYTES} bytes"),
        ));
    }
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
    std::fs::write(path, bytes).map_err(|err| {
        encode_error(
            id.to_string(),
            "plugin_storage_write_failed",
            err.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_code(response: String) -> String {
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        value["error"]["code"].as_str().unwrap().to_string()
    }

    #[test]
    fn storage_key_validation_rejects_invalid_keys() {
        assert_eq!(
            error_code(validate_storage_key("empty", "").unwrap_err()),
            "invalid_plugin_storage_request"
        );
        assert_eq!(
            error_code(validate_storage_key("control", "state/\nvalue").unwrap_err()),
            "invalid_plugin_storage_request"
        );
        let long_key = "k".repeat(MAX_STORAGE_KEY_LEN + 1);
        assert_eq!(
            error_code(validate_storage_key("long", &long_key).unwrap_err()),
            "invalid_plugin_storage_request"
        );
    }

    #[test]
    fn storage_value_validation_rejects_large_values() {
        let large = serde_json::Value::String("x".repeat(MAX_STORAGE_VALUE_BYTES + 1));
        assert_eq!(
            error_code(validate_storage_value("large-value", &large).unwrap_err()),
            "plugin_storage_value_too_large"
        );
    }

    #[test]
    fn storage_entry_count_rejects_new_keys_after_limit() {
        let mut document = StorageDocument::new();
        for index in 0..MAX_STORAGE_KEYS {
            document.insert(format!("state/{index}"), serde_json::json!(index));
        }

        assert_eq!(
            error_code(
                validate_storage_entry_count("too-many", &document, "state/new").unwrap_err()
            ),
            "plugin_storage_entry_limit_exceeded"
        );
        assert!(validate_storage_entry_count("existing", &document, "state/0").is_ok());
    }

    #[test]
    fn storage_document_write_rejects_large_documents() {
        let mut document = StorageDocument::new();
        document.insert(
            "state/value".into(),
            serde_json::Value::String("x".repeat(MAX_STORAGE_DOCUMENT_BYTES + 1)),
        );
        assert_eq!(
            error_code(
                write_storage_document("large-doc", "example.large-doc", &document).unwrap_err()
            ),
            "plugin_storage_document_too_large"
        );
    }
}

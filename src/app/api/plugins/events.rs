use crate::api::schema::{
    EventData, EventEnvelope, EventKind, PluginCapability, PluginEventEmitParams, ResponseResult,
};
use crate::app::api::responses::{encode_error, encode_success};
use crate::app::App;

const PLUGIN_EVENT_NAME_MAX_CHARS: usize = 120;

impl App {
    pub(in crate::app::api) fn handle_plugin_event_emit(
        &mut self,
        id: String,
        params: PluginEventEmitParams,
    ) -> String {
        let Some(plugin_id) = super::manifest::normalize_plugin_id(&params.plugin_id) else {
            return encode_error(
                id,
                "invalid_plugin_id",
                "plugin id must be non-empty, <= 120 characters, and contain only ASCII letters, digits, colon, dot, underscore, or hyphen",
            );
        };
        let event = match normalize_plugin_event_name(&params.event) {
            Ok(event) => event,
            Err((code, message)) => return encode_error(id, code, message),
        };
        let Some(plugin) = self.state.installed_plugins.get(&plugin_id) else {
            return encode_error(
                id,
                "plugin_not_found",
                format!("plugin {plugin_id} is not installed"),
            );
        };
        if !super::plugin_manifest_available(plugin) {
            return encode_error(
                id,
                "plugin_manifest_unavailable",
                format!("plugin {plugin_id} manifest is unavailable"),
            );
        }
        if !plugin.enabled {
            return encode_error(
                id,
                "plugin_disabled",
                format!("plugin {plugin_id} is disabled"),
            );
        }
        if !super::manifest::plugin_has_capability(plugin, PluginCapability::Events) {
            return encode_error(
                id,
                "plugin_capability_required",
                "plugin event emit requires capability 'events'",
            );
        }

        self.emit_event(EventEnvelope {
            event: EventKind::PluginEvent,
            data: EventData::PluginEvent {
                plugin_id,
                event,
                payload: params.payload,
            },
        });
        encode_success(id, ResponseResult::Ok {})
    }
}

fn normalize_plugin_event_name(value: &str) -> Result<String, (&'static str, String)> {
    let value = value.trim();
    if value.is_empty() {
        return Err((
            "invalid_plugin_event",
            "plugin event is required".to_string(),
        ));
    }
    if value.chars().count() > PLUGIN_EVENT_NAME_MAX_CHARS {
        return Err((
            "invalid_plugin_event",
            format!("plugin event must be {PLUGIN_EVENT_NAME_MAX_CHARS} characters or fewer"),
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-'))
    {
        return Err((
            "invalid_plugin_event",
            "plugin event can contain only ASCII letters, digits, colon, dot, underscore, or hyphen"
                .to_string(),
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_event_name_validation_rejects_invalid_names() {
        assert!(normalize_plugin_event_name("resource.updated").is_ok());
        assert!(normalize_plugin_event_name("").is_err());
        assert!(normalize_plugin_event_name("resource/updated").is_err());
        assert!(normalize_plugin_event_name(&"e".repeat(PLUGIN_EVENT_NAME_MAX_CHARS + 1)).is_err());
    }
}

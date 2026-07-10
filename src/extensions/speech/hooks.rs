use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientSpeechHook {
    StartDictation,
    StopDictation,
    TransformTranscript,
}

impl ClientSpeechHook {
    fn action_id(self, plugin: &crate::api::schema::InstalledPluginInfo) -> Option<&str> {
        match self {
            Self::StartDictation => plugin.client_speech.start_dictation.as_deref(),
            Self::StopDictation => plugin.client_speech.stop_dictation.as_deref(),
            Self::TransformTranscript => plugin.client_speech.transform_transcript.as_deref(),
        }
    }

    fn invocation_source(self) -> &'static str {
        match self {
            Self::StartDictation => "client_speech.start_dictation",
            Self::StopDictation => "client_speech.stop_dictation",
            Self::TransformTranscript => "client_speech.transform_transcript",
        }
    }

    fn payload_hook(self) -> &'static str {
        match self {
            Self::StartDictation => "start_dictation",
            Self::StopDictation => "stop_dictation",
            Self::TransformTranscript => "transform_transcript",
        }
    }
}

const CLIENT_SPEECH_TRANSFORM_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1500);

pub(crate) fn emit_start_dictation(app: &mut App, workspace_id: &str, is_agent: bool) {
    emit_hook(
        app,
        ClientSpeechHook::StartDictation,
        serde_json::json!({
            "hook": ClientSpeechHook::StartDictation.payload_hook(),
            "phase": "start",
            "workspace_id": workspace_id,
            "is_agent": is_agent,
        }),
    );
}

pub(crate) fn emit_stop_dictation(app: &mut App, abort: bool) {
    emit_hook(
        app,
        ClientSpeechHook::StopDictation,
        serde_json::json!({
            "hook": ClientSpeechHook::StopDictation.payload_hook(),
            "phase": "stop",
            "abort": abort,
        }),
    );
}

pub(crate) fn transform_transcript(
    app: &mut App,
    workspace_id: &str,
    transcript: &str,
    is_agent: bool,
) -> String {
    let mut current = transcript.to_string();
    let targets = client_speech_hook_targets(app, ClientSpeechHook::TransformTranscript);
    for (plugin_id, action_id) in targets {
        let payload = serde_json::json!({
            "hook": ClientSpeechHook::TransformTranscript.payload_hook(),
            "phase": "transform",
            "workspace_id": workspace_id,
            "is_agent": is_agent,
            "transcript": current,
        });
        let result = match app.call_plugin_action_internal(
            &plugin_id,
            &action_id,
            ClientSpeechHook::TransformTranscript.invocation_source(),
            payload,
            CLIENT_SPEECH_TRANSFORM_TIMEOUT,
        ) {
            Ok(result) => result,
            Err(err) => {
                tracing::warn!(
                    plugin_id,
                    action_id,
                    error = %err,
                    "failed to invoke client speech transform hook"
                );
                continue;
            }
        };
        if result.status != crate::api::schema::PluginCommandStatus::Succeeded {
            tracing::warn!(
                plugin_id,
                action_id,
                timed_out = result.timed_out,
                exit_code = ?result.exit_code,
                error = ?result.error,
                stderr = %result.stderr,
                "client speech transform hook failed"
            );
            continue;
        }
        if let Some(next) = transcript_from_plugin_stdout(&result.stdout) {
            current = next;
        }
    }
    current
}

fn emit_hook(app: &mut App, hook: ClientSpeechHook, payload: serde_json::Value) {
    let targets = client_speech_hook_targets(app, hook);
    for (plugin_id, action_id) in targets {
        if let Err(err) = app.invoke_plugin_action_internal(
            &plugin_id,
            &action_id,
            hook.invocation_source(),
            payload.clone(),
        ) {
            tracing::warn!(
                plugin_id,
                action_id,
                error = %err,
                "failed to invoke client speech plugin hook"
            );
        }
    }
}

fn client_speech_hook_targets(app: &App, hook: ClientSpeechHook) -> Vec<(String, String)> {
    let mut targets = app
        .state
        .installed_plugins
        .values()
        .filter(|plugin| plugin.enabled)
        .filter_map(|plugin| {
            hook.action_id(plugin)
                .map(|action_id| (plugin.plugin_id.clone(), action_id.to_string()))
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

fn transcript_from_plugin_stdout(stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(transcript) = value.as_str() {
            return Some(transcript.to_string());
        }
        if let Some(transcript) = value.get("transcript").and_then(|value| value.as_str()) {
            return Some(transcript.to_string());
        }
        if let Some(transcript) = value.get("text").and_then(|value| value.as_str()) {
            return Some(transcript.to_string());
        }
        return None;
    }
    Some(stdout.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{
        Method, PluginCommandStatus, PluginLinkParams, Request, SuccessResponse,
    };
    use crate::config::Config;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    #[test]
    fn client_speech_hooks_invoke_declared_plugin_actions() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("client-speech-hooks");
        let xdg_home = unique_temp_path("client-speech-hooks-xdg");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_manifest_content(
            &root,
            r#"
id = "example.client-speech"
name = "Client Speech"
version = "0.1.0"
api_version = 2
capabilities = ["actions", "client-speech"]
min_herdr_version = "0.6.10"
platforms = ["linux", "macos"]

[[actions]]
id = "start"
title = "Start"
command = ["sh", "-c", "printf '%s' \"$HERDR_PLUGIN_PAYLOAD_JSON\""]

[[actions]]
id = "stop"
title = "Stop"
command = ["sh", "-c", "printf '%s' \"$HERDR_PLUGIN_PAYLOAD_JSON\""]

[client_speech]
start_dictation = "start"
stop_dictation = "stop"
"#,
        );
        let link = app.handle_api_request(Request {
            id: "link".into(),
            method: Method::PluginLink(PluginLinkParams {
                path: root.display().to_string(),
                enabled: true,
                source: None,
            }),
        });
        serde_json::from_str::<SuccessResponse>(&link).expect("plugin should link");

        emit_start_dictation(&mut app, "w1", true);
        emit_stop_dictation(&mut app, true);
        wait_for_plugin_commands(&mut app, 2);

        let payloads = app
            .state
            .plugin_command_logs
            .iter()
            .map(|log| {
                assert_eq!(log.plugin_id, "example.client-speech");
                assert_eq!(log.status, PluginCommandStatus::Succeeded);
                serde_json::from_str::<serde_json::Value>(log.stdout.as_deref().unwrap_or(""))
                    .expect("stdout should be payload JSON")
            })
            .collect::<Vec<_>>();
        assert!(payloads.iter().any(|payload| {
            payload["hook"] == "start_dictation"
                && payload["phase"] == "start"
                && payload["workspace_id"] == "w1"
                && payload["is_agent"] == true
        }));
        assert!(payloads.iter().any(|payload| {
            payload["hook"] == "stop_dictation"
                && payload["phase"] == "stop"
                && payload["abort"] == true
        }));

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        restore_xdg_home(old_config_home, old_state_home);
    }

    #[cfg(unix)]
    #[test]
    fn client_speech_transform_transcript_uses_plugin_output() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let root = unique_temp_path("client-speech-transform");
        let xdg_home = unique_temp_path("client-speech-transform-xdg");
        let capture = root.join("payload.json");
        let old_config_home = std::env::var_os("XDG_CONFIG_HOME");
        let old_state_home = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg_home);
        std::env::set_var("XDG_STATE_HOME", &xdg_home);
        write_manifest_content(
            &root,
            &format!(
                r#"
id = "example.client-speech-transform"
name = "Client Speech Transform"
version = "0.1.0"
api_version = 2
capabilities = ["actions", "client-speech"]
min_herdr_version = "0.6.10"
platforms = ["linux", "macos"]

[[actions]]
id = "transform"
title = "Transform"
command = ["sh", "-c", "printf '%s' \"$HERDR_PLUGIN_PAYLOAD_JSON\" > {}; printf '{{\"transcript\":\"plugin text\"}}'"]

[client_speech]
transform_transcript = "transform"
"#,
                capture.display()
            ),
        );
        let link = app.handle_api_request(Request {
            id: "link".into(),
            method: Method::PluginLink(PluginLinkParams {
                path: root.display().to_string(),
                enabled: true,
                source: None,
            }),
        });
        serde_json::from_str::<SuccessResponse>(&link).expect("plugin should link");

        let transformed = transform_transcript(&mut app, "w1", "raw text", true);

        assert_eq!(transformed, "plugin text");
        let payload: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&capture).unwrap()).unwrap();
        assert_eq!(payload["hook"], "transform_transcript");
        assert_eq!(payload["phase"], "transform");
        assert_eq!(payload["workspace_id"], "w1");
        assert_eq!(payload["is_agent"], true);
        assert_eq!(payload["transcript"], "raw text");

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(xdg_home);
        restore_xdg_home(old_config_home, old_state_home);
    }

    #[cfg(unix)]
    fn wait_for_plugin_commands(app: &mut App, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            app.drain_all_internal_events();
            let finished = app
                .state
                .plugin_command_logs
                .iter()
                .filter(|log| log.status != PluginCommandStatus::Running)
                .count();
            if finished >= count {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for plugin commands");
    }

    fn unique_temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("herdr-test-{label}-{nanos}"))
    }

    fn write_manifest_content(root: &std::path::Path, content: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("herdr-plugin.toml"), content).unwrap();
    }

    fn restore_xdg_home(
        old_config_home: Option<std::ffi::OsString>,
        old_state_home: Option<std::ffi::OsString>,
    ) {
        match old_config_home {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_state_home {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
    }
}

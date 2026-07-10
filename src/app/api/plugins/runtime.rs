use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use super::manifest::{effective_platforms, ensure_platform_supported};
use super::plugin_manifest_available;
use crate::api::schema::{
    InstalledPluginInfo, PluginCommandLogInfo, PluginCommandStatus, PluginInvocationContext,
};
use crate::app::App;

const PLUGIN_COMMAND_OUTPUT_MAX_BYTES: usize = 64 * 1024;
pub(super) const PLUGIN_ACTION_PAYLOAD_MAX_BYTES: usize = 256 * 1024;
pub(super) const MAX_PLUGIN_COMMANDS_IN_FLIGHT: usize = 32;
const PLUGIN_COMMAND_LOG_LIMIT: usize = 200;
const PLUGIN_COMMAND_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginActionCallResult {
    pub status: PluginCommandStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
    pub timed_out: bool,
}

struct PreparedPluginCommand {
    program: String,
    args: Vec<String>,
    plugin_root: PathBuf,
    env: Vec<(String, String)>,
}

impl App {
    pub(super) fn start_plugin_command(
        &mut self,
        plugin: &InstalledPluginInfo,
        action_id: Option<String>,
        event: Option<String>,
        command: Vec<String>,
        context: &PluginInvocationContext,
        payload: Option<serde_json::Value>,
        event_json: Option<String>,
    ) -> Result<PluginCommandLogInfo, (&'static str, String)> {
        let prepared = prepare_plugin_command(
            plugin,
            action_id.as_deref(),
            event.as_deref(),
            &command,
            context,
            payload,
            event_json.as_deref(),
        )?;
        let log_id = format!("plugin-log-{}", self.state.next_plugin_command_log_id);
        self.state.next_plugin_command_log_id += 1;
        let started_unix_ms = current_unix_ms();
        let correlation_id = context.correlation_id.clone();
        if self.state.plugin_commands_in_flight >= MAX_PLUGIN_COMMANDS_IN_FLIGHT {
            let message = format!(
                "maximum concurrent plugin commands reached ({MAX_PLUGIN_COMMANDS_IN_FLIGHT})"
            );
            let log = PluginCommandLogInfo {
                log_id,
                plugin_id: plugin.plugin_id.clone(),
                action_id,
                event,
                correlation_id,
                command,
                status: PluginCommandStatus::Failed,
                started_unix_ms,
                finished_unix_ms: Some(started_unix_ms),
                exit_code: None,
                stdout: Some(String::new()),
                stderr: Some(String::new()),
                error: Some(message.clone()),
            };
            self.push_plugin_command_log(log);
            return Err(("plugin_command_limit_reached", message));
        }
        let log = PluginCommandLogInfo {
            log_id: log_id.clone(),
            plugin_id: plugin.plugin_id.clone(),
            action_id,
            event,
            correlation_id,
            command: command.clone(),
            status: PluginCommandStatus::Running,
            started_unix_ms,
            finished_unix_ms: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            error: None,
        };
        self.push_plugin_command_log(log.clone());
        self.state.plugin_commands_in_flight += 1;
        let event_tx = self.event_tx.clone();
        std::thread::spawn(move || {
            let child = crate::plugin_command::command_for_argv(&prepared.program, &prepared.args)
                .current_dir(prepared.plugin_root)
                .envs(prepared.env)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let finished = match child {
                Ok(mut child) => {
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let stdout_reader = stdout.map(|stdout| {
                        std::thread::spawn(move || {
                            read_capped_plugin_output(stdout, PLUGIN_COMMAND_OUTPUT_MAX_BYTES)
                        })
                    });
                    let stderr_reader = stderr.map(|stderr| {
                        std::thread::spawn(move || {
                            read_capped_plugin_output(stderr, PLUGIN_COMMAND_OUTPUT_MAX_BYTES)
                        })
                    });
                    match child.wait() {
                        Ok(status) => crate::events::AppEvent::PluginCommandFinished {
                            log_id,
                            finished_unix_ms: current_unix_ms(),
                            exit_code: status.code(),
                            stdout: stdout_reader
                                .and_then(|reader| reader.join().ok())
                                .unwrap_or_default(),
                            stderr: stderr_reader
                                .and_then(|reader| reader.join().ok())
                                .unwrap_or_default(),
                            error: None,
                        },
                        Err(err) => crate::events::AppEvent::PluginCommandFinished {
                            log_id,
                            finished_unix_ms: current_unix_ms(),
                            exit_code: None,
                            stdout: stdout_reader
                                .and_then(|reader| reader.join().ok())
                                .unwrap_or_default(),
                            stderr: stderr_reader
                                .and_then(|reader| reader.join().ok())
                                .unwrap_or_default(),
                            error: Some(err.to_string()),
                        },
                    }
                }
                Err(err) => crate::events::AppEvent::PluginCommandFinished {
                    log_id,
                    finished_unix_ms: current_unix_ms(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: Some(err.to_string()),
                },
            };
            let _ = event_tx.blocking_send(finished);
        });
        Ok(log)
    }

    pub(crate) fn call_plugin_command(
        &mut self,
        plugin: &InstalledPluginInfo,
        action_id: Option<&str>,
        command: &[String],
        context: &PluginInvocationContext,
        payload: Option<serde_json::Value>,
        timeout: Duration,
    ) -> Result<PluginActionCallResult, (&'static str, String)> {
        if self.state.plugin_commands_in_flight >= MAX_PLUGIN_COMMANDS_IN_FLIGHT {
            return Err((
                "plugin_command_limit_reached",
                format!(
                    "maximum concurrent plugin commands reached ({MAX_PLUGIN_COMMANDS_IN_FLIGHT})"
                ),
            ));
        }
        let prepared =
            prepare_plugin_command(plugin, action_id, None, command, context, payload, None)?;
        let mut child = crate::plugin_command::command_for_argv(&prepared.program, &prepared.args)
            .current_dir(prepared.plugin_root)
            .envs(prepared.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| ("plugin_command_spawn_failed", err.to_string()))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_reader = stdout.map(|stdout| {
            std::thread::spawn(move || {
                read_capped_plugin_output(stdout, PLUGIN_COMMAND_OUTPUT_MAX_BYTES)
            })
        });
        let stderr_reader = stderr.map(|stderr| {
            std::thread::spawn(move || {
                read_capped_plugin_output(stderr, PLUGIN_COMMAND_OUTPUT_MAX_BYTES)
            })
        });

        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let wait_result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() >= deadline => {
                    timed_out = true;
                    if let Err(err) = child.kill() {
                        match child.wait() {
                            Ok(status) => break Ok(status),
                            Err(_) => break Err(err),
                        }
                    }
                    break child.wait();
                }
                Ok(None) => std::thread::sleep(PLUGIN_COMMAND_WAIT_POLL_INTERVAL),
                Err(err) => break Err(err),
            }
        };

        let stdout = stdout_reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        let stderr = stderr_reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        match wait_result {
            Ok(status) if timed_out => Ok(PluginActionCallResult {
                status: PluginCommandStatus::Failed,
                exit_code: status.code(),
                stdout,
                stderr,
                error: Some(format!(
                    "plugin action timed out after {}ms",
                    timeout.as_millis()
                )),
                timed_out,
            }),
            Ok(status) => Ok(PluginActionCallResult {
                status: if status.success() {
                    PluginCommandStatus::Succeeded
                } else {
                    PluginCommandStatus::Failed
                },
                exit_code: status.code(),
                stdout,
                stderr,
                error: None,
                timed_out,
            }),
            Err(err) => Ok(PluginActionCallResult {
                status: PluginCommandStatus::Failed,
                exit_code: None,
                stdout,
                stderr,
                error: Some(err.to_string()),
                timed_out,
            }),
        }
    }

    pub(crate) fn run_plugin_event_hooks(&mut self, event: &crate::api::schema::EventEnvelope) {
        let event_name = event.event.dot_name();
        if !crate::api::schema::PLUGIN_HOOK_EVENT_KINDS.contains(&event.event) {
            return;
        }
        let plugins = self
            .state
            .installed_plugins
            .values()
            .filter(|plugin| {
                plugin.enabled
                    && plugin_manifest_available(plugin)
                    && plugin.events.iter().any(|hook| hook.on == event_name)
            })
            .cloned()
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            return;
        }
        let event_json = serde_json::to_string(event).ok();
        let context = self.plugin_context_for_event(event, event_name);
        for plugin in plugins {
            for hook in plugin.events.clone() {
                if hook.on != event_name {
                    continue;
                }
                if ensure_platform_supported(
                    &effective_platforms(&hook.platforms, &plugin.platforms).clone(),
                    event_name,
                )
                .is_err()
                {
                    continue;
                }
                let _ = self.start_plugin_command(
                    &plugin,
                    None,
                    Some(event_name.to_string()),
                    hook.command.clone(),
                    &context,
                    None,
                    event_json.clone(),
                );
            }
        }
    }

    fn push_plugin_command_log(&mut self, log: PluginCommandLogInfo) {
        self.state.plugin_command_logs.push(log);
        if self.state.plugin_command_logs.len() > PLUGIN_COMMAND_LOG_LIMIT {
            let extra = self.state.plugin_command_logs.len() - PLUGIN_COMMAND_LOG_LIMIT;
            self.state.plugin_command_logs.drain(0..extra);
        }
    }
}

fn prepare_plugin_command(
    plugin: &InstalledPluginInfo,
    action_id: Option<&str>,
    event: Option<&str>,
    command: &[String],
    context: &PluginInvocationContext,
    payload: Option<serde_json::Value>,
    event_json: Option<&str>,
) -> Result<PreparedPluginCommand, (&'static str, String)> {
    let Some(program) = command.first().cloned() else {
        return Err((
            "invalid_plugin_command",
            "command must not be empty".to_string(),
        ));
    };
    let args = command.iter().skip(1).cloned().collect::<Vec<_>>();
    let context_json = serde_json::to_string(context)
        .map_err(|err| ("invalid_plugin_context", err.to_string()))?;
    let payload_json = match payload {
        Some(payload) => {
            let json = serde_json::to_string(&payload)
                .map_err(|err| ("invalid_plugin_payload", err.to_string()))?;
            if json.len() > PLUGIN_ACTION_PAYLOAD_MAX_BYTES {
                return Err((
                    "plugin_payload_too_large",
                    format!(
                        "plugin action payload exceeds {PLUGIN_ACTION_PAYLOAD_MAX_BYTES} bytes"
                    ),
                ));
            }
            Some(json)
        }
        None => None,
    };
    super::env::ensure_plugin_user_dirs(plugin)
        .map_err(|err| ("plugin_user_dir_create_failed", err.to_string()))?;
    let mut env = super::env::plugin_path_env(plugin);
    env.extend([
        (
            crate::api::SOCKET_PATH_ENV_VAR.to_string(),
            crate::api::socket_path().display().to_string(),
        ),
        ("HERDR_ENV".to_string(), "1".to_string()),
        ("HERDR_PLUGIN_ID".to_string(), plugin.plugin_id.clone()),
        ("HERDR_PLUGIN_CONTEXT_JSON".to_string(), context_json),
    ]);
    if let Some(correlation_id) = context.correlation_id.as_ref() {
        env.push((
            "HERDR_PLUGIN_CORRELATION_ID".to_string(),
            correlation_id.clone(),
        ));
    }
    if let Ok(current_exe) = std::env::current_exe() {
        env.push((
            "HERDR_BIN_PATH".to_string(),
            current_exe.display().to_string(),
        ));
    }
    if let Some(action_id) = action_id {
        env.push(("HERDR_PLUGIN_ACTION_ID".to_string(), action_id.to_string()));
    }
    if let Some(event) = event {
        env.push(("HERDR_PLUGIN_EVENT".to_string(), event.to_string()));
    }
    if let Some(event_json) = event_json {
        env.push((
            "HERDR_PLUGIN_EVENT_JSON".to_string(),
            event_json.to_string(),
        ));
    }
    if let Some(payload_json) = payload_json {
        env.push(("HERDR_PLUGIN_PAYLOAD_JSON".to_string(), payload_json));
    }
    if let Some(workspace_id) = context.workspace_id.as_ref() {
        env.push(("HERDR_WORKSPACE_ID".to_string(), workspace_id.clone()));
    }
    if let Some(tab_id) = context.tab_id.as_ref() {
        env.push(("HERDR_TAB_ID".to_string(), tab_id.clone()));
    }
    if let Some(pane_id) = context.focused_pane_id.as_ref() {
        env.push(("HERDR_PANE_ID".to_string(), pane_id.clone()));
    }
    if let Some(clicked_url) = context.clicked_url.as_ref() {
        env.push(("HERDR_PLUGIN_CLICKED_URL".to_string(), clicked_url.clone()));
    }
    if let Some(link_handler_id) = context.link_handler_id.as_ref() {
        env.push((
            "HERDR_PLUGIN_LINK_HANDLER_ID".to_string(),
            link_handler_id.clone(),
        ));
    }
    Ok(PreparedPluginCommand {
        program,
        args,
        plugin_root: PathBuf::from(&plugin.plugin_root),
        env,
    })
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub(super) fn read_capped_plugin_output(mut reader: impl Read, cap: usize) -> String {
    let mut kept = Vec::with_capacity(cap.min(8192));
    let mut buf = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let remaining = cap.saturating_sub(kept.len());
                if remaining > 0 {
                    kept.extend_from_slice(&buf[..n.min(remaining)]);
                }
                if n > remaining {
                    truncated = true;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    let mut output = String::from_utf8_lossy(&kept).into_owned();
    if truncated {
        output.push_str(&format!(
            "\n[herdr truncated plugin output after {cap} bytes]"
        ));
    }
    output
}

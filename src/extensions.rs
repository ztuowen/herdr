use crate::api::schema::Method;
use crate::events::AppEvent;
use crate::input::TerminalKey;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct ExtensionsState {
    pub static_image_placements: std::sync::Mutex<Vec<crate::app::state::StaticImagePlacement>>,

    pub speech_to_text: crate::config::SpeechToTextConfig,
    pub recording_workspace: Option<String>,
    pub live_transcription: Option<String>,
    pub kanban: crate::kanban::KanbanState,
}

impl ExtensionsState {
    pub fn new(
        speech_to_text: crate::config::SpeechToTextConfig,
        kanban_items: Vec<crate::api::schema::KanbanItem>,
    ) -> Self {
        Self {
            static_image_placements: std::sync::Mutex::new(Vec::new()),
            speech_to_text,
            recording_workspace: None,
            live_transcription: None,
            kanban: crate::kanban::KanbanState::new(kanban_items),
        }
    }
}

pub struct ExtensionsRuntime {
    pub speech_recorder: crate::speech::SpeechRecorder,
}

impl ExtensionsRuntime {
    pub fn new() -> Self {
        Self {
            speech_recorder: crate::speech::SpeechRecorder::new(),
        }
    }
}

pub fn sanitize_transcription_text(text: &str) -> String {
    let mut result = String::new();
    let mut last_was_space = false;

    for c in text.chars() {
        if c.is_control() {
            continue;
        }

        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}

pub fn handle_extension_event(app: &mut crate::app::App, ev: &AppEvent) -> bool {
    match ev {
        AppEvent::SpeechPartialTranscription { workspace_id, text } => {
            if app.state.extensions.recording_workspace.as_ref() == Some(workspace_id) {
                let sanitized = sanitize_transcription_text(text);
                app.state.extensions.live_transcription = Some(sanitized);
                app.render_dirty
                    .store(true, std::sync::atomic::Ordering::Release);
                app.render_notify.notify_one();
            }
            true
        }
        AppEvent::SpeechRawTranscribed {
            workspace_id,
            result,
        } => {
            let previous_toast = app.state.toast.clone();
            match result {
                Ok(raw_text) => {
                    let mut pane_found = false;
                    if let Some(ws_idx) = app
                        .state
                        .workspaces
                        .iter()
                        .position(|ws| ws.id == *workspace_id)
                    {
                        if let Some(ws) = app.state.workspaces.get(ws_idx) {
                            if let Some(pane_id) = ws.focused_pane_id() {
                                pane_found = true;
                                let is_agent = if let Some(pane) = ws.pane_state(pane_id) {
                                    let term_id = pane.attached_terminal_id.clone();
                                    if let Some(term) = app.state.terminals.get(&term_id) {
                                        term.is_agent_terminal()
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };

                                let system_instruction = if is_agent {
                                    app.state.extensions.speech_to_text.agent_system_instruction.clone()
                                        .or_else(|| app.state.extensions.speech_to_text.system_instruction.clone())
                                        .unwrap_or_else(|| "You are a post-processing engine for speech-to-text. The user is speaking to an AI coding assistant. Clean up the raw transcription to make it clear, coherent, and grammatically correct. Keep the natural phrasing but remove filler words (like 'um', 'uh', 'like') and correct homophones or mistranscribed words. Output only the corrected text without any chat or explanation.".to_string())
                                } else {
                                    app.state.extensions.speech_to_text.terminal_system_instruction.clone()
                                        .or_else(|| app.state.extensions.speech_to_text.system_instruction.clone())
                                        .unwrap_or_else(|| "You are a post-processing engine for speech-to-text. The user is speaking to a command-line terminal. Convert the raw transcription into the most likely shell command or command-line input. Correct spacing, casing, punctuation, and spelling errors for commands, flags, and paths. Output only the corrected terminal input without any chat or explanation.".to_string())
                                };

                                app.state.toast = Some(crate::app::state::ToastNotification {
                                    kind: crate::app::state::ToastKind::Finished,
                                    title: "Speech to Text".into(),
                                    context: "Refining...".into(),
                                    target: None,
                                });

                                let event_tx = app.event_tx.clone();
                                let api_key = match &app
                                    .state
                                    .extensions
                                    .speech_to_text
                                    .gemini_api_key
                                {
                                    Some(k) if !k.trim().is_empty() => k.clone(),
                                    _ => {
                                        let _ = event_tx.try_send(AppEvent::SpeechTranscribed {
                                            workspace_id: workspace_id.clone(),
                                            pane_id: Some(pane_id),
                                            result: Err(
                                                "No Gemini API key configured for post-processing."
                                                    .to_string(),
                                            ),
                                        });
                                        return true;
                                    }
                                };

                                let workspace_id_clone = workspace_id.clone();
                                let raw_text_clone = raw_text.clone();
                                std::thread::spawn(move || {
                                    let postprocess_result =
                                        crate::speech::run_gemini_postprocess(
                                            api_key,
                                            raw_text_clone,
                                            system_instruction,
                                        );
                                    let _ =
                                        event_tx.blocking_send(AppEvent::SpeechTranscribed {
                                            workspace_id: workspace_id_clone,
                                            pane_id: Some(pane_id),
                                            result: postprocess_result,
                                        });
                                });
                            }
                        }
                    }

                    if !pane_found {
                        let event_tx = app.event_tx.clone();
                        let _ = event_tx.try_send(AppEvent::SpeechTranscribed {
                            workspace_id: workspace_id.clone(),
                            pane_id: None,
                            result: Ok(raw_text.clone()),
                        });
                    }
                }
                Err(err) => {
                    app.state.extensions.live_transcription = None;
                    app.state.extensions.recording_workspace = None;
                    app.state.toast = Some(crate::app::state::ToastNotification {
                        kind: crate::app::state::ToastKind::NeedsAttention,
                        title: "Speech to Text Error".into(),
                        context: err.clone(),
                        target: None,
                    });
                }
            }
            app.sync_toast_deadline(previous_toast);
            app.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            app.render_notify.notify_one();
            true
        }
        AppEvent::SpeechTranscribed {
            workspace_id,
            pane_id,
            result,
        } => {
            app.state.extensions.live_transcription = None;
            app.state.extensions.recording_workspace = None;
            let previous_toast = app.state.toast.clone();
            match result {
                Ok(transcription) => {
                    let sanitized = sanitize_transcription_text(transcription);
                    app.state.toast = Some(crate::app::state::ToastNotification {
                        kind: crate::app::state::ToastKind::Finished,
                        title: "Speech to Text".into(),
                        context: sanitized.clone(),
                        target: None,
                    });
                    if let Some(ws_idx) = app
                        .state
                        .workspaces
                        .iter()
                        .position(|ws| ws.id == *workspace_id)
                    {
                        if let Some(ws) = app.state.workspaces.get(ws_idx) {
                            let target_pane_id = pane_id.or_else(|| ws.focused_pane_id());
                            if let Some(focused_pane_id) = target_pane_id {
                                if let Some(runtime) =
                                    app.lookup_runtime_sender(ws_idx, focused_pane_id)
                                {
                                    let bracketed = runtime
                                        .input_state()
                                        .map(|s| s.bracketed_paste)
                                        .unwrap_or(false);
                                    let payload = if bracketed {
                                        format!("\x1b[200~{sanitized}\x1b[201~")
                                    } else {
                                        sanitized.clone()
                                    };
                                    tracing::info!(
                                        "Speech to text: sending transcription to workspace={}, pane={:?}, bracketed={}, text={:?}",
                                        workspace_id,
                                        focused_pane_id,
                                        bracketed,
                                        sanitized
                                    );
                                    if let Err(e) =
                                        runtime.try_send_bytes(bytes::Bytes::from(payload))
                                    {
                                        tracing::error!("Speech to text: failed to write transcription to PTY: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    app.state.toast = Some(crate::app::state::ToastNotification {
                        kind: crate::app::state::ToastKind::NeedsAttention,
                        title: "Speech to Text Error".into(),
                        context: err.clone(),
                        target: None,
                    });
                }
            }
            app.sync_toast_deadline(previous_toast);
            app.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            app.render_notify.notify_one();
            true
        }
        _ => false,
    }
}

pub fn handle_extension_api_request(
    app: &mut crate::app::App,
    request_id: String,
    method: &Method,
) -> Option<String> {
    match method {
        Method::KanbanAdd(params) => Some(app.handle_kanban_add(request_id, params.clone())),
        Method::KanbanList(params) => Some(app.handle_kanban_list(request_id, params.clone())),
        Method::KanbanUpdate(params) => Some(app.handle_kanban_update(request_id, params.clone())),
        Method::KanbanDelete(params) => Some(app.handle_kanban_delete(request_id, params.clone())),
        _ => None,
    }
}

pub fn handle_extension_key(app: &mut crate::app::App, key: TerminalKey) -> bool {
    if app.state.mode == crate::app::Mode::Kanban {
        crate::app::input::handle_kanban_key(&mut app.state, key);
        return true;
    }

    if app.handle_speech_to_text_key(key) {
        return true;
    }

    false
}

pub fn render_extension_ui(
    app: &crate::app::AppState,
    frame: &mut Frame,
    terminal_area: Rect,
) -> bool {
    if app.mode == crate::app::Mode::Kanban {
        crate::ui::kanban::render_kanban(app, frame, terminal_area);
        return true;
    }
    false
}

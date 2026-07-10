use crate::events::AppEvent;
use crate::extensions::PendingEnter;

fn sanitize_transcription_text(text: &str) -> String {
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

pub(crate) fn handle_speech_event(app: &mut crate::app::App, ev: &AppEvent) -> bool {
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
                                    position: None,
                                    target: None,
                                });

                                let event_tx = app.event_tx.clone();
                                let api_key =
                                    match &app.state.extensions.speech_to_text.gemini_api_key {
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
                                        crate::extensions::speech::run_gemini_postprocess(
                                            api_key,
                                            raw_text_clone,
                                            system_instruction,
                                        );
                                    let _ = event_tx.blocking_send(AppEvent::SpeechTranscribed {
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
                        position: None,
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
                    let is_agent = speech_target_is_agent(app, workspace_id, *pane_id);
                    let transcription = crate::extensions::speech::hooks::transform_transcript(
                        app,
                        workspace_id,
                        transcription,
                        is_agent,
                    );
                    let sanitized = sanitize_transcription_text(&transcription);
                    app.state.toast = Some(crate::app::state::ToastNotification {
                        kind: crate::app::state::ToastKind::Finished,
                        title: "Speech to Text".into(),
                        context: sanitized.clone(),
                        position: None,
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
                                let submit_to_agent = ws
                                    .pane_state(focused_pane_id)
                                    .and_then(|pane| {
                                        app.state.terminals.get(&pane.attached_terminal_id)
                                    })
                                    .is_some_and(|terminal| terminal.is_agent_terminal());
                                if let Some(runtime) =
                                    app.lookup_runtime_sender(ws_idx, focused_pane_id)
                                {
                                    let bracketed = runtime
                                        .input_state()
                                        .map(|s| s.bracketed_paste)
                                        .unwrap_or(false);
                                    let payload = if bracketed {
                                        format!("\x1b[200~{sanitized}\x1b[201~").into_bytes()
                                    } else {
                                        sanitized.as_bytes().to_vec()
                                    };
                                    tracing::info!(
                                        "Speech to text: sending transcription to workspace={}, pane={:?}, bracketed={}, submit_to_agent={}, text={:?}",
                                        workspace_id,
                                        focused_pane_id,
                                        bracketed,
                                        submit_to_agent,
                                        sanitized
                                    );
                                    if let Err(e) =
                                        runtime.try_send_bytes(bytes::Bytes::from(payload))
                                    {
                                        tracing::error!("Speech to text: failed to write transcription to PTY: {:?}", e);
                                    }

                                    if submit_to_agent {
                                        app.state.extensions.pending_enter_sequence += 1;
                                        let current_seq =
                                            app.state.extensions.pending_enter_sequence;
                                        app.state.extensions.pending_enter = Some(PendingEnter {
                                            workspace_id: workspace_id.clone(),
                                            pane_id: focused_pane_id,
                                            sequence: current_seq,
                                        });

                                        let event_tx = app.event_tx.clone();
                                        let ws_id = workspace_id.clone();
                                        let p_id = focused_pane_id;
                                        std::thread::spawn(move || {
                                            std::thread::sleep(std::time::Duration::from_secs(1));
                                            let _ = event_tx.blocking_send(
                                                AppEvent::SpeechSubmitEnter {
                                                    workspace_id: ws_id,
                                                    pane_id: p_id,
                                                    sequence: current_seq,
                                                },
                                            );
                                        });
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
                        position: None,
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
        AppEvent::SpeechSubmitEnter {
            workspace_id,
            pane_id,
            sequence,
        } => {
            let matches = if let Some(pending) = &app.state.extensions.pending_enter {
                pending.workspace_id == *workspace_id
                    && pending.pane_id == *pane_id
                    && pending.sequence == *sequence
            } else {
                false
            };

            if matches {
                app.state.extensions.pending_enter = None;
                if let Some(ws_idx) = app
                    .state
                    .workspaces
                    .iter()
                    .position(|ws| ws.id == *workspace_id)
                {
                    if let Some(runtime) = app.lookup_runtime_sender(ws_idx, *pane_id) {
                        let enter_bytes = runtime.encode_terminal_key(
                            crossterm::event::KeyEvent::new(
                                crossterm::event::KeyCode::Enter,
                                crossterm::event::KeyModifiers::empty(),
                            )
                            .into(),
                        );
                        tracing::info!(
                            "Speech to text: sending delayed Enter to workspace={}, pane={:?}",
                            workspace_id,
                            pane_id
                        );
                        if let Err(e) = runtime.try_send_bytes(bytes::Bytes::from(enter_bytes)) {
                            tracing::error!(
                                "Speech to text: failed to write Enter to PTY: {:?}",
                                e
                            );
                        }
                    }
                }
            }
            app.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            app.render_notify.notify_one();
            true
        }
        AppEvent::AudioSummaryFinished => {
            app.extensions.tab_summarizer = None;
            if let Some(toast) = &app.state.toast {
                if toast.title == "Audio Summary" {
                    app.state.toast = None;
                }
            }
            app.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            app.render_notify.notify_one();
            true
        }
        AppEvent::AudioSummaryError(err) => {
            app.extensions.tab_summarizer = None;
            app.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Audio Summary Error".into(),
                context: err.clone(),
                position: None,
                target: None,
            });
            app.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            app.render_notify.notify_one();
            true
        }
        _ => false,
    }
}

fn speech_target_is_agent(
    app: &crate::app::App,
    workspace_id: &str,
    pane_id: Option<crate::layout::PaneId>,
) -> bool {
    let Some(ws_idx) = app
        .state
        .workspaces
        .iter()
        .position(|ws| ws.id == workspace_id)
    else {
        return false;
    };
    let Some(ws) = app.state.workspaces.get(ws_idx) else {
        return false;
    };
    let Some(target_pane_id) = pane_id.or_else(|| ws.focused_pane_id()) else {
        return false;
    };
    ws.pane_state(target_pane_id)
        .and_then(|pane| app.state.terminals.get(&pane.attached_terminal_id))
        .is_some_and(|terminal| terminal.is_agent_terminal())
}

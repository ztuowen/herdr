use crate::app::App;
use crate::input::TerminalKey;

impl App {
    pub(crate) fn cancel_audio_summary(&mut self) {
        if !self.no_session {
            if let Err(e) = self
                .event_tx
                .try_send(crate::events::AppEvent::AudioSummaryCancel)
            {
                tracing::error!("failed to send AudioSummaryCancel event: {:?}", e);
            }
        }
        if let Some(summarizer) = self.extensions.tab_summarizer.take() {
            summarizer.stop();
        }
        if let Some(toast) = &self.state.toast {
            if toast.title == "Audio Summary" {
                self.state.toast = None;
            }
        }
        self.render_dirty
            .store(true, std::sync::atomic::Ordering::Release);
        self.render_notify.notify_one();
    }

    pub(crate) fn trigger_audio_summary(&mut self, ws_idx: usize, tab_idx: usize) {
        let Some(text_content) =
            self.state
                .gather_tab_content(&self.terminal_runtimes, ws_idx, tab_idx)
        else {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Audio Summary".into(),
                context: "No text found to summarize.".into(),
                position: None,
                target: None,
            });
            return;
        };

        if !self.no_session {
            self.cancel_audio_summary();
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::Finished,
                title: "Audio Summary".into(),
                context: "Playing audio summary...".into(),
                position: None,
                target: None,
            });
            if let Err(e) = self
                .event_tx
                .try_send(crate::events::AppEvent::AudioSummaryStart { text_content })
            {
                tracing::error!("failed to send AudioSummaryStart event: {:?}", e);
            }
            self.render_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            self.render_notify.notify_one();
            return;
        }

        let Some(api_key) = self.state.extensions.speech_to_text.gemini_api_key.clone() else {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Audio Summary Error".into(),
                context: "Gemini API key is not configured.".into(),
                position: None,
                target: None,
            });
            return;
        };

        let model = self
            .state
            .extensions
            .speech_to_text
            .model
            .clone()
            .unwrap_or_else(|| "gemini-3.1-flash-live-preview".to_string());

        self.cancel_audio_summary();

        let system_instruction = self
            .state
            .extensions
            .speech_to_text
            .summary_system_instruction
            .clone()
            .unwrap_or_else(|| {
                "You are an AI assistant. Summarize the text present on the screen layout. Be concise and conversational, as this summary will be read aloud to the user.".to_string()
            });

        let event_tx = self.event_tx.clone();

        match crate::extensions::speech::summary::start_summary(
            api_key,
            model,
            system_instruction,
            text_content,
            event_tx,
        ) {
            Ok(summarizer) => {
                self.extensions.tab_summarizer = Some(summarizer);
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::Finished,
                    title: "Audio Summary".into(),
                    context: "Playing audio summary...".into(),
                    position: None,
                    target: None,
                });
            }
            Err(err) => {
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Audio Summary Error".into(),
                    context: err,
                    position: None,
                    target: None,
                });
            }
        }

        self.render_dirty
            .store(true, std::sync::atomic::Ordering::Release);
        self.render_notify.notify_one();
    }

    pub(crate) fn start_recording(&mut self, ws_idx: usize, key: TerminalKey) -> bool {
        let Some(ws) = self.state.workspaces.get(ws_idx) else {
            return false;
        };
        let workspace_id = ws.id.clone();
        let pane_id = ws.focused_pane_id();
        let is_agent = if let Some(pid) = pane_id {
            if let Some(pane) = ws.pane_state(pid) {
                let term_id = pane.attached_terminal_id.clone();
                if let Some(term) = self.state.terminals.get(&term_id) {
                    term.is_agent_terminal()
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !self.no_session {
            self.state.extensions.recording_workspace = Some(workspace_id.clone());
            self.extensions.speech_recorder.start_server(key);
            self.state.extensions.live_transcription = Some(String::new());

            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::Finished,
                title: "Speech to Text".into(),
                context: "Listening...".into(),
                position: None,
                target: None,
            });

            if let Err(e) = self
                .event_tx
                .try_send(crate::events::AppEvent::SpeechStartRecording {
                    workspace_id,
                    pane_id,
                    is_agent,
                })
            {
                tracing::error!("failed to send SpeechStartRecording: {:?}", e);
            }
            return true;
        }

        let api_key = match &self.state.extensions.speech_to_text.gemini_api_key {
            Some(k) if !k.trim().is_empty() => k.clone(),
            _ => return false,
        };

        let model = self.state.extensions.speech_to_text.model.clone();
        let postprocess_instruction = crate::extensions::speech::postprocess_instruction(
            &self.state.extensions.speech_to_text,
            is_agent,
        );

        if let Err(e) = self.extensions.speech_recorder.start_local(
            key,
            crate::extensions::speech::pipeline::AppPipelineConfig {
                workspace_id: workspace_id.clone(),
                pane_id,
                api_key,
                model: crate::extensions::speech::model_or_default(model),
                postprocess_instruction,
                event_tx: self.event_tx.clone(),
            },
        ) {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Speech to Text".into(),
                context: e,
                position: None,
                target: None,
            });
            return false;
        }

        self.state.extensions.recording_workspace = Some(workspace_id);
        self.state.extensions.live_transcription = Some(String::new());
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "Speech to Text".into(),
            context: "Listening...".into(),
            position: None,
            target: None,
        });

        true
    }

    pub(crate) fn stop_recording(&mut self, abort: bool) {
        let was_recording = self.extensions.speech_recorder.is_recording();
        let pipeline = self.extensions.speech_recorder.stop();

        if !was_recording && !abort {
            return;
        }

        if let Some(pipeline) = &pipeline {
            if abort {
                pipeline.abort();
            } else {
                pipeline.stop();
            }
        }

        if !abort && pipeline.is_some() {
            let previous_toast = self.state.toast.clone();
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::Finished,
                title: "Speech to Text".into(),
                context: "Post-processing...".into(),
                position: None,
                target: None,
            });
            self.sync_toast_deadline(previous_toast);
        }

        if !self.no_session {
            if let Err(e) = self
                .event_tx
                .try_send(crate::events::AppEvent::SpeechStopRecording { abort })
            {
                tracing::error!("failed to send SpeechStopRecording: {:?}", e);
            }
        }

        if abort {
            self.state.extensions.recording_workspace = None;
            self.state.extensions.live_transcription = None;
        }
    }
}

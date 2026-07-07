use crate::client::ClientLoopEvent;
use crate::layout::PaneId;
use crate::protocol::ClientMessage;

#[derive(Default)]
pub(crate) struct ClientSpeechRuntime {
    recording_pipeline: Option<crate::extensions::speech::TranscriptionPipeline>,
    tab_summarizer: Option<crate::extensions::speech::summary::TabSummarizer>,
}

impl ClientSpeechRuntime {
    pub(crate) fn start_recording(
        &mut self,
        workspace_id: String,
        pane_id: Option<PaneId>,
        is_agent: bool,
        event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    ) -> Result<(), ClientMessage> {
        if let Some(pipeline) = self.recording_pipeline.take() {
            pipeline.abort();
        }

        let loaded_config =
            crate::config::load_live_config().map_err(|errs| ClientMessage::SpeechTranscribed {
                workspace_id: workspace_id.clone(),
                pane_id,
                result: Err(format!("Failed to load local config: {}", errs.join("; "))),
            })?;

        let api_key = match loaded_config.config.speech_to_text.gemini_api_key.as_ref() {
            Some(k) if !k.trim().is_empty() => k.clone(),
            _ => {
                return Err(ClientMessage::SpeechTranscribed {
                    workspace_id,
                    pane_id,
                    result: Err("Gemini API key is not configured in local config (~/.config/herdr/config.toml).".to_string()),
                });
            }
        };

        let model = crate::extensions::speech::model_or_default(
            loaded_config.config.speech_to_text.model.clone(),
        );
        let postprocess_instruction = crate::extensions::speech::postprocess_instruction(
            &loaded_config.config.speech_to_text,
            is_agent,
        );

        let pipeline =
            crate::extensions::speech::pipeline::TranscriptionPipeline::start_client_messages(
                crate::extensions::speech::pipeline::ClientPipelineConfig {
                    workspace_id: workspace_id.clone(),
                    pane_id,
                    api_key,
                    model,
                    postprocess_instruction,
                },
                event_tx,
            )
            .map_err(|e| ClientMessage::SpeechTranscribed {
                workspace_id,
                pane_id,
                result: Err(e),
            })?;

        self.recording_pipeline = Some(pipeline);
        Ok(())
    }

    pub(crate) fn stop_recording(&mut self, abort: bool) {
        if let Some(pipeline) = self.recording_pipeline.take() {
            if abort {
                pipeline.abort();
            } else {
                pipeline.stop();
            }
        }
    }

    pub(crate) fn start_audio_summary(
        &mut self,
        text_content: String,
        event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    ) -> Result<(), ClientMessage> {
        self.cancel_audio_summary();

        let loaded_config = crate::config::load_live_config().map_err(|errs| {
            ClientMessage::AudioSummaryError(format!(
                "Failed to load local config: {}",
                errs.join("; ")
            ))
        })?;

        let api_key = match loaded_config.config.speech_to_text.gemini_api_key.as_ref() {
            Some(k) if !k.trim().is_empty() => k.clone(),
            _ => {
                return Err(ClientMessage::AudioSummaryError(
                    "Gemini API key is not configured in local config (~/.config/herdr/config.toml)."
                        .to_string(),
                ));
            }
        };

        let model = loaded_config
            .config
            .speech_to_text
            .model
            .clone()
            .unwrap_or_else(|| "gemini-3.1-flash-live-preview".to_string());

        let system_instruction = loaded_config
            .config
            .speech_to_text
            .summary_system_instruction
            .clone()
            .unwrap_or_else(|| {
                "You are a helpful assistant. You will generate a concise audio summary of the user's terminal session. Focus on key developments, status changes, and any recent errors."
                    .to_string()
            });

        let summarizer = crate::extensions::speech::summary::start_client_summary(
            api_key,
            model,
            system_instruction,
            text_content,
            event_tx,
        )
        .map_err(ClientMessage::AudioSummaryError)?;

        self.tab_summarizer = Some(summarizer);
        Ok(())
    }

    pub(crate) fn cancel_audio_summary(&mut self) {
        if let Some(summarizer) = self.tab_summarizer.take() {
            summarizer.stop();
        }
    }
}

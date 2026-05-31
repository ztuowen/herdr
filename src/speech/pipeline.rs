use crate::input::TerminalKey;
use crate::speech::audio::{AudioStream, SendStream};
use crate::speech::gemini::{self, TranscriptionEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const DEFAULT_TRANSCRIPTION_MODEL: &str = "gemini-3.1-flash-live-preview";

const RAW_TRANSCRIPTION_INSTRUCTION: &str = "You are a transcription engine. Output the exact text of the audio you hear. Do not converse, do not answer questions, and do not add commentary.";
const AGENT_POSTPROCESS_INSTRUCTION: &str = "You are a post-processing engine for speech-to-text. The user is speaking to an AI coding assistant. Clean up the raw transcription to make it clear, coherent, and grammatically correct. Keep the natural phrasing but remove filler words (like 'um', 'uh', 'like') and correct homophones or mistranscribed words. Output only the corrected text without any chat or explanation.";
const TERMINAL_POSTPROCESS_INSTRUCTION: &str = "You are a post-processing engine for speech-to-text. The user is speaking to a command-line terminal. Convert the raw transcription into the most likely shell command or command-line input. Correct spacing, casing, punctuation, and spelling errors for commands, flags, and paths. Output only the corrected terminal input without any chat or explanation.";

pub fn model_or_default(model: Option<String>) -> String {
    model.unwrap_or_else(|| DEFAULT_TRANSCRIPTION_MODEL.to_string())
}

pub fn postprocess_instruction(
    config: &crate::config::SpeechToTextConfig,
    is_agent: bool,
) -> String {
    if is_agent {
        config
            .agent_system_instruction
            .clone()
            .or_else(|| config.system_instruction.clone())
            .unwrap_or_else(|| AGENT_POSTPROCESS_INSTRUCTION.to_string())
    } else {
        config
            .terminal_system_instruction
            .clone()
            .or_else(|| config.system_instruction.clone())
            .unwrap_or_else(|| TERMINAL_POSTPROCESS_INSTRUCTION.to_string())
    }
}

pub struct TranscriptionPipeline {
    _stream: SendStream,
    active: Arc<AtomicBool>,
    aborted: Option<Arc<AtomicBool>>,
}

struct CapturedAudio {
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

impl TranscriptionPipeline {
    pub fn start_app_events(config: AppPipelineConfig) -> Result<Self, String> {
        let audio = AudioStream::start()?;
        let AudioStream {
            stream,
            buffer,
            sample_rate,
            channels,
        } = audio;
        let captured_audio = CapturedAudio {
            buffer,
            sample_rate,
            channels,
        };
        let active = Arc::new(AtomicBool::new(true));
        let pipeline = Self {
            _stream: stream,
            active: active.clone(),
            aborted: None,
        };

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Speech to Text: failed to build tokio runtime: {}", e);
                    let _ =
                        config
                            .event_tx
                            .blocking_send(crate::events::AppEvent::SpeechTranscribed {
                                workspace_id: config.workspace_id,
                                pane_id: config.pane_id,
                                result: Err(format!("Failed to build tokio runtime: {}", e)),
                            });
                    return;
                }
            };

            rt.block_on(run_app_pipeline(config, captured_audio, active));
        });

        Ok(pipeline)
    }

    pub fn start_client_messages(
        config: ClientPipelineConfig,
        event_tx: tokio::sync::mpsc::Sender<crate::client::ClientLoopEvent>,
    ) -> Result<Self, String> {
        let audio = AudioStream::start()?;
        let AudioStream {
            stream,
            buffer,
            sample_rate,
            channels,
        } = audio;
        let captured_audio = CapturedAudio {
            buffer,
            sample_rate,
            channels,
        };
        let active = Arc::new(AtomicBool::new(true));
        let aborted = Arc::new(AtomicBool::new(false));
        let pipeline = Self {
            _stream: stream,
            active: active.clone(),
            aborted: Some(aborted.clone()),
        };

        tokio::spawn(run_client_pipeline(
            config,
            captured_audio,
            active,
            aborted,
            event_tx,
        ));

        Ok(pipeline)
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
    }

    pub fn abort(&self) {
        self.active.store(false, Ordering::Release);
        if let Some(aborted) = &self.aborted {
            aborted.store(true, Ordering::Release);
        }
    }
}

pub struct AppPipelineConfig {
    pub workspace_id: String,
    pub pane_id: Option<crate::layout::PaneId>,
    pub api_key: String,
    pub model: String,
    pub postprocess_instruction: String,
    pub event_tx: tokio::sync::mpsc::Sender<crate::events::AppEvent>,
}

pub struct ClientPipelineConfig {
    pub workspace_id: String,
    pub pane_id: Option<crate::layout::PaneId>,
    pub api_key: String,
    pub model: String,
    pub postprocess_instruction: String,
}

async fn run_app_pipeline(
    config: AppPipelineConfig,
    audio: CapturedAudio,
    active: Arc<AtomicBool>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    tokio::spawn(gemini::run_websocket_transcription(
        config.api_key.clone(),
        config.model,
        RAW_TRANSCRIPTION_INSTRUCTION.to_string(),
        audio.buffer,
        audio.channels,
        audio.sample_rate,
        active,
        None,
        tx,
    ));

    while let Some(event) = rx.recv().await {
        match event {
            TranscriptionEvent::Partial(text) => {
                let _ = config
                    .event_tx
                    .send(crate::events::AppEvent::SpeechPartialTranscription {
                        workspace_id: config.workspace_id.clone(),
                        text,
                    })
                    .await;
            }
            TranscriptionEvent::Finished(result) => {
                let result = postprocess_result(
                    config.api_key.clone(),
                    config.postprocess_instruction.clone(),
                    result,
                )
                .await;
                let _ = config
                    .event_tx
                    .send(crate::events::AppEvent::SpeechTranscribed {
                        workspace_id: config.workspace_id.clone(),
                        pane_id: config.pane_id,
                        result,
                    })
                    .await;
            }
        }
    }
}

async fn run_client_pipeline(
    config: ClientPipelineConfig,
    audio: CapturedAudio,
    active: Arc<AtomicBool>,
    aborted: Arc<AtomicBool>,
    event_tx: tokio::sync::mpsc::Sender<crate::client::ClientLoopEvent>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    tokio::spawn(gemini::run_websocket_transcription(
        config.api_key.clone(),
        config.model,
        RAW_TRANSCRIPTION_INSTRUCTION.to_string(),
        audio.buffer,
        audio.channels,
        audio.sample_rate,
        active,
        Some(aborted),
        tx,
    ));

    while let Some(event) = rx.recv().await {
        match event {
            TranscriptionEvent::Partial(text) => {
                let _ = event_tx
                    .send(crate::client::ClientLoopEvent::ClientMessageToSend(
                        crate::protocol::ClientMessage::SpeechPartialTranscription {
                            workspace_id: config.workspace_id.clone(),
                            text,
                        },
                    ))
                    .await;
            }
            TranscriptionEvent::Finished(result) => {
                let result = postprocess_result(
                    config.api_key.clone(),
                    config.postprocess_instruction.clone(),
                    result,
                )
                .await;
                let _ = event_tx
                    .send(crate::client::ClientLoopEvent::ClientMessageToSend(
                        crate::protocol::ClientMessage::SpeechTranscribed {
                            workspace_id: config.workspace_id.clone(),
                            pane_id: config.pane_id,
                            result,
                        },
                    ))
                    .await;
            }
        }
    }
}

async fn postprocess_result(
    api_key: String,
    postprocess_instruction: String,
    result: Result<String, String>,
) -> Result<String, String> {
    match result {
        Ok(text) => tokio::task::spawn_blocking(move || {
            gemini::run_gemini_postprocess(api_key, text, postprocess_instruction)
        })
        .await
        .unwrap_or_else(|e| Err(format!("Post-process task join error: {}", e))),
        Err(e) => Err(e),
    }
}

/// Encapsulates speech-to-text recording state for App.
pub struct SpeechRecorder {
    pub(crate) pipeline: Option<TranscriptionPipeline>,
    pub(crate) start_time: Option<std::time::Instant>,
    pub(crate) key: Option<TerminalKey>,
    pub(crate) is_toggle: bool,
}

impl SpeechRecorder {
    /// Creates a new, inactive `SpeechRecorder`.
    pub fn new() -> Self {
        Self {
            pipeline: None,
            start_time: None,
            key: None,
            is_toggle: false,
        }
    }

    /// Checks if a speech recording is currently in progress.
    #[allow(dead_code)] // Exposed as part of the public SpeechRecorder API for completeness
    pub fn is_recording(&self) -> bool {
        self.start_time.is_some()
    }

    /// Returns the start time of the active recording, if any.
    pub fn start_time(&self) -> Option<std::time::Instant> {
        self.start_time
    }

    /// Returns the key that was pressed to trigger the active recording, if any.
    pub fn recording_key(&self) -> Option<TerminalKey> {
        self.key
    }

    /// Starts tracking a recording session in server/client mode.
    pub fn start_server(&mut self, key: TerminalKey) {
        self.key = Some(key);
        self.start_time = Some(std::time::Instant::now());
        self.is_toggle = false;
    }

    /// Starts a local monolithic recording session.
    pub fn start_local(
        &mut self,
        key: TerminalKey,
        config: AppPipelineConfig,
    ) -> Result<(), String> {
        self.key = Some(key);
        self.start_time = Some(std::time::Instant::now());
        self.is_toggle = false;
        self.pipeline = Some(TranscriptionPipeline::start_app_events(config)?);
        Ok(())
    }

    /// Stops the current recording session.
    pub fn stop(&mut self) -> Option<TranscriptionPipeline> {
        self.key = None;
        self.start_time = None;
        self.is_toggle = false;
        self.pipeline.take()
    }
}

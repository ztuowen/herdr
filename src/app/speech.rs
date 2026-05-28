use super::App;
use crate::input::TerminalKey;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;

impl App {
    pub(crate) fn start_recording(&mut self, ws_idx: usize, key: TerminalKey) -> bool {
        let Some(ws) = self.state.workspaces.get(ws_idx) else {
            return false;
        };
        let workspace_id = ws.id.clone();

        let api_key = match &self.state.speech_to_text.gemini_api_key {
            Some(k) if !k.trim().is_empty() => k.clone(),
            _ => return false,
        };

        let is_agent = if let Some(pane_id) = ws.focused_pane_id() {
            if let Some(pane) = ws.pane_state(pane_id) {
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

        let system_instruction = if is_agent {
            self.state.speech_to_text.agent_system_instruction.clone()
        } else {
            self.state.speech_to_text.terminal_system_instruction.clone()
        };
        let system_instruction = system_instruction
            .or_else(|| self.state.speech_to_text.system_instruction.clone())
            .unwrap_or_else(|| "You are a transcription engine. Output the exact text of the audio you hear. Do not converse, do not answer questions, and do not add commentary.".to_string());

        let model = self
            .state
            .speech_to_text
            .model
            .clone()
            .unwrap_or_else(|| "gemini-3.1-flash-live-preview".to_string());

        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(dev) => dev,
            None => {
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Speech to Text".into(),
                    context: "No default input device found.".into(),
                    target: None,
                });
                return false;
            }
        };

        let config = match device.default_input_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Speech to Text".into(),
                    context: format!("Failed to get input config: {}", e),
                    target: None,
                });
                return false;
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        let buffer = Arc::new(std::sync::Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        let err_fn = move |err| {
            tracing::error!("Audio recording stream error: {}", err);
        };

        let stream = match sample_format {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    write_input_data(data, &buffer_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| {
                    write_input_data(data, &buffer_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    write_input_data(data, &buffer_clone);
                },
                err_fn,
                None,
            ),
            _ => {
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Speech to Text".into(),
                    context: "Unsupported sample format.".into(),
                    target: None,
                });
                return false;
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                self.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Speech to Text".into(),
                    context: format!("Failed to build input stream: {}", e),
                    target: None,
                });
                return false;
            }
        };

        if let Err(e) = stream.play() {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Speech to Text".into(),
                context: format!("Failed to play input stream: {}", e),
                target: None,
            });
            return false;
        }

        self.recording_stream = Some(stream);
        self.recording_buffer = Some(buffer.clone());
        self.recording_sample_rate = sample_rate;
        self.recording_channels = channels;
        self.recording_key = Some(key);
        self.recording_start_time = Some(std::time::Instant::now());
        self.state.recording_workspace = Some(workspace_id.clone());

        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "Speech to Text".into(),
            context: "Listening...".into(),
            target: None,
        });

        // Initialize live transcription
        self.state.live_transcription = Some(String::new());

        let recording_active = Arc::new(std::sync::atomic::AtomicBool::new(true));
        self.recording_active = Some(recording_active.clone());

        let event_tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Speech to Text: failed to build tokio runtime: {}", e);
                    let _ = event_tx.blocking_send(crate::events::AppEvent::SpeechTranscribed {
                        workspace_id: workspace_id.clone(),
                        result: Err(format!("Failed to build tokio runtime: {}", e)),
                    });
                    return;
                }
            };

            rt.block_on(async {
                run_websocket_transcription(
                    workspace_id,
                    api_key,
                    model,
                    system_instruction,
                    buffer,
                    channels,
                    sample_rate,
                    recording_active,
                    event_tx,
                )
                .await;
            });
        });

        true
    }

    pub(crate) fn stop_recording(&mut self) -> Option<(Vec<f32>, u32, u16)> {
        self.recording_stream = None;
        self.recording_buffer = None;
        self.recording_key = None;
        self.recording_start_time = None;

        if let Some(active) = self.recording_active.take() {
            active.store(false, std::sync::atomic::Ordering::Release);

            let previous_toast = self.state.toast.clone();
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::Finished,
                title: "Speech to Text".into(),
                context: "Transcribing...".into(),
                target: None,
            });
            self.sync_toast_deadline(previous_toast);
        }

        None
    }
}

fn write_input_data<T>(data: &[T], buffer: &Arc<std::sync::Mutex<Vec<f32>>>)
where
    T: cpal::Sample<Float = f32>,
{
    if let Ok(mut buf) = buffer.lock() {
        let is_first = buf.is_empty();
        for &sample in data {
            buf.push(sample.to_float_sample());
        }
        if is_first && !data.is_empty() {
            tracing::debug!(
                "Speech to Text: write_input_data received first batch of {} samples",
                data.len()
            );
        }
    }
}

fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    let channels = channels as usize;
    let len = samples.len() / channels;
    let mut mono = Vec::with_capacity(len);
    for i in 0..len {
        let mut sum = 0.0f32;
        for c in 0..channels {
            if let Some(&sample) = samples.get(i * channels + c) {
                sum += sample;
            }
        }
        mono.push(sum / channels as f32);
    }
    mono
}

fn resample(samples: &[f32], src_rate: u32, target_rate: u32) -> Vec<f32> {
    if src_rate == target_rate {
        return samples.to_vec();
    }
    let ratio = src_rate as f64 / target_rate as f64;
    let target_len = (samples.len() as f64 / ratio).round() as usize;
    let mut resampled = Vec::with_capacity(target_len);
    for i in 0..target_len {
        let src_index = i as f64 * ratio;
        let index = src_index.floor() as usize;
        let fract = src_index - index as f64;
        if index + 1 < samples.len() {
            let s1 = samples[index];
            let s2 = samples[index + 1];
            resampled.push((s1 as f64 + (s2 - s1) as f64 * fract) as f32);
        } else if index < samples.len() {
            resampled.push(samples[index]);
        }
    }
    resampled
}

async fn run_websocket_transcription(
    workspace_id: String,
    api_key: String,
    model: String,
    system_instruction: String,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    channels: u16,
    sample_rate: u32,
    recording_active: Arc<std::sync::atomic::AtomicBool>,
    event_tx: tokio::sync::mpsc::Sender<crate::events::AppEvent>,
) {
    use base64::Engine;
    use futures_util::{SinkExt, StreamExt};

    let model_name = if model.starts_with("models/") {
        model
    } else {
        format!("models/{}", model)
    };

    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key={}",
        api_key
    );

    tracing::info!("Speech to Text: connecting to Gemini Live WebSocket...");
    let ws_stream = match tokio_tungstenite::connect_async(&url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!(
                "Speech to Text: failed to connect to Gemini Live WebSocket: {}",
                e
            );
            let _ = event_tx
                .send(crate::events::AppEvent::SpeechTranscribed {
                    workspace_id,
                    result: Err(format!("WebSocket connection failed: {}", e)),
                })
                .await;
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    let setup_msg = serde_json::json!({
        "setup": {
            "model": model_name,
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "maxOutputTokens": 1
            },
            "systemInstruction": {
                "parts": [
                    {
                        "text": system_instruction
                    }
                ]
            },
            "inputAudioTranscription": {},
            "outputAudioTranscription": {}
        }
    });

    tracing::info!("Speech to Text: sending setup message...");
    if let Err(e) = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            setup_msg.to_string(),
        ))
        .await
    {
        tracing::error!("Speech to Text: failed to send setup message: {}", e);
        let _ = event_tx
            .send(crate::events::AppEvent::SpeechTranscribed {
                workspace_id,
                result: Err(format!("Failed to send setup message: {}", e)),
            })
            .await;
        return;
    }

    let mut finalized_text = String::new();
    let mut current_turn_text = String::new();
    let mut recording_stopped = false;
    let mut stop_time: Option<std::time::Instant> = None;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !recording_stopped {
                    if !recording_active.load(std::sync::atomic::Ordering::Acquire) {
                        tracing::info!("Speech to Text: stop recording detected, sending turnComplete (audioStreamEnd)...");
                        recording_stopped = true;
                        stop_time = Some(std::time::Instant::now());

                        let turn_msg = serde_json::json!({
                            "realtimeInput": {
                                "audioStreamEnd": true
                            }
                        });

                        if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(turn_msg.to_string())).await {
                            tracing::error!("Speech to Text: failed to send audioStreamEnd message: {}", e);
                        }
                        continue;
                    }

                    let new_samples = {
                        match buffer.lock() {
                            Ok(mut buf) => {
                                let samples = std::mem::take(&mut *buf);
                                if !samples.is_empty() {
                                    tracing::debug!("Speech to Text: read {} raw samples from buffer", samples.len());
                                }
                                samples
                            }
                            Err(_) => Vec::new(),
                        }
                    };

                    if !new_samples.is_empty() {
                        let mono = downmix_to_mono(&new_samples, channels);
                        let resampled = resample(&mono, sample_rate, 16000);

                        tracing::debug!("Speech to Text: downmixed to {} samples, resampled to {} samples", mono.len(), resampled.len());

                        let mut raw_bytes = Vec::with_capacity(resampled.len() * 2);
                        for &sample in &resampled {
                            let clamped = sample.clamp(-1.0, 1.0);
                            let scaled = (clamped * 32767.0) as i16;
                            raw_bytes.extend_from_slice(&scaled.to_le_bytes());
                        }

                        let base64_audio = base64::prelude::BASE64_STANDARD.encode(&raw_bytes);

                        let media_msg = serde_json::json!({
                            "realtimeInput": {
                                "audio": {
                                    "mimeType": "audio/pcm;rate=16000",
                                    "data": base64_audio
                                }
                            }
                        });

                        if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(media_msg.to_string())).await {
                            tracing::error!("Speech to Text: failed to send audio chunk: {}", e);
                            break;
                        }
                    }
                }
            }

            msg_res = read.next() => {
                let msg = match msg_res {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::error!("Speech to Text: WebSocket read error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("Speech to Text: WebSocket stream ended by server");
                        break;
                    }
                };

                tracing::debug!("Speech to Text: WebSocket message received: {:?}", msg);
                let text_opt = match &msg {
                    tokio_tungstenite::tungstenite::Message::Text(text) => Some(text.clone()),
                    tokio_tungstenite::tungstenite::Message::Binary(bin) => String::from_utf8(bin.clone()).ok(),
                    _ => None,
                };

                if let Some(text) = text_opt {
                    if let Some((partial, turn_complete)) = parse_live_transcription_frame(&text) {
                        if !partial.is_empty() {
                            current_turn_text.push_str(&partial);

                            let mut full_text = finalized_text.clone();
                            if !full_text.is_empty() && !current_turn_text.is_empty() {
                                full_text.push(' ');
                            }
                            full_text.push_str(&current_turn_text);

                            let _ = event_tx.send(crate::events::AppEvent::SpeechPartialTranscription {
                                workspace_id: workspace_id.clone(),
                                text: full_text,
                            }).await;
                        }
                        if turn_complete {
                            tracing::info!("Speech to Text: turnComplete received from server");
                            if !current_turn_text.is_empty() {
                                if !finalized_text.is_empty() {
                                    finalized_text.push(' ');
                                }
                                finalized_text.push_str(&current_turn_text);
                                current_turn_text = String::new();
                            }
                            if recording_stopped {
                                break;
                            }
                        }
                    }
                }

                if let tokio_tungstenite::tungstenite::Message::Close(frame) = msg {
                    tracing::info!("Speech to Text: Close frame received: {:?}", frame);
                    break;
                }
            }
        }

        if recording_stopped {
            if let Some(t) = stop_time {
                if t.elapsed() > tokio::time::Duration::from_secs(5) {
                    tracing::warn!("Speech to Text: grace period of 5 seconds expired waiting for final transcription");
                    break;
                }
            }
        }
    }

    let mut final_text = finalized_text.clone();
    if !current_turn_text.is_empty() {
        if !final_text.is_empty() {
            final_text.push(' ');
        }
        final_text.push_str(&current_turn_text);
    }

    let result = if !final_text.trim().is_empty() {
        Ok(final_text.trim().to_string())
    } else {
        Err("No speech detected / transcription empty.".to_string())
    };

    let _ = event_tx
        .send(crate::events::AppEvent::SpeechTranscribed {
            workspace_id,
            result,
        })
        .await;
}

fn parse_live_transcription_frame(json_str: &str) -> Option<(String, bool)> {
    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if let Some(err) = json
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        tracing::error!("Speech to Text: server returned error: {}", err);
        return None;
    }

    let mut text = String::new();
    let mut turn_complete = false;

    if let Some(server_content) = json
        .get("serverContent")
        .or_else(|| json.get("server_content"))
    {
        if let Some(input_trans) = server_content
            .get("inputTranscription")
            .or_else(|| server_content.get("input_transcription"))
        {
            if let Some(t) = input_trans.get("text").and_then(|v| v.as_str()) {
                text.push_str(t);
            }
        }
        if let Some(tc) = server_content
            .get("turnComplete")
            .or_else(|| server_content.get("turn_complete"))
            .and_then(|v| v.as_bool())
        {
            turn_complete = tc;
        }
    }

    if let Some(input_trans) = json
        .get("inputTranscription")
        .or_else(|| json.get("input_transcription"))
    {
        if let Some(t) = input_trans.get("text").and_then(|v| v.as_str()) {
            text.push_str(t);
        }
    }

    if let Some(tc) = json
        .get("turnComplete")
        .or_else(|| json.get("turn_complete"))
        .and_then(|v| v.as_bool())
    {
        turn_complete = tc;
    }

    Some((text, turn_complete))
}

#[cfg(test)]
mod tests {
    use crate::app::tests::test_app;

    #[test]
    fn speech_transcribed_event_handles_result() {
        let mut app = test_app();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("main")];
        let workspace_id = app.state.workspaces[0].id.clone();

        let event_ok = crate::events::AppEvent::SpeechTranscribed {
            workspace_id: workspace_id.clone(),
            result: Ok("hello world".to_string()),
        };
        app.handle_internal_event(event_ok);

        let toast = app.state.toast.as_ref().unwrap();
        assert_eq!(toast.title, "Speech to Text");
        assert_eq!(toast.context, "hello world");
        assert!(app.toast_deadline.is_some());

        let event_err = crate::events::AppEvent::SpeechTranscribed {
            workspace_id,
            result: Err("mic failure".to_string()),
        };
        app.handle_internal_event(event_err);

        let toast = app.state.toast.as_ref().unwrap();
        assert_eq!(toast.title, "Speech to Text Error");
        assert_eq!(toast.context, "mic failure");
        assert!(app.toast_deadline.is_some());
    }

    #[test]
    fn test_sync_toast_deadline_exemption() {
        let mut app = test_app();

        // Transcribing... should not set a deadline
        app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "Speech to Text".into(),
            context: "Transcribing...".into(),
            target: None,
        });
        app.sync_toast_deadline(None);
        assert!(app.toast_deadline.is_none());

        // Other content under Finished should set a deadline (5 seconds)
        app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "Speech to Text".into(),
            context: "hello world".into(),
            target: None,
        });
        let previous = app.state.toast.clone();
        app.sync_toast_deadline(None);
        assert!(app.toast_deadline.is_some());

        // Reset deadline and test abort
        app.toast_deadline = None;
        app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: "Speech to Text".into(),
            context: "Recording aborted.".into(),
            target: None,
        });
        app.sync_toast_deadline(previous);
        assert!(app.toast_deadline.is_some());
    }

    #[test]
    fn test_parse_live_transcription_frame() {
        use super::parse_live_transcription_frame;

        // Test camelCase nested structure
        let msg = r#"{"serverContent": {"inputTranscription": {"text": "hello "}, "turnComplete": true}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("hello ".to_string(), true)));

        // Test snake_case nested structure
        let msg = r#"{"server_content": {"input_transcription": {"text": "world"}, "turn_complete": false}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("world".to_string(), false)));

        // Test root level structure (camelCase)
        let msg = r#"{"inputTranscription": {"text": "foo"}, "turnComplete": true}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("foo".to_string(), true)));

        // Test root level structure (snake_case)
        let msg = r#"{"input_transcription": {"text": "bar"}, "turn_complete": false}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("bar".to_string(), false)));

        // Test error response
        let msg = r#"{"error": {"message": "Invalid API key"}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, None);

        // Test empty/invalid JSON
        assert_eq!(parse_live_transcription_frame(""), None);
        assert_eq!(parse_live_transcription_frame("{invalid}"), None);
    }
}

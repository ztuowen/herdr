use crate::input::TerminalKey;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;

/// Thread-safe Send/Sync wrapper around cpal::Stream.
pub struct SendStream(#[allow(dead_code)] pub cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

/// Reusable audio capture stream configuration and buffer.
pub struct AudioStream {
    pub stream: SendStream,
    pub buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioStream {
    /// Discovers the default input device and builds/starts a CPAL recording stream.
    pub fn start() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default input device found.".to_string())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

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
            _ => return Err("Unsupported sample format.".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to play input stream: {}", e))?;

        Ok(Self {
            stream: SendStream(stream),
            buffer,
            sample_rate,
            channels,
        })
    }
}

/// Transcription events produced by the WebSocket streaming thread.
#[derive(Debug)]
pub enum TranscriptionEvent {
    Partial(String),
    Finished(Result<String, String>),
}

/// Encapsulates the speech-to-text recording state and runtime machinery for App.
pub struct SpeechRecorder {
    pub(crate) stream: Option<SendStream>,
    pub(crate) active: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) start_time: Option<std::time::Instant>,
    pub(crate) key: Option<TerminalKey>,
    pub(crate) is_toggle: bool,
}

impl SpeechRecorder {
    /// Creates a new, inactive `SpeechRecorder`.
    pub fn new() -> Self {
        Self {
            stream: None,
            active: None,
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
        workspace_id: String,
        key: TerminalKey,
        api_key: String,
        model: String,
        app_event_tx: tokio::sync::mpsc::Sender<crate::events::AppEvent>,
    ) -> Result<(), String> {
        self.key = Some(key);
        self.start_time = Some(std::time::Instant::now());
        self.is_toggle = false;

        let audio = AudioStream::start()?;
        self.stream = Some(audio.stream);
        let active = Arc::new(std::sync::atomic::AtomicBool::new(true));
        self.active = Some(active.clone());

        let system_instruction = "You are a transcription engine. Output the exact text of the audio you hear. Do not converse, do not answer questions, and do not add commentary.".to_string();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Speech to Text: failed to build tokio runtime: {}", e);
                    let _ =
                        app_event_tx.blocking_send(crate::events::AppEvent::SpeechRawTranscribed {
                            workspace_id,
                            result: Err(format!("Failed to build tokio runtime: {}", e)),
                        });
                    return;
                }
            };

            let app_event_tx_clone = app_event_tx.clone();
            let workspace_id_clone = workspace_id.clone();
            rt.block_on(async {
                tokio::spawn(async move {
                    run_websocket_transcription(
                        api_key,
                        model,
                        system_instruction,
                        audio.buffer,
                        audio.channels,
                        audio.sample_rate,
                        active,
                        None,
                        tx,
                    )
                    .await;
                });

                while let Some(event) = rx.recv().await {
                    match event {
                        TranscriptionEvent::Partial(text) => {
                            let _ = app_event_tx_clone
                                .send(crate::events::AppEvent::SpeechPartialTranscription {
                                    workspace_id: workspace_id_clone.clone(),
                                    text,
                                })
                                .await;
                        }
                        TranscriptionEvent::Finished(result) => {
                            let _ = app_event_tx_clone
                                .send(crate::events::AppEvent::SpeechRawTranscribed {
                                    workspace_id: workspace_id_clone.clone(),
                                    result,
                                })
                                .await;
                        }
                    }
                }
            });
        });

        Ok(())
    }

    /// Stops the current recording session.
    pub fn stop(&mut self) -> Option<Arc<std::sync::atomic::AtomicBool>> {
        self.stream = None;
        self.key = None;
        self.start_time = None;
        self.is_toggle = false;
        self.active.take()
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

/// Downmixes a multi-channel audio buffer to mono.
pub fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
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

/// Linearly resamples an audio buffer from `src_rate` to `target_rate`.
pub fn resample(samples: &[f32], src_rate: u32, target_rate: u32) -> Vec<f32> {
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

/// Runs the main WebSocket streaming transcription loop, writing events to the Sender channel.
pub async fn run_websocket_transcription(
    api_key: String,
    model: String,
    system_instruction: String,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    channels: u16,
    sample_rate: u32,
    recording_active: Arc<std::sync::atomic::AtomicBool>,
    recording_aborted: Option<Arc<std::sync::atomic::AtomicBool>>,
    event_tx: tokio::sync::mpsc::Sender<TranscriptionEvent>,
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

    let ws_stream = match tokio_tungstenite::connect_async(&url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!(
                "Speech to Text: failed to connect to Gemini Live WebSocket: {}",
                e
            );
            let _ = event_tx
                .send(TranscriptionEvent::Finished(Err(format!(
                    "WebSocket connection failed: {}",
                    e
                ))))
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

    if let Err(e) = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            setup_msg.to_string(),
        ))
        .await
    {
        tracing::error!("Speech to Text: failed to send setup message: {}", e);
        let _ = event_tx
            .send(TranscriptionEvent::Finished(Err(format!(
                "Failed to send setup message: {}",
                e
            ))))
            .await;
        return;
    }

    let mut finalized_text = String::new();
    let mut current_turn_text = String::new();
    let mut recording_stopped = false;
    let mut stop_time: Option<std::time::Instant> = None;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

    loop {
        if let Some(aborted) = &recording_aborted {
            if aborted.load(std::sync::atomic::Ordering::Acquire) {
                return;
            }
        }

        tokio::select! {
            _ = interval.tick() => {
                if !recording_stopped {
                    if !recording_active.load(std::sync::atomic::Ordering::Acquire) {
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
                            Ok(mut buf) => std::mem::take(&mut *buf),
                            Err(_) => Vec::new(),
                        }
                    };

                    if !new_samples.is_empty() {
                        let mono = downmix_to_mono(&new_samples, channels);
                        let resampled = resample(&mono, sample_rate, 16000);

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
                        break;
                    }
                };

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

                            let _ = event_tx.send(TranscriptionEvent::Partial(full_text)).await;
                        }
                        if turn_complete {
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

                if let tokio_tungstenite::tungstenite::Message::Close(_frame) = msg {
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

    let _ = event_tx.send(TranscriptionEvent::Finished(result)).await;
}

/// Parses a JSON frame from the Gemini Live WebSocket, returning the partial transcription
/// text and whether the turn is complete.
pub fn parse_live_transcription_frame(json_str: &str) -> Option<(String, bool)> {
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

/// Runs the Gemini post-processing API request.
pub fn run_gemini_postprocess(
    api_key: String,
    raw_text: String,
    system_instruction: String,
) -> Result<String, String> {
    let payload = serde_json::json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": raw_text
                    }
                ]
            }
        ],
        "systemInstruction": {
            "parts": [
                {
                    "text": system_instruction
                }
            ]
        }
    });

    let payload_str = payload.to_string();

    let output = std::process::Command::new("curl")
        .args([
            "-sL",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &payload_str,
            &format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-lite:generateContent?key={}", api_key)
        ])
        .output()
        .map_err(|e| format!("Failed to run curl for postprocess: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!(
            "Gemini postprocess API request failed: {}",
            err_msg
        ));
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

    if let Some(err) = response
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        return Err(format!("Gemini postprocess API error: {}", err));
    }

    let processed_text = response
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .and_then(|p| p.first())
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Failed to extract text from Gemini postprocess response".to_string())?;

    Ok(processed_text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_live_transcription_frame() {
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

pub mod summary;

use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::events::AppEvent;

const MIN_SUMMARY_OUTPUT_CHANNELS: u16 = 2;

pub struct TabSummarizer {
    _stream: cpal::Stream,
    active: Arc<AtomicBool>,
}

impl TabSummarizer {
    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub enum SummaryEvent {
    Finished,
    Error(String),
}

pub fn start_summary(
    api_key: String,
    model: String,
    system_instruction: String,
    text_content: String,
    app_event_tx: mpsc::Sender<AppEvent>,
) -> Result<TabSummarizer, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No default output device found.".to_string())?;

    let config = summary_output_config(&device)?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();

    let audio_buffer = Arc::new(Mutex::new(VecDeque::<f32>::new()));
    let active = Arc::new(AtomicBool::new(true));

    let audio_buffer_clone = audio_buffer.clone();
    let active_clone = active.clone();

    let err_fn = |err| error!("Audio playback stream error: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        _ => return Err("Unsupported sample format.".to_string()),
    }
    .map_err(|e| format!("Failed to build output stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start output stream: {}", e))?;

    let (summary_event_tx, mut summary_event_rx) = mpsc::channel::<SummaryEvent>(10);

    let active_task = active.clone();
    tokio::spawn(async move {
        run_websocket_summary(
            api_key,
            model,
            system_instruction,
            text_content,
            audio_buffer,
            sample_rate,
            active_task,
            summary_event_tx,
        )
        .await;
    });

    tokio::spawn(async move {
        while let Some(event) = summary_event_rx.recv().await {
            match event {
                SummaryEvent::Finished => {
                    let _ = app_event_tx.send(AppEvent::AudioSummaryFinished).await;
                }
                SummaryEvent::Error(err) => {
                    let _ = app_event_tx.send(AppEvent::AudioSummaryError(err)).await;
                }
            }
        }
    });

    Ok(TabSummarizer {
        _stream: stream,
        active,
    })
}

pub fn start_client_summary(
    api_key: String,
    model: String,
    system_instruction: String,
    text_content: String,
    event_tx: mpsc::Sender<crate::client::ClientLoopEvent>,
) -> Result<TabSummarizer, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No default output device found.".to_string())?;

    let config = summary_output_config(&device)?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();

    let audio_buffer = Arc::new(Mutex::new(VecDeque::<f32>::new()));
    let active = Arc::new(AtomicBool::new(true));

    let audio_buffer_clone = audio_buffer.clone();
    let active_clone = active.clone();

    let err_fn = |err| error!("Audio playback stream error: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                write_output_data(data, channels, &audio_buffer_clone, &active_clone);
            },
            err_fn,
            None,
        ),
        _ => return Err("Unsupported sample format.".to_string()),
    }
    .map_err(|e| format!("Failed to build output stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start output stream: {}", e))?;

    let (summary_event_tx, mut summary_event_rx) = mpsc::channel::<SummaryEvent>(10);

    let active_task = active.clone();
    tokio::spawn(async move {
        run_websocket_summary(
            api_key,
            model,
            system_instruction,
            text_content,
            audio_buffer,
            sample_rate,
            active_task,
            summary_event_tx,
        )
        .await;
    });

    tokio::spawn(async move {
        while let Some(event) = summary_event_rx.recv().await {
            match event {
                SummaryEvent::Finished => {
                    let _ = event_tx
                        .send(crate::client::ClientLoopEvent::ClientMessageToSend(
                            crate::protocol::ClientMessage::AudioSummaryFinished,
                        ))
                        .await;
                }
                SummaryEvent::Error(err) => {
                    let _ = event_tx
                        .send(crate::client::ClientLoopEvent::ClientMessageToSend(
                            crate::protocol::ClientMessage::AudioSummaryError(err),
                        ))
                        .await;
                }
            }
        }
    });

    Ok(TabSummarizer {
        _stream: stream,
        active,
    })
}

fn summary_output_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    let default_config = device
        .default_output_config()
        .map_err(|e| format!("Failed to get default output config: {}", e))?;

    if default_config.channels() >= MIN_SUMMARY_OUTPUT_CHANNELS {
        return Ok(default_config);
    }

    let supported_configs = device
        .supported_output_configs()
        .map_err(|e| format!("Failed to get supported output configs: {}", e))?;

    stereo_output_config_from_ranges(&default_config, supported_configs).ok_or_else(|| {
        "Default output device does not advertise a stereo output config.".to_string()
    })
}

fn stereo_output_config_from_ranges(
    default_config: &cpal::SupportedStreamConfig,
    ranges: impl IntoIterator<Item = cpal::SupportedStreamConfigRange>,
) -> Option<cpal::SupportedStreamConfig> {
    let preferred_sample_rate = default_config.sample_rate();
    let preferred_sample_format = default_config.sample_format();

    ranges
        .into_iter()
        .filter(|range| range.channels() >= MIN_SUMMARY_OUTPUT_CHANNELS)
        .map(|range| {
            let min_rate = range.min_sample_rate().0;
            let max_rate = range.max_sample_rate().0;
            let sample_rate = cpal::SampleRate(preferred_sample_rate.0.clamp(min_rate, max_rate));
            let score = (
                range.sample_format() != preferred_sample_format,
                range.channels() != MIN_SUMMARY_OUTPUT_CHANNELS,
                sample_rate.0.abs_diff(preferred_sample_rate.0),
                range.channels(),
            );

            (score, range.with_sample_rate(sample_rate))
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, config)| config)
}

fn write_output_data<T>(
    data: &mut [T],
    channels: u16,
    buffer: &Arc<Mutex<VecDeque<f32>>>,
    active: &Arc<AtomicBool>,
) where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    if !active.load(Ordering::Acquire) {
        for s in data.iter_mut() {
            *s = T::from_sample(0.0);
        }
        return;
    }

    let mut buf = match buffer.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let channels = channels as usize;
    for frame in data.chunks_mut(channels) {
        if let Some(sample) = buf.pop_front() {
            for s in frame.iter_mut() {
                *s = T::from_sample(sample);
            }
        } else {
            // Buffer is empty.
            for s in frame.iter_mut() {
                *s = T::from_sample(0.0);
            }
        }
    }
}

async fn run_websocket_summary(
    api_key: String,
    model: String,
    system_instruction: String,
    text_content: String,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    output_sample_rate: u32,
    active: Arc<AtomicBool>,
    summary_event_tx: mpsc::Sender<SummaryEvent>,
) {
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
            error!("Audio Summary: failed to connect to Gemini Live: {}", e);
            let _ = summary_event_tx
                .send(SummaryEvent::Error(format!(
                    "WebSocket connection failed: {}",
                    e
                )))
                .await;
            active.store(false, Ordering::Release);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    let setup_msg = serde_json::json!({
        "setup": {
            "model": model_name,
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "speechConfig": {
                    "voiceConfig": {
                        "prebuiltVoiceConfig": {
                            "voiceName": "Puck"
                        }
                    }
                }
            },
            "systemInstruction": {
                "parts": [
                    {
                        "text": system_instruction
                    }
                ]
            }
        }
    });

    if let Err(e) = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            setup_msg.to_string(),
        ))
        .await
    {
        error!("Audio Summary: failed to send setup message: {}", e);
        let _ = summary_event_tx
            .send(SummaryEvent::Error(format!(
                "Failed to send setup message: {}",
                e
            )))
            .await;
        active.store(false, Ordering::Release);
        return;
    }

    if let Err(err) = wait_for_summary_setup_complete(&mut read).await {
        let _ = summary_event_tx.send(SummaryEvent::Error(err)).await;
        active.store(false, Ordering::Release);
        return;
    }

    let content_msg = serde_json::json!({
        "realtimeInput": {
            "text": text_content
        }
    });

    if let Err(e) = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            content_msg.to_string(),
        ))
        .await
    {
        error!("Audio Summary: failed to send content message: {}", e);
        let _ = summary_event_tx
            .send(SummaryEvent::Error(format!(
                "Failed to send content message: {}",
                e
            )))
            .await;
        active.store(false, Ordering::Release);
        return;
    }

    info!("Audio Summary: setup complete, waiting for audio response stream...");

    let has_audio = Arc::new(AtomicBool::new(false));
    let mut server_error_message = None;

    while active.load(Ordering::Acquire) {
        let msg = match read.next().await {
            Some(Ok(m)) => m,
            Some(Err(e)) => {
                error!("Audio Summary: WebSocket read error: {}", e);
                server_error_message = Some(format!("WebSocket read error: {}", e));
                break;
            }
            None => {
                info!("Audio Summary: WebSocket read returned None");
                break;
            }
        };

        if let tokio_tungstenite::tungstenite::Message::Close(cf) = &msg {
            info!("Audio Summary: WebSocket Close frame received: {:?}", cf);
            if let Some(frame) = cf {
                if !frame.reason.is_empty() {
                    server_error_message = Some(frame.reason.to_string());
                }
            }
        } else {
            info!("Audio Summary: WebSocket message received: {:?}", msg);
        }

        let text_opt = match &msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => Some(text.clone()),
            tokio_tungstenite::tungstenite::Message::Binary(bin) => {
                String::from_utf8(bin.clone()).ok()
            }
            _ => None,
        };

        if let Some(text) = text_opt {
            let json: Result<serde_json::Value, _> = serde_json::from_str(&text);
            if let Ok(json_val) = json {
                if let Some(err) = json_val
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    error!("Audio Summary: server returned error: {}", err);
                    server_error_message = Some(err.to_string());
                    break;
                }

                // Look for inline output audio data
                if let Some(server_content) = json_val
                    .get("serverContent")
                    .or_else(|| json_val.get("server_content"))
                {
                    if let Some(model_turn) = server_content
                        .get("modelTurn")
                        .or_else(|| server_content.get("model_turn"))
                    {
                        if let Some(parts) = model_turn.get("parts").and_then(|p| p.as_array()) {
                            for part in parts {
                                if let Some(inline_data) =
                                    part.get("inlineData").or_else(|| part.get("inline_data"))
                                {
                                    if let Some(mime_type) = inline_data
                                        .get("mimeType")
                                        .or_else(|| inline_data.get("mime_type"))
                                        .and_then(|m| m.as_str())
                                    {
                                        if mime_type.starts_with("audio/pcm") {
                                            if let Some(base64_data) =
                                                inline_data.get("data").and_then(|d| d.as_str())
                                            {
                                                if let Ok(raw_bytes) =
                                                    base64::prelude::BASE64_STANDARD
                                                        .decode(base64_data)
                                                {
                                                    has_audio.store(true, Ordering::Release);
                                                    let mut raw_samples = Vec::new();
                                                    for chunk in raw_bytes.chunks_exact(2) {
                                                        let sample_i16 = i16::from_le_bytes([
                                                            chunk[0], chunk[1],
                                                        ]);
                                                        let sample_f32 =
                                                            sample_i16 as f32 / i16::MAX as f32;
                                                        raw_samples.push(sample_f32);
                                                    }

                                                    let resampled =
                                                        crate::extensions::speech::resample(
                                                            &raw_samples,
                                                            24000,
                                                            output_sample_rate,
                                                        );
                                                    if let Ok(mut buf) = audio_buffer.lock() {
                                                        buf.extend(resampled);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Check if turn complete
                    let turn_complete = server_content
                        .get("turnComplete")
                        .or_else(|| server_content.get("turn_complete"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if turn_complete {
                        info!("Audio Summary: generation turn complete.");
                        break;
                    }
                }
            }
        }

        if let tokio_tungstenite::tungstenite::Message::Close(_) = msg {
            break;
        }
    }

    let was_cancelled = !active.load(Ordering::Acquire);

    if !was_cancelled {
        if let Some(err) = server_error_message {
            active.store(false, Ordering::Release);
            let _ = summary_event_tx.send(SummaryEvent::Error(err)).await;
        } else if !has_audio.load(Ordering::Acquire) {
            active.store(false, Ordering::Release);
            let _ = summary_event_tx
                .send(SummaryEvent::Error(
                    "WebSocket connection closed by server before audio was received.".to_string(),
                ))
                .await;
        } else {
            wait_for_audio_buffer_to_drain(
                &audio_buffer,
                &active,
                std::time::Duration::from_secs(30),
            )
            .await;
            active.store(false, Ordering::Release);
            let _ = summary_event_tx.send(SummaryEvent::Finished).await;
        }
    }
}

async fn wait_for_summary_setup_complete<R>(read: &mut R) -> Result<(), String>
where
    R: futures_util::Stream<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    while let Some(msg) = read.next().await {
        let msg = msg.map_err(|e| format!("WebSocket read error before setup completed: {e}"))?;
        let text_opt = match msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => Some(text),
            tokio_tungstenite::tungstenite::Message::Binary(bin) => String::from_utf8(bin).ok(),
            tokio_tungstenite::tungstenite::Message::Close(frame) => {
                return Err(frame
                    .and_then(|frame| (!frame.reason.is_empty()).then(|| frame.reason.to_string()))
                    .unwrap_or_else(|| "WebSocket closed before setup completed.".to_string()));
            }
            _ => None,
        };

        if let Some(text) = text_opt {
            if let Some(err) = live_error_message(&text) {
                return Err(err);
            }
            if is_setup_complete_frame(&text) {
                return Ok(());
            }
        }
    }

    Err("WebSocket closed before setup completed.".to_string())
}

fn live_error_message(json_str: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;
    json.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(str::to_string)
}

fn is_setup_complete_frame(json_str: &str) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return false;
    };

    json.get("setupComplete")
        .or_else(|| json.get("setup_complete"))
        .is_some()
}

async fn wait_for_audio_buffer_to_drain(
    audio_buffer: &Arc<Mutex<VecDeque<f32>>>,
    active: &Arc<AtomicBool>,
    timeout: std::time::Duration,
) {
    let start = std::time::Instant::now();
    while active.load(Ordering::Acquire) && start.elapsed() < timeout {
        let empty = audio_buffer
            .lock()
            .map(|buf| buf.is_empty())
            .unwrap_or(true);
        if empty {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::{SampleFormat, SampleRate, SupportedBufferSize};

    #[test]
    fn stereo_output_config_prefers_stereo_when_default_is_mono() {
        let default_config = cpal::SupportedStreamConfig::new(
            1,
            SampleRate(48_000),
            SupportedBufferSize::Unknown,
            SampleFormat::F32,
        );
        let ranges = vec![
            cpal::SupportedStreamConfigRange::new(
                1,
                SampleRate(44_100),
                SampleRate(48_000),
                SupportedBufferSize::Unknown,
                SampleFormat::F32,
            ),
            cpal::SupportedStreamConfigRange::new(
                2,
                SampleRate(44_100),
                SampleRate(48_000),
                SupportedBufferSize::Unknown,
                SampleFormat::F32,
            ),
        ];

        let config = stereo_output_config_from_ranges(&default_config, ranges)
            .expect("stereo config should be selected");

        assert_eq!(config.channels(), 2);
        assert_eq!(config.sample_rate(), SampleRate(48_000));
        assert_eq!(config.sample_format(), SampleFormat::F32);
    }

    #[test]
    fn stereo_output_config_clamps_default_sample_rate_to_supported_range() {
        let default_config = cpal::SupportedStreamConfig::new(
            1,
            SampleRate(48_000),
            SupportedBufferSize::Unknown,
            SampleFormat::F32,
        );
        let ranges = vec![cpal::SupportedStreamConfigRange::new(
            2,
            SampleRate(24_000),
            SampleRate(44_100),
            SupportedBufferSize::Unknown,
            SampleFormat::F32,
        )];

        let config = stereo_output_config_from_ranges(&default_config, ranges)
            .expect("stereo config should be selected");

        assert_eq!(config.channels(), 2);
        assert_eq!(config.sample_rate(), SampleRate(44_100));
    }

    #[test]
    fn stereo_output_config_returns_none_without_stereo_support() {
        let default_config = cpal::SupportedStreamConfig::new(
            1,
            SampleRate(48_000),
            SupportedBufferSize::Unknown,
            SampleFormat::F32,
        );
        let ranges = vec![cpal::SupportedStreamConfigRange::new(
            1,
            SampleRate(44_100),
            SampleRate(48_000),
            SupportedBufferSize::Unknown,
            SampleFormat::F32,
        )];

        assert!(stereo_output_config_from_ranges(&default_config, ranges).is_none());
    }

    #[test]
    fn setup_complete_frame_accepts_camel_and_snake_case() {
        assert!(is_setup_complete_frame(r#"{"setupComplete": {}}"#));
        assert!(is_setup_complete_frame(r#"{"setup_complete": {}}"#));
        assert!(!is_setup_complete_frame(
            r#"{"serverContent": {"turnComplete": true}}"#
        ));
        assert!(!is_setup_complete_frame("{invalid}"));
    }

    #[test]
    fn live_error_message_extracts_server_error() {
        assert_eq!(
            live_error_message(r#"{"error": {"message": "bad request"}}"#),
            Some("bad request".to_string())
        );
        assert_eq!(live_error_message(r#"{"setupComplete": {}}"#), None);
    }

    #[tokio::test]
    async fn drain_wait_keeps_playback_active_until_queue_is_empty() {
        let audio_buffer = Arc::new(Mutex::new(VecDeque::from([0.5_f32])));
        let active = Arc::new(AtomicBool::new(true));
        let draining_buffer = audio_buffer.clone();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            draining_buffer.lock().expect("buffer lock").pop_front();
        });

        wait_for_audio_buffer_to_drain(
            &audio_buffer,
            &active,
            std::time::Duration::from_millis(500),
        )
        .await;

        assert!(
            active.load(Ordering::Acquire),
            "playback must remain active while queued audio drains"
        );
        assert!(audio_buffer.lock().expect("buffer lock").is_empty());
    }
}

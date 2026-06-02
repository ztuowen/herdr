use crate::speech::audio::{downmix_to_mono, resample};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

/// Transcription events produced by the Gemini Live streaming adapter.
#[derive(Debug)]
pub enum TranscriptionEvent {
    Partial(String),
    Finished(Result<String, String>),
}

/// Runs the main Gemini Live WebSocket streaming transcription loop.
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
    let model_name = model_name(model);
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

    let mut setup_complete = false;
    let mut finalized_text = String::new();
    let mut current_turn_text = String::new();
    let mut recording_stopped = false;
    let mut turn_completed = true;
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
                if !setup_complete {
                    continue;
                }

                if !recording_stopped {
                    if !recording_active.load(std::sync::atomic::Ordering::Acquire) {
                        recording_stopped = true;
                        stop_time = Some(std::time::Instant::now());

                        if let Some(media_msg) =
                            take_audio_media_message(&buffer, channels, sample_rate)
                        {
                            if let Err(e) = write
                                .send(tokio_tungstenite::tungstenite::Message::Text(media_msg))
                                .await
                            {
                                tracing::error!(
                                    "Speech to Text: failed to send final audio chunk: {}",
                                    e
                                );
                                break;
                            }
                            turn_completed = false;
                        }

                        if turn_completed {
                            break;
                        }

                        let turn_msg = serde_json::json!({
                            "realtimeInput": {
                                "audioStreamEnd": true
                            }
                        });

                        if let Err(e) = write
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                turn_msg.to_string(),
                            ))
                            .await
                        {
                            tracing::error!("Speech to Text: failed to send audioStreamEnd message: {}", e);
                        }
                        continue;
                    }

                    if let Some(media_msg) = take_audio_media_message(&buffer, channels, sample_rate) {
                        if let Err(e) = write
                            .send(tokio_tungstenite::tungstenite::Message::Text(media_msg))
                            .await
                        {
                            tracing::error!("Speech to Text: failed to send audio chunk: {}", e);
                            break;
                        }
                        turn_completed = false;
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
                    if is_setup_complete_frame(&text) {
                        setup_complete = true;
                        continue;
                    }

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
                            turn_completed = true;
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
                if t.elapsed() > tokio::time::Duration::from_secs(1) {
                    tracing::warn!(
                        "Speech to Text: grace period of 1s expired waiting for final transcription"
                    );
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

fn is_setup_complete_frame(json_str: &str) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return false;
    };

    json.get("setupComplete")
        .or_else(|| json.get("setup_complete"))
        .is_some()
}

fn encode_input_audio_chunk(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<u8> {
    let mono = downmix_to_mono(samples, channels);
    let resampled = resample(&mono, sample_rate, 16000);

    let mut raw_bytes = Vec::with_capacity(resampled.len() * 2);
    for &sample in &resampled {
        let clamped = sample.clamp(-1.0, 1.0);
        let scaled = (clamped * 32767.0) as i16;
        raw_bytes.extend_from_slice(&scaled.to_le_bytes());
    }
    raw_bytes
}

fn take_audio_media_message(
    buffer: &Arc<std::sync::Mutex<Vec<f32>>>,
    channels: u16,
    sample_rate: u32,
) -> Option<String> {
    let samples = match buffer.lock() {
        Ok(mut buf) => std::mem::take(&mut *buf),
        Err(_) => Vec::new(),
    };

    if samples.is_empty() {
        return None;
    }

    let raw_bytes = encode_input_audio_chunk(&samples, channels, sample_rate);
    let base64_audio = base64::prelude::BASE64_STANDARD.encode(&raw_bytes);

    Some(
        serde_json::json!({
            "realtimeInput": {
                "audio": {
                    "mimeType": "audio/pcm;rate=16000",
                    "data": base64_audio
                }
            }
        })
        .to_string(),
    )
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

fn model_name(model: String) -> String {
    if model.starts_with("models/") {
        model
    } else {
        format!("models/{}", model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_live_transcription_frame() {
        let msg = r#"{"serverContent": {"inputTranscription": {"text": "hello "}, "turnComplete": true}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("hello ".to_string(), true)));

        let msg = r#"{"server_content": {"input_transcription": {"text": "world"}, "turn_complete": false}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("world".to_string(), false)));

        let msg = r#"{"inputTranscription": {"text": "foo"}, "turnComplete": true}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("foo".to_string(), true)));

        let msg = r#"{"input_transcription": {"text": "bar"}, "turn_complete": false}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, Some(("bar".to_string(), false)));

        let msg = r#"{"error": {"message": "Invalid API key"}}"#;
        let res = parse_live_transcription_frame(msg);
        assert_eq!(res, None);

        assert_eq!(parse_live_transcription_frame(""), None);
        assert_eq!(parse_live_transcription_frame("{invalid}"), None);
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
    fn take_audio_media_message_drains_buffer_and_builds_audio_frame() {
        let buffer = Arc::new(std::sync::Mutex::new(vec![0.25, -0.25]));

        let message = take_audio_media_message(&buffer, 1, 16_000).unwrap();

        assert!(buffer.lock().unwrap().is_empty());
        let json: serde_json::Value = serde_json::from_str(&message).unwrap();
        let audio = json
            .get("realtimeInput")
            .and_then(|input| input.get("audio"))
            .unwrap();
        assert_eq!(
            audio.get("mimeType").and_then(|value| value.as_str()),
            Some("audio/pcm;rate=16000")
        );
        assert!(audio
            .get("data")
            .and_then(|value| value.as_str())
            .is_some_and(|data| !data.is_empty()));
    }

    #[test]
    fn take_audio_media_message_returns_none_for_empty_buffer() {
        let buffer = Arc::new(std::sync::Mutex::new(Vec::new()));

        assert!(take_audio_media_message(&buffer, 1, 16_000).is_none());
    }
}

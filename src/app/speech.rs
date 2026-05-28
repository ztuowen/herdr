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
        self.recording_buffer = Some(buffer);
        self.recording_sample_rate = sample_rate;
        self.recording_channels = channels;
        self.recording_key = Some(key);
        self.state.recording_workspace = Some(workspace_id);

        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "Speech to Text".into(),
            context: "Listening...".into(),
            target: None,
        });

        true
    }

    pub(crate) fn stop_recording(&mut self) -> Option<(Vec<f32>, u32, u16)> {
        self.recording_stream = None;
        let buffer = self.recording_buffer.take()?;
        let sample_rate = self.recording_sample_rate;
        let channels = self.recording_channels;
        self.recording_key = None;
        self.state.recording_workspace = None;

        let samples = match buffer.lock() {
            Ok(buf) => buf.clone(),
            Err(_) => return None,
        };

        Some((samples, sample_rate, channels))
    }

    pub(super) fn trigger_transcription(
        &mut self,
        workspace_id: String,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        api_key: String,
    ) {
        let duration_secs = samples.len() as f32 / (sample_rate as f32 * channels as f32);
        let previous_toast = self.state.toast.clone();
        if duration_secs < 0.3 {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Speech to Text".into(),
                context: "Recording too short.".into(),
                target: None,
            });
            self.sync_toast_deadline(previous_toast);
            return;
        }

        let mut samples = samples;
        if duration_secs > 120.0 {
            let max_samples = (120.0 * sample_rate as f32 * channels as f32) as usize;
            samples.truncate(max_samples);
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::NeedsAttention,
                title: "Speech to Text Warning".into(),
                context: "Recording truncated to 2 minutes.".into(),
                target: None,
            });
        } else {
            self.state.toast = Some(crate::app::state::ToastNotification {
                kind: crate::app::state::ToastKind::Finished,
                title: "Speech to Text".into(),
                context: "Transcribing...".into(),
                target: None,
            });
        }
        self.sync_toast_deadline(previous_toast);

        let event_tx = self.event_tx.clone();
        let model = self
            .state
            .speech_to_text
            .model
            .clone()
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());

        tracing::info!(
            "Speech to text: starting transcription. workspace_id={}, samples={}, sample_rate={}, channels={}, model={}",
            workspace_id,
            samples.len(),
            sample_rate,
            channels,
            model
        );

        std::thread::spawn(move || {
            let mono = downmix_to_mono(&samples, channels);
            let resampled = resample(&mono, sample_rate, 16000);
            let wav_bytes = create_wav_data(&resampled, 16000);

            tracing::info!(
                "Speech to text: sending request to Gemini API (WAV size: {} bytes)",
                wav_bytes.len()
            );
            let result = request_gemini_transcription(&wav_bytes, &model, &api_key);
            tracing::info!(
                "Speech to text: API response result: {:?}",
                result.as_ref().map(|s| s.len())
            );

            let event = crate::events::AppEvent::SpeechTranscribed {
                workspace_id,
                result,
            };
            let _ = event_tx.blocking_send(event);
        });
    }
}

fn write_input_data<T>(data: &[T], buffer: &Arc<std::sync::Mutex<Vec<f32>>>)
where
    T: cpal::Sample<Float = f32>,
{
    if let Ok(mut buf) = buffer.lock() {
        for &sample in data {
            buf.push(sample.to_float_sample());
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

fn create_wav_data(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(file_size as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_size as u32).to_le_bytes());

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let scaled = (clamped * 32767.0) as i16;
        wav.extend_from_slice(&scaled.to_le_bytes());
    }

    wav
}

fn request_gemini_transcription(
    wav_bytes: &[u8],
    model: &str,
    api_key: &str,
) -> Result<String, String> {
    use base64::Engine;
    let base64_audio = base64::prelude::BASE64_STANDARD.encode(wav_bytes);

    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "inlineData": {
                        "mimeType": "audio/wav",
                        "data": base64_audio
                    }
                },
                {
                    "text": "Please provide an accurate transcription of the audio. Output only the transcription, nothing else."
                }
            ]
        }]
    });

    let payload_str = payload.to_string();

    tracing::info!("Speech to text: spawning curl to post audio to Gemini API...");
    let mut child = std::process::Command::new("curl")
        .arg("-sS")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg("30")
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("x-goog-api-key: {}", api_key))
        .arg("--data-binary")
        .arg("@-")
        .arg(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        ))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            tracing::error!("Speech to text: failed to spawn curl: {}", e);
            format!("Failed to spawn curl: {}", e)
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        tracing::debug!("Speech to text: writing base64 payload to curl stdin...");
        stdin.write_all(payload_str.as_bytes()).map_err(|e| {
            tracing::error!("Speech to text: failed to write payload to curl: {}", e);
            format!("Failed to write to curl stdin: {}", e)
        })?;
    }

    tracing::debug!("Speech to text: waiting for curl process output...");
    let output = child.wait_with_output().map_err(|e| {
        tracing::error!("Speech to text: failed to wait on curl: {}", e);
        format!("Failed to wait on curl: {}", e)
    })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        tracing::error!(
            "Speech to text: curl process failed. exit_code={:?}, stderr={}",
            output.status.code(),
            err_msg
        );
        return Err(format!("curl failed: {}", err_msg));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    if let Some(text) = parse_gemini_transcription(&output_str) {
        if text.trim().is_empty() {
            Err("No speech detected / transcription empty.".to_string())
        } else {
            Ok(text)
        }
    } else {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            if let Some(err) = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                return Err(format!("API Error: {}", err));
            }
        }
        Err("Failed to parse Gemini API response.".to_string())
    }
}

fn parse_gemini_transcription(output_str: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(output_str).ok()?;
    let text = json
        .get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .get(0)?
        .get("text")?
        .as_str()?;
    Some(text.trim().to_string())
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
}

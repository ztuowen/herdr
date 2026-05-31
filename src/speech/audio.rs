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

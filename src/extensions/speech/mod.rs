pub mod app;
pub mod audio;
pub(crate) mod client;
pub mod events;
pub mod gemini;
pub(crate) mod hooks;
pub mod input;
pub mod pipeline;
pub(crate) mod server;
pub mod summary;

pub use audio::resample;
pub use gemini::run_gemini_postprocess;
pub use pipeline::{
    model_or_default, postprocess_instruction, SpeechRecorder, TranscriptionPipeline,
};

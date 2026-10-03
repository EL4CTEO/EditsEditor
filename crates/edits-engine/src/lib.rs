//! # edits-engine
//!
//! The EditsEditor engine: an editing session over a project document with undo history,
//! media management, music/scene analysis, per-frame evaluation on the GPU, Rhai expressions
//! and scripting, presets, audio mixdown and export. The MCP server, CLI and viewer are thin
//! layers over this crate.

pub mod audio_mix;
pub mod engine;
pub mod eval;
pub mod export;
pub mod expr;
pub mod media_pool;
pub mod script;

pub use edits_audio::AnalysisOptions;
pub use edits_media::{ExportSettings, scenes::SceneOptions};
pub use edits_render::OutputMode;
pub use engine::{Engine, ImportOptions, ImportedAsset, RenderedFrame, ScriptReport};
pub use export::{ExportReport, Progress};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Edit(#[from] edits_core::EditError),
    #[error(transparent)]
    Media(#[from] edits_media::MediaError),
    #[error(transparent)]
    Render(#[from] edits_render::RenderError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
    #[error("cancelled")]
    Cancelled,
}

pub type Result<T, E = EngineError> = std::result::Result<T, E>;

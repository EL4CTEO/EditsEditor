//! # edits-render
//!
//! GPU compositor built on wgpu: layer placement with 2.5D transforms and motion blur,
//! 29 blend modes, SDF masks, track mattes, multi-pass WGSL effects/transitions/generators,
//! pooled textures and pipelined readback. Also CPU text (cosmic-text) and vector shape
//! (kurbo + tiny-skia) rasterization.

pub mod gpu;
pub mod renderer;
pub mod shape;
pub mod text;
pub mod transform;

pub use gpu::{GpuContext, GpuOptions};
pub use renderer::{EffectCall, FrameCtx, Globals, MaskParams, OutputMode, PendingReadback, Renderer, Tex};
pub use text::{TextFrameParams, TextRenderer};
pub use transform::PlaceParams;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("gpu: {0}")]
    Gpu(String),
    #[error("shader: {0}")]
    Shader(String),
}

pub type Result<T, E = RenderError> = std::result::Result<T, E>;

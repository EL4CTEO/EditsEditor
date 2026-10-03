//! # edits-core
//!
//! The document model of EditsEditor: projects, compositions, tracks, clips, effects,
//! animatable properties (keyframes + expressions), easing, edit operations and undo history.
//! This crate has no GPU, media or scripting dependencies.

pub mod color;
pub mod easing;
pub mod history;
pub mod math;
pub mod model;
pub mod ops;
pub mod property;
pub mod query;
pub mod summary;
pub mod validate;
pub mod value;

pub use color::Color;
pub use easing::Easing;
pub use math::{Mat4, Vec2};
pub use model::*;
pub use property::{Animated, Evaluator, Keyframe, LoopMode, NoExpr, Property};
pub use value::{Animatable, Value};

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = EditError> = std::result::Result<T, E>;

/// JSON schema of the project file format.
pub fn project_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(Project)).unwrap_or_default()
}

/// JSON schema of a single named sub-type (clip, effect, transition, mask, text, shape, ...).
pub fn type_schema(name: &str) -> Option<serde_json::Value> {
    use schemars::schema_for;
    let s = match name.to_ascii_lowercase().as_str() {
        "project" => schema_for!(Project),
        "composition" | "comp" => schema_for!(Composition),
        "track" => schema_for!(Track),
        "clip" => schema_for!(Clip),
        "source" | "clip_source" => schema_for!(ClipSource),
        "transform" => schema_for!(Transform),
        "effect" | "effect_instance" => schema_for!(EffectInstance),
        "transition" => schema_for!(Transition),
        "mask" => schema_for!(Mask),
        "matte" => schema_for!(Matte),
        "text" => schema_for!(TextSource),
        "text_animator" | "animator" => schema_for!(TextAnimator),
        "shape" => schema_for!(ShapeSource),
        "audio" | "clip_audio" => schema_for!(ClipAudio),
        "audio_effect" => schema_for!(AudioEffect),
        "asset" => schema_for!(Asset),
        "timing" => schema_for!(Timing),
        "easing" => schema_for!(Easing),
        "blend_mode" => schema_for!(BlendMode),
        "echo" => schema_for!(Echo),
        "marker" => schema_for!(Marker),
        "property" => schema_for!(Property<Value>),
        _ => return None,
    };
    serde_json::to_value(s).ok()
}

pub const SCHEMA_TYPES: &[&str] = &[
    "project", "composition", "track", "clip", "source", "transform", "effect", "transition", "mask", "matte",
    "text", "text_animator", "shape", "audio", "audio_effect", "asset", "timing", "easing", "blend_mode", "echo",
    "marker", "property",
];

#[cfg(test)]
mod tests {
    #[test]
    fn schemas_generate() {
        let s = super::project_schema();
        assert!(s.to_string().len() > 1000);
        for t in super::SCHEMA_TYPES {
            assert!(super::type_schema(t).is_some(), "{t}");
        }
    }
}

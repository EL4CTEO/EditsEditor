//! # edits-audio
//!
//! Music analysis for beat-synced editing (tempo, beats, downbeats, drops, sections, accents,
//! band envelopes) and an offline mixer with time mapping, automation and effects.

pub mod analysis;
pub mod mix;
pub mod waveform;

pub use analysis::{AnalysisOptions, AudioAnalysis, Envelope, SectionInfo, analyze};
pub use mix::{AudioFx, MixClip, mix};

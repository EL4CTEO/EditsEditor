//! id = "beat_bump"
//! name = "Beat Bump"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Scale bumps on every beat via expression (always in sync, no keyframes)."
//! tags = ["beat", "bump", "pulse", "zoom"]
//! params = [
//!   { name = "amount", type = "float", default = 0.08 },
//!   { name = "decay", type = "float", default = 9.0 },
//! ]
clip.set_path("transform.scale", expr("value * (1.0 + " + args.amount + " * pulse(" + args.decay + "))", 1.0));

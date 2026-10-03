//! id = "audio_glow"
//! name = "Audio Glow"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Deep glow intensity driven by loudness and beat pulses."
//! tags = ["glow", "audio", "reactive", "beat"]
//! params = [
//!   { name = "base", type = "float", default = 0.4 },
//!   { name = "amount", type = "float", default = 1.5 },
//! ]
clip.add_fx(fx("deep_glow", #{ intensity: expr(args.base + " + " + args.amount + " * (0.6 * level() + 0.4 * pulse(8.0))") }));

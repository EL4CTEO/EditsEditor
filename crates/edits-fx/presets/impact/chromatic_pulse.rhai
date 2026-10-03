//! id = "chromatic_pulse"
//! name = "Chromatic Pulse"
//! scope = "clip"
//! category = "impact"
//! description = "Lens chromatic aberration breathing with the beat (expression-driven, follows the music live)."
//! tags = ["chromatic", "pulse", "beat", "lens"]
//! params = [
//!   { name = "amount", type = "float", default = 0.03 },
//!   { name = "decay", type = "float", default = 8.0 },
//! ]
clip.add_fx(fx("chromatic_aberration", #{ amount: expr("0.002 + " + args.amount + " * pulse(" + args.decay + ")") }));

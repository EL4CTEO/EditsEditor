//! id = "rgb_split_beats"
//! name = "RGB Split on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "RGB split pulses on beats (keyframed, deterministic)."
//! tags = ["rgb split", "beat", "glitch", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "beats" },
//!   { name = "every", type = "int", default = 1 },
//!   { name = "amount", type = "float", default = 25.0 },
//!   { name = "release", type = "float", default = 0.2 },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no hits in clip"); return; }
clip.add_fx(fx("rgb_split", #{ amount: pulse_keys(ts, 0.0, args.amount, 0.0, args.release, "ease_out_expo") }));

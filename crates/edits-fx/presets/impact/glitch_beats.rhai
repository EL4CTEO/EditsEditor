//! id = "glitch_beats"
//! name = "Glitch on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "Short glitch bursts on hits."
//! tags = ["glitch", "beat", "digital"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "downbeats" },
//!   { name = "every", type = "int", default = 1 },
//!   { name = "intensity", type = "float", default = 0.7 },
//!   { name = "release", type = "float", default = 0.15 },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no hits in clip"); return; }
clip.add_fx(fx("glitch", #{ intensity: pulse_keys(ts, 0.0, args.intensity, 0.0, args.release, "linear"), speed: 24.0 }));

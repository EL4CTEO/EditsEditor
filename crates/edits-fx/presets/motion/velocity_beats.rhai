//! id = "velocity_beats"
//! name = "Velocity on Beats"
//! scope = "clip"
//! category = "motion"
//! description = "Speed surges on each beat and eases into slow motion until the next — rhythmic velocity edit."
//! tags = ["velocity", "beat", "speed", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "half"], default = "beats" },
//!   { name = "fast", type = "float", default = 3.0 },
//!   { name = "slow", type = "float", default = 0.3 },
//! ]
let ts = pick_times(ctx, args.on, 1);
if ts.len() == 0 { log("no beats in clip"); return; }
let ks = [[0.0, args.slow, "linear"]];
for t in ts { ks.push([t, args.fast, "ease_out_expo"]); ks.push([t + ctx.beat_len * 0.6, args.slow, "linear"]); }
clip.speed = keys(ks);

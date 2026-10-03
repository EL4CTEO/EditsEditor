//! id = "shake_beats"
//! name = "Shake on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "Short shakes on each beat/downbeat/drop inside the clip."
//! tags = ["shake", "beat", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "downbeats" },
//!   { name = "every", type = "int", default = 1 },
//!   { name = "amplitude", type = "float", default = 25.0 },
//!   { name = "release", type = "float", default = 0.3 },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no hits in clip"); return; }
clip.add_fx(fx("shake", #{ amplitude: pulse_keys(ts, 0.0, args.amplitude, 0.0, args.release, "ease_out_quad"), frequency: 16.0, smear: 0.5, scale_fill: 1.06, rotation: 2.0 }));

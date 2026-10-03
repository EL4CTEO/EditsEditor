//! id = "flash_beats"
//! name = "Flash on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "Flashes on beats, downbeats or drops inside the clip."
//! tags = ["flash", "beat", "strobe", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "downbeats" },
//!   { name = "every", type = "int", default = 1 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "intensity", type = "float", default = 0.8 },
//!   { name = "release", type = "float", default = 0.2 },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no hits in clip"); return; }
clip.add_fx(fx("flash", #{ amount: pulse_keys(ts, 0.0, args.intensity, 0.0, args.release, "ease_out_quad"), color: args.color, exposure: 1.5 }));

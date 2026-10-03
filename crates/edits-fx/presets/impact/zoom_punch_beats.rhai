//! id = "zoom_punch_beats"
//! name = "Zoom Punch on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "Scale punch on every beat / downbeat / drop inside the clip (requires project timing)."
//! tags = ["zoom", "beat", "pulse", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "beats" },
//!   { name = "every", type = "int", default = 1, desc = "Use every Nth hit" },
//!   { name = "strength", type = "float", default = 0.12 },
//!   { name = "release", type = "float", default = 0.25 },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no " + args.on + " inside this clip — run analyze_audio with apply=true"); return; }
clip.add_fx(fx("transform", #{ scale: pulse_keys(ts, 1.0, 1.0 + args.strength, 0.0, args.release, "ease_out_expo"), edge: "mirror" }));

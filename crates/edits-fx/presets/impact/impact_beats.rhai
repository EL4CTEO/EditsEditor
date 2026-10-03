//! id = "impact_beats"
//! name = "Impact on Downbeats"
//! scope = "clip"
//! category = "impact"
//! description = "Impact combo on every downbeat (or drop) inside the clip, lighter pulses on the other beats."
//! tags = ["impact", "beat", "combo", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["downbeats", "drops", "beats", "bars2"], default = "downbeats" },
//!   { name = "intensity", type = "float", default = 0.8 },
//!   { name = "beat_pulse", type = "float", default = 0.06, desc = "Zoom pulse on remaining beats (0 = off)" },
//! ]
let hits = pick_times(ctx, args.on, 1);
if hits.len() == 0 { log("no hits in clip"); return; }
let k = args.intensity;
let zoom_times = hits;
clip.add_fx(fx("transform", #{ scale: pulse_keys(zoom_times, 1.0, 1.0 + 0.25 * k, 0.0, 0.35, "ease_out_expo"), edge: "mirror" }));
if args.beat_pulse > 0.0 {
    clip.add_fx(fx("transform", #{ scale: pulse_keys(ctx.beats, 1.0, 1.0 + args.beat_pulse, 0.0, 0.2, "ease_out_expo"), edge: "mirror" }));
}
clip.add_fx(fx("rgb_split", #{ amount: pulse_keys(hits, 0.0, 25.0 * k, 0.0, 0.25, "ease_out_expo") }));
clip.add_fx(fx("shake", #{ amplitude: pulse_keys(hits, 0.0, 30.0 * k, 0.0, 0.4, "ease_out_quad"), frequency: 18.0, smear: 0.5, scale_fill: 1.08 }));
clip.add_fx(fx("flash", #{ amount: pulse_keys(hits, 0.0, 0.7 * k, 0.0, 0.18, "ease_out_quad"), exposure: 1.5 }));

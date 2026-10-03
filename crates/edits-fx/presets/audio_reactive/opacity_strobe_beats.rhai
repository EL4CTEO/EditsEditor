//! id = "beat_blink"
//! name = "Beat Blink"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Clip blinks on/off with the beat (alternating beats or short dips)."
//! tags = ["blink", "opacity", "beat", "strobe"]
//! params = [ { name = "mode", type = "enum", options = ["alternate", "dip"], default = "dip" } ]
if args.mode == "alternate" { clip.opacity = expr("1.0 - beat_toggle()"); }
else { clip.opacity = expr("1.0 - 0.85 * pulse(14.0)"); }

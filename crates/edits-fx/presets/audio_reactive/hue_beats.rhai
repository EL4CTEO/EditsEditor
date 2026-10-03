//! id = "hue_beats"
//! name = "Hue Jump on Beats"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Shifts hue by a step on every beat (psychedelic color cycling locked to music)."
//! tags = ["hue", "color", "beat", "psychedelic"]
//! params = [ { name = "step", type = "float", default = 45.0, desc = "Degrees per beat" } ]
clip.add_fx(fx("hue_saturation", #{ hue: expr("fmod(max(beat(), 0.0) * " + args.step + ", 360.0)"), saturation: 1.2 }));

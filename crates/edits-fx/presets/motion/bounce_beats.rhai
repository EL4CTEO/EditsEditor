//! id = "bounce_beats"
//! name = "Bounce on Beats"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Hops up on every beat and falls back (great for stickers, chibis, text)."
//! tags = ["bounce", "hop", "beat", "cute"]
//! params = [ { name = "height", type = "float", default = 30.0 } ]
clip.set_path("transform.position", expr("add(value, [0.0, -" + args.height + " * sin(beat_phase() * 3.14159)])", [0.0, 0.0]));

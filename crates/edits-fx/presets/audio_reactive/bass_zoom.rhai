//! id = "bass_zoom"
//! name = "Bass Zoom"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Scale follows the bass energy of the music."
//! tags = ["bass", "audio", "zoom", "reactive"]
//! params = [ { name = "amount", type = "float", default = 0.12 } ]
clip.set_path("transform.scale", expr("value * (1.0 + " + args.amount + " * bass())", 1.0));

//! id = "beat_rotate"
//! name = "Beat Rotate"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Snaps rotation left/right alternately on each beat."
//! tags = ["rotate", "beat", "alternate", "amv"]
//! params = [ { name = "angle", type = "float", default = 4.0 } ]
clip.set_path("transform.rotation", expr("(beat_toggle() * 2.0 - 1.0) * " + args.angle + " * (0.4 + 0.6 * pulse(6.0))"));
clip.set_path("transform.scale", 1.06);

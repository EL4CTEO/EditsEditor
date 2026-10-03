//! id = "fade_in"
//! name = "Fade In"
//! scope = "clip"
//! category = "entrance"
//! description = "Opacity fade at the start of the clip."
//! tags = ["fade", "in", "entrance"]
//! params = [ { name = "duration", type = "float", default = 0.4 }, { name = "ease", type = "string", default = "ease_out_cubic" } ]
clip.set_kf("opacity", 0.0, 0.0, args.ease);
clip.set_kf("opacity", args.duration, 1.0, "linear");

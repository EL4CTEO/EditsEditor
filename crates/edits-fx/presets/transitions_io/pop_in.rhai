//! id = "pop_in"
//! name = "Pop In"
//! scope = "clip"
//! category = "entrance"
//! description = "Springy scale-up from nothing (stickers, titles, characters)."
//! tags = ["pop", "scale", "entrance", "bounce", "cute"]
//! params = [ { name = "duration", type = "float", default = 0.45 }, { name = "overshoot", type = "float", default = 2.2 } ]
clip.set_path("transform.scale", keys([[0.0, 0.0, #{ back: args.overshoot }], [args.duration, 1.0]]));
clip.set_kf("opacity", 0.0, 0.0, "ease_out_quad");
clip.set_kf("opacity", args.duration * 0.4, 1.0, "linear");

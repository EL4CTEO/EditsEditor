//! id = "blur_in"
//! name = "Blur In"
//! scope = "clip"
//! category = "entrance"
//! description = "Comes into focus from a heavy blur."
//! tags = ["blur", "focus", "entrance", "soft"]
//! params = [ { name = "duration", type = "float", default = 0.6 }, { name = "radius", type = "float", default = 40.0 } ]
clip.add_fx(fx("gaussian_blur", #{ radius: keys([[0.0, args.radius, "ease_out_cubic"], [args.duration, 0.0]]) }));
clip.set_kf("opacity", 0.0, 0.0, "linear");
clip.set_kf("opacity", args.duration * 0.5, 1.0, "linear");

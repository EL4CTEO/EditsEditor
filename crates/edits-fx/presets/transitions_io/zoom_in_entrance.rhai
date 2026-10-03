//! id = "zoom_in_entrance"
//! name = "Zoom In Entrance"
//! scope = "clip"
//! category = "entrance"
//! description = "Starts huge and blurred, slams into place."
//! tags = ["zoom", "entrance", "slam", "blur"]
//! params = [ { name = "duration", type = "float", default = 0.4 }, { name = "from_scale", type = "float", default = 3.0 } ]
clip.set_path("transform.scale", keys([[0.0, args.from_scale, "ease_out_expo"], [args.duration, 1.0]]));
clip.add_fx(fx("radial_blur", #{ amount: keys([[0.0, 0.4, "ease_out_quad"], [args.duration, 0.0]]) }));
clip.set_kf("opacity", 0.0, 0.0, "linear");
clip.set_kf("opacity", args.duration * 0.3, 1.0, "linear");

//! id = "spin_in"
//! name = "Spin In"
//! scope = "clip"
//! category = "entrance"
//! description = "Spins and scales into place."
//! tags = ["spin", "rotate", "entrance"]
//! params = [ { name = "duration", type = "float", default = 0.6 }, { name = "turns", type = "float", default = 0.5 } ]
clip.set_path("transform.rotation", keys([[0.0, -360.0 * args.turns, "ease_out_cubic"], [args.duration, 0.0]]));
clip.set_path("transform.scale", keys([[0.0, 0.2, "ease_out_back"], [args.duration, 1.0]]));
clip.motion_blur = true;

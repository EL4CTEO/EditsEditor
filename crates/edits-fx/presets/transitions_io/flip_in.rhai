//! id = "flip_in"
//! name = "3D Flip In"
//! scope = "clip"
//! category = "entrance"
//! description = "Card flips in around the Y axis with perspective."
//! tags = ["flip", "3d", "card", "entrance"]
//! params = [ { name = "duration", type = "float", default = 0.6 }, { name = "axis", type = "enum", options = ["y", "x"], default = "y" } ]
let path = if args.axis == "x" { "transform.rotation_x" } else { "transform.rotation_y" };
clip.set_path(path, keys([[0.0, 90.0, "ease_out_back"], [args.duration, 0.0]]));

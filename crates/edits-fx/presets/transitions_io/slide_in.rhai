//! id = "slide_in"
//! name = "Slide In"
//! scope = "clip"
//! category = "entrance"
//! description = "Slides in from a direction with a snappy ease and motion blur."
//! tags = ["slide", "entrance", "move", "whip"]
//! params = [
//!   { name = "from", type = "enum", options = ["left", "right", "up", "down"], default = "left" },
//!   { name = "distance", type = "float", default = 0.0, desc = "Pixels (0 = full frame)" },
//!   { name = "duration", type = "float", default = 0.45 },
//!   { name = "ease", type = "string", default = "ease_out_expo" },
//! ]
let dist = if args.distance > 0.0 { args.distance } else { if args.from == "left" || args.from == "right" { ctx.width } else { ctx.height } };
let v = dir_vec(args.from, dist);
clip.set_path("transform.position", keys([[0.0, v, args.ease], [args.duration, [0.0, 0.0]]]));
clip.motion_blur = true;

//! id = "slide_out"
//! name = "Slide Out"
//! scope = "clip"
//! category = "exit"
//! description = "Slides out toward a direction at the end of the clip."
//! tags = ["slide", "exit", "move"]
//! params = [
//!   { name = "to", type = "enum", options = ["left", "right", "up", "down"], default = "right" },
//!   { name = "duration", type = "float", default = 0.4 },
//!   { name = "ease", type = "string", default = "ease_in_expo" },
//! ]
let dist = if args.to == "left" || args.to == "right" { ctx.width } else { ctx.height };
let d = ctx.clip_duration;
clip.set_path("transform.position", keys([[max(d - args.duration, 0.0), [0.0, 0.0], args.ease], [d, dir_vec(args.to, dist)]]));
clip.motion_blur = true;

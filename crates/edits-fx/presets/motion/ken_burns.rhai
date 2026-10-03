//! id = "ken_burns"
//! name = "Ken Burns"
//! scope = "clip"
//! category = "motion"
//! description = "Slow zoom and drift across the clip — brings stills and calm shots to life."
//! tags = ["ken burns", "zoom", "pan", "still", "documentary"]
//! params = [
//!   { name = "zoom", type = "float", default = 0.15 },
//!   { name = "direction", type = "enum", options = ["left", "right", "up", "down"], default = "right" },
//!   { name = "distance", type = "float", default = 40.0 },
//!   { name = "zoom_out", type = "bool", default = false },
//! ]
let d = ctx.clip_duration;
let a = if args.zoom_out { 1.0 + args.zoom } else { 1.0 };
let b = if args.zoom_out { 1.0 } else { 1.0 + args.zoom };
clip.set_path("transform.scale", keys([[0.0, a, "ease_in_out_sine"], [d, b]]));
let v = dir_vec(args.direction, args.distance);
clip.set_path("transform.position", keys([[0.0, [-v[0] / 2.0, -v[1] / 2.0], "ease_in_out_sine"], [d, [v[0] / 2.0, v[1] / 2.0]]]));

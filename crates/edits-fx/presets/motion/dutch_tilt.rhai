//! id = "dutch_tilt"
//! name = "Dutch Tilt"
//! scope = "clip"
//! category = "motion"
//! description = "Rotates into a tilted angle (unease, tension) with a slight zoom to hide edges."
//! tags = ["dutch", "tilt", "rotate", "tension"]
//! params = [
//!   { name = "angle", type = "float", default = 12.0 },
//!   { name = "duration", type = "float", default = 0.5 },
//! ]
clip.set_path("transform.rotation", keys([[0.0, 0.0, "snap"], [args.duration, args.angle]]));
clip.set_path("transform.scale", keys([[0.0, 1.0, "snap"], [args.duration, 1.0 + abs(args.angle) / 60.0]]));

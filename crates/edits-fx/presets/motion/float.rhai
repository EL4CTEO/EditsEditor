//! id = "float"
//! name = "Float"
//! scope = "clip"
//! category = "motion"
//! description = "Dreamy floating drift (smooth noise on position and rotation)."
//! tags = ["float", "drift", "dreamy", "wiggle"]
//! params = [
//!   { name = "amount", type = "float", default = 12.0 },
//!   { name = "speed", type = "float", default = 0.6 },
//! ]
clip.set_path("transform.position", expr("add(value, wiggle2(" + args.speed + ", " + args.amount + "))", [0.0, 0.0]));
clip.set_path("transform.rotation", expr("wiggle(" + args.speed + " * 0.7, " + args.amount + " * 0.1)"));

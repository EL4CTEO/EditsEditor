//! id = "pendulum"
//! name = "Pendulum Swing"
//! scope = "clip"
//! category = "motion"
//! description = "Swings back and forth around the anchor (set an anchor offset to hang from the top)."
//! tags = ["pendulum", "swing", "rotate"]
//! params = [
//!   { name = "angle", type = "float", default = 10.0 },
//!   { name = "speed", type = "float", default = 1.0 },
//! ]
clip.set_path("transform.rotation", expr("sin(t * 6.2832 * " + args.speed + ") * " + args.angle));

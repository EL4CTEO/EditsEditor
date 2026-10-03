//! id = "orbit_3d"
//! name = "3D Orbit Sway"
//! scope = "clip"
//! category = "motion"
//! description = "Gentle 3D yaw/pitch sway with perspective — a fake camera orbit."
//! tags = ["3d", "orbit", "sway", "perspective", "parallax"]
//! params = [
//!   { name = "yaw", type = "float", default = 12.0, desc = "Degrees" },
//!   { name = "pitch", type = "float", default = 5.0 },
//!   { name = "speed", type = "float", default = 0.3, desc = "Cycles per second" },
//! ]
clip.set_path("transform.rotation_y", expr("sin(t * 6.2832 * " + args.speed + ") * " + args.yaw));
clip.set_path("transform.rotation_x", expr("cos(t * 6.2832 * " + args.speed + " * 0.7) * " + args.pitch));
clip.set_path("transform.scale", 1.12);

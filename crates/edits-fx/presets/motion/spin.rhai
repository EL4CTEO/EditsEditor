//! id = "spin"
//! name = "Spin"
//! scope = "clip"
//! category = "motion"
//! description = "Continuous rotation over the clip."
//! tags = ["spin", "rotate", "loop"]
//! params = [
//!   { name = "turns", type = "float", default = 1.0 },
//!   { name = "ease", type = "string", default = "linear" },
//! ]
clip.set_path("transform.rotation", keys([[0.0, 0.0, args.ease], [ctx.clip_duration, 360.0 * args.turns]]));

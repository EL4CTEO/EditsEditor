//! id = "velocity_ramp"
//! name = "Velocity Ramp"
//! scope = "clip"
//! category = "motion"
//! description = "Classic velocity edit: fast → slow-motion → fast, with smooth speed curves (source time is the integral of speed)."
//! tags = ["velocity", "speed ramp", "slow motion", "amv", "twixtor"]
//! params = [
//!   { name = "fast", type = "float", default = 2.5, desc = "Speed at the start/end" },
//!   { name = "slow", type = "float", default = 0.25, desc = "Speed at the slowest point" },
//!   { name = "pivot", type = "float", default = 0.4, desc = "Where the slowest point is (fraction of clip)" },
//!   { name = "ease", type = "string", default = "whip" },
//! ]
let d = ctx.clip_duration;
let p = clamp(args.pivot, 0.05, 0.95) * d;
clip.speed = keys([[0.0, args.fast, args.ease], [p, args.slow, args.ease], [d, args.fast]]);

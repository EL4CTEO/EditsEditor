//! id = "push_in"
//! name = "Push In"
//! scope = "clip"
//! category = "motion"
//! description = "Steady dolly-in (scale up over the clip). Set `amount` negative to pull out."
//! tags = ["push in", "zoom", "dolly", "dramatic"]
//! params = [
//!   { name = "amount", type = "float", default = 0.2 },
//!   { name = "ease", type = "string", default = "ease_in_out_cubic" },
//! ]
clip.set_path("transform.scale", keys([[0.0, 1.0, args.ease], [ctx.clip_duration, 1.0 + args.amount]]));

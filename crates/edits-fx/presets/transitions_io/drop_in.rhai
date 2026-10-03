//! id = "drop_in"
//! name = "Drop In"
//! scope = "clip"
//! category = "entrance"
//! description = "Falls in from above and bounces."
//! tags = ["drop", "bounce", "entrance", "playful"]
//! params = [ { name = "duration", type = "float", default = 0.8 } ]
clip.set_path("transform.position", keys([[0.0, [0.0, -ctx.height], "ease_out_bounce"], [args.duration, [0.0, 0.0]]]));

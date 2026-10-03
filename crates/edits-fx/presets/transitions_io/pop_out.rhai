//! id = "pop_out"
//! name = "Pop Out"
//! scope = "clip"
//! category = "exit"
//! description = "Quick anticipation bump then shrink to nothing."
//! tags = ["pop", "scale", "exit"]
//! params = [ { name = "duration", type = "float", default = 0.35 } ]
let d = ctx.clip_duration;
let s = max(d - args.duration, 0.0);
clip.set_path("transform.scale", keys([[s, 1.0, "ease_out_quad"], [s + args.duration * 0.3, 1.15, "ease_in_back"], [d, 0.0]]));

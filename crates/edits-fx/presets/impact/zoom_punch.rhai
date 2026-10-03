//! id = "zoom_punch"
//! name = "Zoom Punch"
//! scope = "clip"
//! category = "impact"
//! description = "Snappy scale punch at a hit point (added as a transform effect, so it stacks with your own keyframes)."
//! tags = ["zoom", "punch", "hit", "impact", "amv"]
//! params = [
//!   { name = "at", type = "float", default = 0.0, desc = "Clip-local time of the hit (s)" },
//!   { name = "strength", type = "float", default = 0.25, desc = "Extra scale at the peak (0.25 = +25%)" },
//!   { name = "duration", type = "float", default = 0.35, desc = "Settle time (s)" },
//!   { name = "rotation", type = "float", default = 0.0, desc = "Optional rotation kick (degrees)" },
//! ]
let f = fx("transform", #{ scale: hit_keys(args.at, 1.0, 1.0 + args.strength, args.duration, "ease_out_expo"), edge: "mirror" });
if args.rotation != 0.0 { f.params.rotation = hit_keys(args.at, 0.0, args.rotation, args.duration, "ease_out_expo"); }
clip.add_fx(f);

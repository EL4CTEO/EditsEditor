//! id = "shockwave_hit"
//! name = "Shockwave Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Expanding refractive shockwave ring from a point."
//! tags = ["shockwave", "ring", "impact", "distortion"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "duration", type = "float", default = 0.7 },
//!   { name = "strength", type = "float", default = 0.06 },
//! ]
let a = args.at;
clip.add_fx(fx("shockwave", #{
    center: args.center,
    radius: keys([[a, 0.0, "ease_out_cubic"], [a + args.duration, 1.6]]),
    strength: keys([[a, args.strength], [a + args.duration, 0.0]]),
    width: 0.1, chroma: 0.8, brighten: 0.4
}));

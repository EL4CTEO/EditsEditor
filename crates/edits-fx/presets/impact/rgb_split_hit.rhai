//! id = "rgb_split_hit"
//! name = "RGB Split Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Chromatic channel split that snaps back after a hit."
//! tags = ["rgb split", "chromatic", "hit", "glitch"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "amount", type = "float", default = 40.0 },
//!   { name = "duration", type = "float", default = 0.3 },
//!   { name = "angle", type = "float", default = 0.0 },
//! ]
clip.add_fx(fx("rgb_split", #{ amount: hit_keys(args.at, 0.0, args.amount, args.duration, "ease_out_expo"), angle: args.angle }));

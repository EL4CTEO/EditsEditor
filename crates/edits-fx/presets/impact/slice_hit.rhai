//! id = "slice_hit"
//! name = "Slice Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Frame slices shove sideways and snap back — sharp, stylish impact."
//! tags = ["slice", "hit", "glitch", "amv"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "amount", type = "float", default = 120.0 },
//!   { name = "slices", type = "float", default = 10.0 },
//!   { name = "duration", type = "float", default = 0.3 },
//! ]
clip.add_fx(fx("slice", #{ amount: hit_keys(args.at, 0.0, args.amount, args.duration, "ease_out_expo"), slices: args.slices, random: 1.0, edge: "wrap" }));

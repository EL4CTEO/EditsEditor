//! id = "zoom_blur_hit"
//! name = "Zoom Blur Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Radial zoom blur burst at a hit."
//! tags = ["zoom blur", "radial", "hit", "speed"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "amount", type = "float", default = 0.35 },
//!   { name = "duration", type = "float", default = 0.3 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
clip.add_fx(fx("radial_blur", #{ amount: hit_keys(args.at, 0.0, args.amount, args.duration, "ease_out_quad"), center: args.center, samples: 40 }));

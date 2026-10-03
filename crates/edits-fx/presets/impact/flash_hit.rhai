//! id = "flash_hit"
//! name = "Flash Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Bright flash (white by default) that fades out after a hit."
//! tags = ["flash", "white", "hit", "impact", "amv"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "duration", type = "float", default = 0.25 },
//!   { name = "intensity", type = "float", default = 1.0 },
//! ]
clip.add_fx(fx("flash", #{ amount: hit_keys(args.at, 0.0, args.intensity, args.duration, "ease_out_quad"), color: args.color, exposure: 2.0 }));

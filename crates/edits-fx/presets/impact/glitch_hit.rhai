//! id = "glitch_hit"
//! name = "Glitch Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Burst of digital glitch + RGB tearing at a hit."
//! tags = ["glitch", "hit", "digital", "amv"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "intensity", type = "float", default = 0.9 },
//!   { name = "duration", type = "float", default = 0.35 },
//! ]
clip.add_fx(fx("glitch", #{ intensity: hit_keys(args.at, 0.0, args.intensity, args.duration, "ease_out_quad"), speed: 20.0, rgb_shift: 40.0 }));

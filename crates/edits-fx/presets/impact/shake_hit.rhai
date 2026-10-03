//! id = "shake_hit"
//! name = "Shake Hit"
//! scope = "clip"
//! category = "impact"
//! description = "Violent camera shake that decays after a hit."
//! tags = ["shake", "hit", "impact", "camera"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "amplitude", type = "float", default = 40.0, desc = "Pixels" },
//!   { name = "duration", type = "float", default = 0.5 },
//!   { name = "rotation", type = "float", default = 3.0 },
//! ]
clip.add_fx(fx("shake", #{
    amplitude: hit_keys(args.at, 0.0, args.amplitude, args.duration, "ease_out_quad"),
    rotation: hit_keys(args.at, 0.0, args.rotation, args.duration, "ease_out_quad"),
    frequency: 18.0, smear: 0.6, scale_fill: 1.08
}));

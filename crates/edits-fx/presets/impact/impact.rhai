//! id = "impact"
//! name = "Impact Combo"
//! scope = "clip"
//! category = "impact"
//! description = "The full hit: zoom punch + flash + shake + radial blur + chromatic split, all timed to one moment."
//! tags = ["impact", "hit", "combo", "amv", "drop"]
//! params = [
//!   { name = "at", type = "float", default = 0.0 },
//!   { name = "intensity", type = "float", default = 1.0, desc = "Scales every component" },
//!   { name = "flash_color", type = "color", default = "#ffffff" },
//! ]
let k = args.intensity;
let a = args.at;
clip.add_fx(fx("transform", #{ scale: hit_keys(a, 1.0, 1.0 + 0.3 * k, 0.4, "ease_out_expo"), edge: "mirror" }));
clip.add_fx(fx("radial_blur", #{ amount: hit_keys(a, 0.0, 0.3 * k, 0.25), samples: 32 }));
clip.add_fx(fx("rgb_split", #{ amount: hit_keys(a, 0.0, 30.0 * k, 0.3) }));
clip.add_fx(fx("shake", #{ amplitude: hit_keys(a, 0.0, 35.0 * k, 0.5, "ease_out_quad"), rotation: 2.5 * k, frequency: 18.0, smear: 0.6, scale_fill: 1.08 }));
clip.add_fx(fx("flash", #{ amount: hit_keys(a, 0.0, 0.9 * k, 0.2, "ease_out_quad"), color: args.flash_color, exposure: 2.0 }));

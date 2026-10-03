//! id = "sunset_look"
//! name = "Golden Sunset Look"
//! scope = "clip"
//! category = "look"
//! description = "Golden-hour warmth with soft bloom and a warm gradient overlay."
//! tags = ["sunset", "golden hour", "warm", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "golden_hour", amount: k }));
clip.add_fx(fx("gradient_overlay", #{ color_a: "#ff8a3d", color_b: "#5b2a86", angle: 90.0, blend: "soft_light", amount: 0.4 * k }));
clip.add_fx(fx("bloom", #{ intensity: 0.5 * k }));

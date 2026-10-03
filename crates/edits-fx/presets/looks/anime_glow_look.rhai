//! id = "anime_glow_look"
//! name = "Anime Glow Look"
//! scope = "clip"
//! category = "look"
//! description = "Vivid colors, crisp lines and a rich deep glow — the go-to anime edit grade."
//! tags = ["anime", "glow", "vivid", "look", "amv"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "anime_vivid", amount: 0.8 * k }));
clip.add_fx(fx("vibrance", #{ amount: 0.4 * k }));
clip.add_fx(fx("sharpen", #{ amount: 0.5 * k, radius: 1.2 }));
clip.add_fx(fx("deep_glow", #{ intensity: 0.9 * k, threshold: 0.55, radius: 18.0 }));

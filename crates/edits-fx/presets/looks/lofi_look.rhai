//! id = "lofi_look"
//! name = "Lo-fi Look"
//! scope = "clip"
//! category = "look"
//! description = "Warm, soft, grainy lo-fi aesthetic with subtle chroma bleed."
//! tags = ["lofi", "chill", "warm", "soft", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "warm", amount: 0.7 * k }));
clip.add_fx(fx("gaussian_blur", #{ radius: 1.2 * k }));
clip.add_fx(fx("vhs", #{ intensity: 0.3 * k, tracking: 0.05 }));
clip.add_fx(fx("film_grain", #{ amount: 0.14 * k }));

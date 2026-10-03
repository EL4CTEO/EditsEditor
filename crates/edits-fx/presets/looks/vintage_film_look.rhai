//! id = "vintage_film_look"
//! name = "Vintage Film Look"
//! scope = "clip"
//! category = "look"
//! description = "Faded warm film stock with grain, flicker and vignette."
//! tags = ["vintage", "film", "warm", "grain", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "vintage", amount: k }));
clip.add_fx(fx("film_grain", #{ amount: 0.18 * k, size: 1.8 }));
clip.add_fx(fx("flicker", #{ amount: 0.08 * k }));
clip.add_fx(fx("vignette", #{ amount: 0.5 * k, softness: 0.6 }));

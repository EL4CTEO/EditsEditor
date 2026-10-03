//! id = "horror_look"
//! name = "Horror Look"
//! scope = "clip"
//! category = "look"
//! description = "Desaturated sickly green grade, flicker, grain and dark edges."
//! tags = ["horror", "dark", "creepy", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "horror", amount: k }));
clip.add_fx(fx("flicker", #{ amount: 0.15 * k, speed: 14.0 }));
clip.add_fx(fx("film_grain", #{ amount: 0.2 * k }));
clip.add_fx(fx("vignette", #{ amount: 0.7 * k, radius: 0.5 }));

//! id = "noir_look"
//! name = "Noir Look"
//! scope = "clip"
//! category = "look"
//! description = "High-contrast black & white with heavy vignette and grain."
//! tags = ["noir", "black and white", "dramatic", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
clip.add_fx(fx("color_grade", #{ look: "noir", amount: args.intensity }));
clip.add_fx(fx("film_grain", #{ amount: 0.15 }));
clip.add_fx(fx("vignette", #{ amount: 0.6, radius: 0.6 }));

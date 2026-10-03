//! id = "cinematic_look"
//! name = "Cinematic Look"
//! scope = "clip"
//! category = "look"
//! description = "Teal & orange grade, 2.39:1 letterbox, subtle grain and vignette."
//! tags = ["cinematic", "teal orange", "letterbox", "movie", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 }, { name = "letterbox", type = "bool", default = true } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "teal_orange", amount: 0.9 * k }));
clip.add_fx(fx("film_grain", #{ amount: 0.06 * k }));
clip.add_fx(fx("vignette", #{ amount: 0.35 * k }));
if args.letterbox { clip.add_fx(fx("letterbox", #{ ratio: 2.39 })); }

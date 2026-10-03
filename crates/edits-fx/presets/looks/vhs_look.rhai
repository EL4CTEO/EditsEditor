//! id = "vhs_look"
//! name = "VHS Look"
//! scope = "clip"
//! category = "look"
//! description = "90s camcorder tape: chroma bleed, tracking noise, grain, soft scanlines."
//! tags = ["vhs", "retro", "90s", "tape", "lofi", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "vintage", amount: 0.6 * k }));
clip.add_fx(fx("vhs", #{ intensity: 0.7 * k }));
clip.add_fx(fx("film_grain", #{ amount: 0.12 * k }));
clip.add_fx(fx("vignette", #{ amount: 0.35 * k }));

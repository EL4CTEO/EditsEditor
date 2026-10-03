//! id = "dreamy_look"
//! name = "Dreamy Look"
//! scope = "clip"
//! category = "look"
//! description = "Soft pastel glow with light leaks and gentle bloom (romance, memories)."
//! tags = ["dreamy", "soft", "pastel", "romance", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "dreamy", amount: 0.8 * k }));
clip.add_fx(fx("bloom", #{ intensity: 0.7 * k, radius: 60.0, threshold: 0.5 }));
clip.add_fx(fx("light_leak", #{ intensity: 0.4 * k, speed: 0.2 }));
clip.add_fx(fx("vignette", #{ amount: 0.25 * k, color: "#2a1630" }));

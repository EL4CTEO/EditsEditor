//! id = "sketch_look"
//! name = "Pencil Sketch Look"
//! scope = "clip"
//! category = "look"
//! description = "Hand-drawn pencil sketch on paper."
//! tags = ["sketch", "pencil", "drawing", "art", "look"]
//! params = [ { name = "strength", type = "float", default = 1.5 } ]
clip.add_fx(fx("sketch", #{ strength: args.strength }));
clip.add_fx(fx("noise_overlay", #{ amount: 0.15 }));

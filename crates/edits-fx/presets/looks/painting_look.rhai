//! id = "painting_look"
//! name = "Painting Look"
//! scope = "clip"
//! category = "look"
//! description = "Oil-painting (Kuwahara) with boosted vibrance — painterly anime backgrounds."
//! tags = ["painting", "oil", "art", "painterly", "look"]
//! params = [ { name = "radius", type = "float", default = 5.0 } ]
clip.add_fx(fx("oil_paint", #{ radius: args.radius }));
clip.add_fx(fx("vibrance", #{ amount: 0.5 }));
clip.add_fx(fx("noise_overlay", #{ amount: 0.1, scale: 2.0 }));

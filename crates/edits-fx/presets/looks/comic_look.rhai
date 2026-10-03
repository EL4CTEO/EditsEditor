//! id = "comic_look"
//! name = "Comic Book Look"
//! scope = "clip"
//! category = "look"
//! description = "Bold posterized colors, ink outlines and halftone dots."
//! tags = ["comic", "pop art", "halftone", "look"]
//! params = [ { name = "levels", type = "float", default = 5.0 } ]
clip.add_fx(fx("comic", #{ levels: args.levels }));
clip.add_fx(fx("vibrance", #{ amount: 0.6 }));

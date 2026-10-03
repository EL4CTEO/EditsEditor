//! id = "manga_look"
//! name = "Manga Panel Look"
//! scope = "clip"
//! category = "look"
//! description = "Black & white manga page: ink lines, screentone shading, paper texture."
//! tags = ["manga", "black and white", "comic", "screentone", "look"]
//! params = [ { name = "tone_size", type = "float", default = 5.0 } ]
clip.add_fx(fx("sharpen", #{ amount: 1.0 }));
clip.add_fx(fx("manga_screentone", #{ tone_size: args.tone_size }));
clip.add_fx(fx("noise_overlay", #{ amount: 0.12 }));

//! id = "hologram_look"
//! name = "Hologram Look"
//! scope = "clip"
//! category = "look"
//! description = "Blue sci-fi hologram: tint, scanlines, flicker and RGB jitter."
//! tags = ["hologram", "sci-fi", "blue", "tech", "look"]
//! params = [ { name = "color", type = "color", default = "#3fd8ff" } ]
clip.add_fx(fx("tint", #{ black: "#000814", white: args.color }));
clip.add_fx(fx("scanlines", #{ amount: 0.35, spacing: 3.0, speed: 30.0 }));
clip.add_fx(fx("rgb_split", #{ amount: expr("2.0 + jitter(10.0, 4.0)") }));
clip.add_fx(fx("flicker", #{ amount: 0.12, speed: 24.0 }));
clip.add_fx(fx("glow", #{ intensity: 1.2, tint: args.color }));
clip.blend_mode = "screen";

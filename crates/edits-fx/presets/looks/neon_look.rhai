//! id = "neon_look"
//! name = "Neon Outline Look"
//! scope = "clip"
//! category = "look"
//! description = "Darkened image with glowing color-cycling neon edges."
//! tags = ["neon", "edges", "glow", "outline", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
clip.add_fx(fx("neon_edges", #{ strength: 3.0 * args.intensity, darken: 0.75 }));
clip.add_fx(fx("deep_glow", #{ intensity: 1.2 * args.intensity, threshold: 0.3 }));

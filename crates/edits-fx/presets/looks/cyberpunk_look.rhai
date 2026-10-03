//! id = "cyberpunk_look"
//! name = "Cyberpunk Look"
//! scope = "clip"
//! category = "look"
//! description = "Magenta/cyan split grade, neon glow, chromatic fringe and scanlines."
//! tags = ["cyberpunk", "neon", "magenta", "cyan", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("color_grade", #{ look: "cyberpunk", amount: k }));
clip.add_fx(fx("deep_glow", #{ intensity: 1.0 * k, tint: "#ff4fd8", threshold: 0.5 }));
clip.add_fx(fx("chromatic_aberration", #{ amount: 0.008 * k }));
clip.add_fx(fx("scanlines", #{ amount: 0.15 * k, spacing: 3.0 }));

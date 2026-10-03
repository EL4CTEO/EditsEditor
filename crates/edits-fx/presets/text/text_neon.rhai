//! id = "text_neon"
//! name = "Neon Sign Text"
//! scope = "clip"
//! category = "text"
//! description = "Glowing neon tube text with flicker-on."
//! tags = ["text", "neon", "glow", "sign", "cyberpunk"]
//! params = [ { name = "color", type = "color", default = "#ff2bd6" } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.color = "#ffffff";
clip.source.stroke = #{ color: args.color, width: 4.0 };
clip.add_fx(fx("deep_glow", #{ intensity: 2.2, threshold: 0.0, tint: args.color, radius: 16.0 }));
clip.opacity = keys([[0.0, 0.0, "hold"], [0.08, 1.0, "hold"], [0.14, 0.2, "hold"], [0.2, 1.0, "hold"], [0.3, 0.5, "hold"], [0.36, 1.0]]);

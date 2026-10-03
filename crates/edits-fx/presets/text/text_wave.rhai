//! id = "text_wave"
//! name = "Text Wave"
//! scope = "clip"
//! category = "text"
//! description = "Characters bob in a continuous wave."
//! tags = ["text", "wave", "loop", "playful"]
//! params = [ { name = "amplitude", type = "float", default = 10.0 }, { name = "frequency", type = "float", default = 1.5 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 0.03, duration: 0.3, from: #{ opacity: 0.0 }, wave: [args.amplitude, args.frequency, 0.5] };

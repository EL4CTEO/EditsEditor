//! id = "text_pop"
//! name = "Text Pop"
//! scope = "clip"
//! category = "text"
//! description = "Each character pops in with an elastic scale (titles, hype text)."
//! tags = ["text", "pop", "bounce", "title"]
//! params = [ { name = "stagger", type = "float", default = 0.04 }, { name = "exit", type = "bool", default = true } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
let a = #{ unit: "char", stagger: args.stagger, duration: 0.45, ease: #{ back: 2.5 }, from: #{ opacity: 0.0, scale: 0.0, offset: [0.0, 20.0] } };
if args.exit { a.out = #{ stagger: 0.02, duration: 0.25, ease: "ease_in_back", to: #{ opacity: 0.0, scale: 0.0 } }; }
clip.source.animator = a;

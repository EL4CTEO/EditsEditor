//! id = "text_typewriter"
//! name = "Typewriter"
//! scope = "clip"
//! category = "text"
//! description = "Characters appear one by one (text clips)."
//! tags = ["text", "typewriter", "type", "reveal"]
//! params = [ { name = "cps", type = "float", default = 18.0, desc = "Characters per second" }, { name = "start", type = "float", default = 0.0 } ]
if clip.source.type != "text" { log("typewriter needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 1.0 / args.cps, duration: 0.0, start: args.start, from: #{ opacity: 0.0 } };

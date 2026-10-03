//! id = "text_spin_in"
//! name = "Spin In Text"
//! scope = "clip"
//! category = "text"
//! description = "Characters spin and scale into place from the center outward."
//! tags = ["text", "spin", "rotate", "title"]
//! params = [ { name = "rotation", type = "float", default = 180.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 0.035, duration: 0.6, order: "center_out", ease: "ease_out_back",
    from: #{ opacity: 0.0, rotation: args.rotation, scale: 0.3 } };

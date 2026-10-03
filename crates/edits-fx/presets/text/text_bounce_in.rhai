//! id = "text_bounce_in"
//! name = "Bounce In Text"
//! scope = "clip"
//! category = "text"
//! description = "Characters drop in and bounce."
//! tags = ["text", "bounce", "drop", "playful"]
//! params = [ { name = "height", type = "float", default = 120.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 0.05, duration: 0.7, ease: "ease_out_bounce", from: #{ opacity: 0.0, offset: [0.0, -args.height] } };

//! id = "text_slide_up"
//! name = "Words Slide Up"
//! scope = "clip"
//! category = "text"
//! description = "Words rise into place one after another (clean lyric style)."
//! tags = ["text", "slide", "words", "lyrics", "clean"]
//! params = [ { name = "stagger", type = "float", default = 0.08 }, { name = "distance", type = "float", default = 40.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "word", stagger: args.stagger, duration: 0.5, ease: "ease_out_expo",
    from: #{ opacity: 0.0, offset: [0.0, args.distance] },
    out: #{ stagger: 0.04, duration: 0.3, ease: "ease_in_cubic", to: #{ opacity: 0.0, offset: [0.0, -args.distance * 0.5] } } };

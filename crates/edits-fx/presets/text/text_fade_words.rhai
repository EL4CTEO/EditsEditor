//! id = "text_fade_words"
//! name = "Words Fade"
//! scope = "clip"
//! category = "text"
//! description = "Gentle word-by-word fade with slight blur-like scale."
//! tags = ["text", "fade", "words", "soft"]
//! params = [ { name = "stagger", type = "float", default = 0.15 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "word", stagger: args.stagger, duration: 0.6, ease: "ease_out_cubic", from: #{ opacity: 0.0, scale: 1.15 },
    out: #{ stagger: 0.05, duration: 0.4, to: #{ opacity: 0.0 } } };

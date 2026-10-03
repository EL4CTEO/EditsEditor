//! id = "text_glitch"
//! name = "Glitch Text"
//! scope = "clip"
//! category = "text"
//! description = "Jittering characters, RGB split and glitch blocks."
//! tags = ["text", "glitch", "jitter", "cyber"]
//! params = [ { name = "jitter", type = "float", default = 4.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 0.02, duration: 0.1, order: "random", from: #{ opacity: 0.0 }, jitter: args.jitter, jitter_rate: 15.0 };
clip.add_fx(fx("rgb_split", #{ amount: expr("4.0 + jitter(12.0, 6.0)") }));
clip.add_fx(fx("glitch", #{ intensity: 0.35, block_size: 12.0 }));

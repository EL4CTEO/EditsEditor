//! id = "text_shake"
//! name = "Shaking Text"
//! scope = "clip"
//! category = "text"
//! description = "Per-character trembling (anger, fear, intensity)."
//! tags = ["text", "shake", "tremble", "angry"]
//! params = [ { name = "amount", type = "float", default = 3.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.animator = #{ unit: "char", stagger: 0.0, duration: 0.0, from: #{ opacity: 1.0 }, jitter: args.amount, jitter_rate: 24.0 };

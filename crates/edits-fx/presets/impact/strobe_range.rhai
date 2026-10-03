//! id = "strobe_range"
//! name = "Strobe Section"
//! scope = "clip"
//! category = "impact"
//! description = "Beat-locked strobe between two clip-local times."
//! tags = ["strobe", "flash", "rave", "section"]
//! params = [
//!   { name = "start", type = "float", default = 0.0 },
//!   { name = "end", type = "float", default = 2.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "amount", type = "float", default = 0.7 },
//!   { name = "on_beat", type = "bool", default = true },
//! ]
let f = fx("strobe", #{ on_beat: args.on_beat, rate: 12.0, color: args.color, amount: args.amount, duty: 0.4 });
f.range = [args.start, args.end];
clip.add_fx(f);

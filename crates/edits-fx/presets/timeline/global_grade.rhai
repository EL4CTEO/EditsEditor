//! id = "global_grade"
//! name = "Global Grade"
//! scope = "timeline"
//! category = "timeline"
//! description = "Adjustment layer over the whole composition with a color look, vignette and grain."
//! tags = ["grade", "color", "global", "look"]
//! params = [
//!   { name = "look", type = "enum", options = ["teal_orange", "anime_vivid", "cyberpunk", "sunset", "vintage", "bleach_bypass", "noir", "cold", "warm", "pastel", "matrix", "dreamy", "horror", "golden_hour"], default = "anime_vivid" },
//!   { name = "amount", type = "float", default = 0.8 },
//!   { name = "grain", type = "float", default = 0.05 },
//! ]
let comp = project.comp();
let tr = project.add_track("grade");
let adj = project.add_clip(tr, #{ start: 0.0, duration: comp.duration, source: #{ type: "adjustment" } });
project.add_effect(adj, fx("color_grade", #{ look: args.look, amount: args.amount }));
project.add_effect(adj, fx("vignette", #{ amount: 0.3 }));
if args.grain > 0.0 { project.add_effect(adj, fx("film_grain", #{ amount: args.grain })); }
adj

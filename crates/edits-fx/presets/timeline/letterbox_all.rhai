//! id = "letterbox_all"
//! name = "Cinema Bars"
//! scope = "timeline"
//! category = "timeline"
//! description = "Animated cinema bars over the whole edit (slide in at start, out at end)."
//! tags = ["letterbox", "cinema", "bars", "global"]
//! params = [ { name = "ratio", type = "float", default = 2.39 }, { name = "slide", type = "float", default = 0.6 } ]
let comp = project.comp();
let d = comp.duration;
let tr = project.add_track("letterbox");
let adj = project.add_clip(tr, #{ start: 0.0, duration: d, source: #{ type: "adjustment" } });
project.add_effect(adj, fx("letterbox", #{ ratio: args.ratio, amount: keys([[0.0, 0.0, "ease_out_cubic"], [args.slide, 1.0, "hold"], [d - args.slide, 1.0, "ease_in_cubic"], [d, 0.0]]) }));
adj

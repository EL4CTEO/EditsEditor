//! id = "hue_cycle_beats"
//! name = "Global Hue Cycle on Beats"
//! scope = "timeline"
//! category = "timeline"
//! description = "Adjustment layer whose hue jumps on every beat (between `start` and `end`)."
//! tags = ["hue", "color", "beat", "psychedelic", "global"]
//! params = [ { name = "start", type = "float", default = 0.0 }, { name = "end", type = "float", default = -1.0 }, { name = "step", type = "float", default = 60.0 } ]
let comp = project.comp();
let e = if args.end < 0.0 { comp.duration } else { args.end };
let tr = project.add_track("hue cycle");
let c = project.add_clip(tr, #{ start: args.start, duration: e - args.start, source: #{ type: "adjustment" } });
project.add_effect(c, fx("hue_saturation", #{ hue: expr("fmod(max(beat(), 0.0) * " + args.step + ", 360.0)") }));
c

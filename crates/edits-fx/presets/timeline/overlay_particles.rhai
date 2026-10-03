//! id = "overlay_particles"
//! name = "Particle Overlay"
//! scope = "timeline"
//! category = "timeline"
//! description = "Adds a full-length particle/weather overlay (sakura, snow, rain, embers, sparkles, bokeh, hearts, confetti)."
//! tags = ["overlay", "particles", "sakura", "snow", "sparkles"]
//! params = [
//!   { name = "kind", type = "enum", options = ["sakura", "snow", "rain", "embers", "sparkles", "bokeh", "hearts", "confetti", "starfield"], default = "sakura" },
//!   { name = "start", type = "float", default = 0.0 },
//!   { name = "duration", type = "float", default = -1.0 },
//!   { name = "opacity", type = "float", default = 0.9 },
//! ]
let comp = project.comp();
let d = if args.duration < 0.0 { comp.duration - args.start } else { args.duration };
let tr = project.add_track(args.kind);
let blend = if args.kind == "rain" || args.kind == "bokeh" || args.kind == "embers" || args.kind == "starfield" { "screen" } else { "normal" };
project.add_clip(tr, #{ start: args.start, duration: d, opacity: args.opacity, blend_mode: blend, source: #{ type: "generator", effect: args.kind } })

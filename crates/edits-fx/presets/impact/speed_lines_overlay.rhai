//! id = "speed_lines_overlay"
//! name = "Speed Lines Overlay"
//! scope = "clip"
//! category = "anime"
//! description = "Adds an anime speed/focus-lines generator clip above this clip for its whole duration."
//! tags = ["speed lines", "anime", "manga", "overlay", "focus"]
//! params = [
//!   { name = "style", type = "enum", options = ["radial", "linear"], default = "radial" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "opacity", type = "float", default = 0.8 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
let tr = project.add_track("speed lines");
let gen = if args.style == "linear" { "speed_lines_linear" } else { "speed_lines" };
let p = if args.style == "linear" { #{ color: args.color } } else { #{ color: args.color, center: args.center } };
project.add_clip(tr, #{ start: ctx.clip_start, duration: ctx.clip_duration, blend_mode: "add", opacity: args.opacity,
    source: #{ type: "generator", effect: gen, params: p } });

//! id = "drop_impact"
//! name = "Drop Impact + Speed Lines"
//! scope = "clip"
//! category = "impact"
//! description = "At every drop inside the clip: impact combo plus an anime speed-lines overlay clip for one bar."
//! tags = ["drop", "impact", "speed lines", "anime", "amv"]
//! params = [
//!   { name = "intensity", type = "float", default = 1.0 },
//!   { name = "lines_color", type = "color", default = "#ffffff" },
//! ]
let drops = ctx.drops;
if drops.len() == 0 { log("no drops inside this clip"); return; }
for d in drops { project.apply_preset(ctx.clip_id, "impact", #{ at: d, intensity: args.intensity }); }
let tr = project.add_track("speed lines");
let bar = ctx.beat_len * 4.0;
for d in drops {
    project.add_clip(tr, #{ start: ctx.clip_start + d, duration: bar, blend_mode: "add", opacity: 0.85,
        source: #{ type: "generator", effect: "speed_lines", params: #{ color: args.lines_color, count: 110.0, inner_radius: 0.3 } } });
}

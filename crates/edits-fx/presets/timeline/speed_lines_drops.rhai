//! id = "speed_lines_drops"
//! name = "Speed Lines on Drops"
//! scope = "timeline"
//! category = "timeline"
//! description = "Anime focus lines for one bar at each drop."
//! tags = ["speed lines", "drop", "anime", "global"]
//! params = [ { name = "bars", type = "float", default = 1.0 }, { name = "color", type = "color", default = "#ffffff" } ]
let tr = project.add_track("speed lines");
let bl = if bpm() > 0.0 { 60.0 / bpm() } else { 0.5 };
for d in drops() {
    project.add_clip(tr, #{ start: d, duration: bl * 4.0 * args.bars, blend_mode: "add", opacity: 0.85,
        source: #{ type: "generator", effect: "speed_lines", params: #{ color: args.color } } });
}
drops().len()

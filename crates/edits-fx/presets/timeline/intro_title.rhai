//! id = "intro_title"
//! name = "Intro Title Card"
//! scope = "timeline"
//! category = "timeline"
//! description = "Title + subtitle on an animated background at the start of the edit."
//! tags = ["title", "intro", "card", "text"]
//! params = [
//!   { name = "title", type = "string", default = "TITLE" },
//!   { name = "subtitle", type = "string", default = "" },
//!   { name = "duration", type = "float", default = 3.0 },
//!   { name = "background", type = "enum", options = ["sunburst", "halftone_pattern", "gradient", "plasma", "none"], default = "gradient" },
//!   { name = "color", type = "color", default = "#ffffff" },
//! ]
let comp = project.comp();
if args.background != "none" {
    let bt = project.add_track("title bg");
    project.add_clip(bt, #{ start: 0.0, duration: args.duration, source: #{ type: "generator", effect: args.background } });
}
let tt = project.add_track("title");
let t1 = project.add_clip(tt, #{ start: 0.1, duration: args.duration - 0.1,
    source: #{ type: "text", text: args.title, size: comp.height * 0.13, color: args.color, stroke: #{ color: "#000000", width: 6.0 } } });
project.apply_preset(t1, "text_pop", #{});
if args.subtitle != "" {
    let t2 = project.add_clip(tt, #{ start: 0.5, duration: args.duration - 0.5,
        source: #{ type: "text", text: args.subtitle, size: comp.height * 0.045, color: args.color, weight: 400 },
        transform: #{ position: [0.0, comp.height * 0.12] } });
    project.apply_preset(t2, "text_fade_words", #{});
}
t1

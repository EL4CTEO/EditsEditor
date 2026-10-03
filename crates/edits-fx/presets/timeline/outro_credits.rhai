//! id = "outro_credits"
//! name = "Scrolling Credits"
//! scope = "timeline"
//! category = "timeline"
//! description = "Scrolling credits text at the end of the edit."
//! tags = ["credits", "outro", "ending", "scroll"]
//! params = [
//!   { name = "lines", type = "string", default = "Edited with EditsEditor", desc = "Credits text; use | to separate lines" },
//!   { name = "duration", type = "float", default = 6.0 },
//!   { name = "size", type = "float", default = 0.0 },
//! ]
let comp = project.comp();
let d = min(args.duration, comp.duration);
let text = args.lines.split("|").map(|x| x.trim());
let joined = "";
for l in text { joined += l + "\n"; }
let size = if args.size > 0.0 { args.size } else { comp.height * 0.05 };
let h = size * 1.4 * text.len();
let tr = project.add_track("credits");
let c = project.add_clip(tr, #{ start: comp.duration - d, duration: d, source: #{ type: "text", text: joined, size: size, weight: 500 } });
project.set(c, "transform.position", keys([[0.0, [0.0, comp.height / 2.0 + h / 2.0]], [d, [0.0, -comp.height / 2.0 - h / 2.0]]]));
c

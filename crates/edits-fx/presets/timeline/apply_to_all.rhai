//! id = "apply_to_all"
//! name = "Apply Preset to All Clips"
//! scope = "timeline"
//! category = "timeline"
//! description = "Runs a clip preset on every clip of a track (e.g. velocity_beats, anime_glow_look)."
//! tags = ["batch", "preset", "all"]
//! params = [
//!   { name = "preset", type = "string", default = "anime_glow_look" },
//!   { name = "track", type = "string", default = "" },
//!   { name = "args_json", type = "string", default = "{}", desc = "Preset args as JSON" },
//! ]
let a = parse_json(args.args_json);
if type_of(a) != "map" { a = #{}; }
let tracks = if args.track == "" { project.tracks() } else { [args.track] };
let n = 0;
for t in tracks { for c in project.clips(t) { project.apply_preset(c, args.preset, a); n += 1; } }
n

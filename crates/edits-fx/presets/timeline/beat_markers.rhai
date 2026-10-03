//! id = "beat_markers"
//! name = "Beat Markers"
//! scope = "timeline"
//! category = "timeline"
//! description = "Adds markers for downbeats, drops and section starts (handy for reviewing in the viewer)."
//! tags = ["markers", "beats", "sections"]
//! params = [ { name = "downbeats", type = "bool", default = false } ]
for d in drops() { project.add_marker(d, "DROP"); }
for s in sections() { project.add_marker(s.start, s.label); }
if args.downbeats { for d in downbeats() { project.add_marker(d, ""); } }

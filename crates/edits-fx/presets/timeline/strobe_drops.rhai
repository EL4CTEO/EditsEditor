//! id = "strobe_drops"
//! name = "Strobe in Drops"
//! scope = "timeline"
//! category = "timeline"
//! description = "Beat-locked strobe during drop/chorus sections."
//! tags = ["strobe", "drop", "rave", "global"]
//! params = [ { name = "amount", type = "float", default = 0.35 } ]
let tr = project.add_track("strobe");
let n = 0;
for s in sections() {
    if s.label == "drop" || s.label == "chorus" {
        let c = project.add_clip(tr, #{ start: s.start, duration: s.end - s.start, source: #{ type: "adjustment" } });
        project.add_effect(c, fx("strobe", #{ on_beat: true, amount: args.amount, duty: 0.3 }));
        n += 1;
    }
}
n

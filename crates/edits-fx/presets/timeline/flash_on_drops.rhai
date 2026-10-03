//! id = "flash_on_drops"
//! name = "Flash on Drops"
//! scope = "timeline"
//! category = "timeline"
//! description = "Adds an adjustment layer with big flashes + shake at every drop (and lighter flashes on the following downbeats)."
//! tags = ["drop", "flash", "global", "amv"]
//! params = [ { name = "follow_bars", type = "int", default = 4, desc = "Downbeats after each drop that also flash" }, { name = "intensity", type = "float", default = 1.0 } ]
let comp = project.comp();
let ds = drops();
if ds.len() == 0 { throw "no drops in project timing"; }
let hits = [];
for d in ds {
    hits.push(d);
    let n = 0;
    for db in downbeats() { if db > d + 0.01 && n < args.follow_bars { hits.push(db); n += 1; } }
}
let tr = project.add_track("drop fx");
let adj = project.add_clip(tr, #{ start: 0.0, duration: comp.duration, source: #{ type: "adjustment" } });
project.add_effect(adj, fx("flash", #{ amount: pulse_keys(hits, 0.0, 0.8 * args.intensity, 0.0, 0.2, "ease_out_quad"), exposure: 2.0 }));
project.add_effect(adj, fx("shake", #{ amplitude: pulse_keys(ds, 0.0, 45.0 * args.intensity, 0.0, 0.8, "ease_out_quad"), frequency: 16.0, scale_fill: 1.08 }));
adj

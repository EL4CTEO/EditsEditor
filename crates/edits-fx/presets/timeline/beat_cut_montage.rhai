//! id = "beat_cut_montage"
//! name = "Beat Cut Montage"
//! scope = "timeline"
//! category = "timeline"
//! description = "Cuts footage on the beat grid: sequential or random shots from the given video assets (optionally using detected scenes so cuts land inside real shots)."
//! tags = ["montage", "beat", "cuts", "auto", "amv"]
//! params = [
//!   { name = "assets", type = "string", default = "", desc = "Comma-separated video asset ids (empty = all videos)" },
//!   { name = "every", type = "int", default = 2, desc = "Beats per cut" },
//!   { name = "start", type = "float", default = 0.0, desc = "Timeline start (s)" },
//!   { name = "end", type = "float", default = -1.0, desc = "Timeline end (-1 = composition end)" },
//!   { name = "use_scenes", type = "bool", default = true, desc = "Pick segments inside detected shots" },
//!   { name = "order", type = "enum", options = ["random", "sequential", "motion"], default = "random" },
//!   { name = "track", type = "string", default = "", desc = "Existing track id (empty = new track)" },
//!   { name = "speed", type = "float", default = 1.0 },
//! ]
let ids = if args.assets == "" { project.assets("video") } else { args.assets.split(",").map(|x| x.trim()) };
if ids.len() == 0 { throw "no video assets — import footage first"; }
let comp = project.comp();
let end_t = if args.end < 0.0 { comp.duration } else { args.end };
let grid = beats_between(args.start, end_t);
if grid.len() < 2 { throw "no beat grid in range — run analyze_audio with apply=true"; }
let cuts = every(grid, args.every);
cuts.push(end_t);
// pool of candidate segments [asset, start, end, motion]
let pool = [];
for id in ids {
    let dur = project.asset_duration(id);
    if args.use_scenes {
        let shots = scenes(id);
        for s in shots { if s.end - s.start > 0.4 { pool.push(#{ asset: id, start: s.start, end: s.end, motion: s.motion }); } }
    } else {
        pool.push(#{ asset: id, start: 0.0, end: dur, motion: 0.5 });
    }
}
if pool.len() == 0 { throw "no usable shots"; }
if args.order == "random" { pool = shuffle(pool); }
if args.order == "motion" { pool.sort(|a, b| if a.motion > b.motion { -1 } else if a.motion < b.motion { 1 } else { 0 }); }
let tr = args.track;
if tr == "" { tr = project.add_track("montage"); }
let made = 0;
for i in 0..cuts.len() - 1 {
    let a = cuts[i];
    let b = cuts[i + 1];
    if b - a < 0.05 { continue; }
    let seg = pool[i % pool.len()];
    let need = (b - a) * args.speed;
    let room = seg.end - seg.start - need;
    let src_in = if room > 0.0 { seg.start + rand(0.0, room) } else { seg.start };
    project.add_clip(tr, #{ start: a, duration: b - a, source: #{ type: "media", asset: seg.asset, no_audio: true }, "in": src_in, speed: args.speed, fit: "cover" });
    made += 1;
}
log("montage: " + made + " cuts on track " + tr);
tr

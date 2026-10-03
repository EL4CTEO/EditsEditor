//! id = "invert_flash_beats"
//! name = "Invert Flash on Beats"
//! scope = "clip"
//! category = "impact"
//! description = "Hard color inversion for a couple of frames on each hit — classic AMV accent."
//! tags = ["invert", "flash", "beat", "amv"]
//! params = [
//!   { name = "on", type = "enum", options = ["beats", "downbeats", "drops", "half", "accents"], default = "downbeats" },
//!   { name = "every", type = "int", default = 1 },
//!   { name = "frames", type = "int", default = 2, desc = "How many frames stay inverted" },
//! ]
let ts = pick_times(ctx, args.on, args.every);
if ts.len() == 0 { log("no hits in clip"); return; }
let d = args.frames / ctx.fps;
let ks = [];
for t in ts { ks.push([t, 1.0, "hold"]); ks.push([t + d, 0.0, "hold"]); }
if ts[0] > 0.0 { ks.insert(0, [0.0, 0.0, "hold"]); }
clip.add_fx(fx("invert", #{ amount: keys(ks) }));

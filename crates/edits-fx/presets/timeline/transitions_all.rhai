//! id = "transitions_all"
//! name = "Transitions Everywhere"
//! scope = "timeline"
//! category = "timeline"
//! description = "Gives every cut on a track an in-transition (one effect, or random from a pool)."
//! tags = ["transitions", "batch", "cuts"]
//! params = [
//!   { name = "track", type = "string", default = "", desc = "Track id (empty = the track with the most clips)" },
//!   { name = "effect", type = "string", default = "random", desc = "Transition id or 'random'" },
//!   { name = "duration", type = "float", default = 0.3 },
//!   { name = "probability", type = "float", default = 1.0, desc = "Chance that a cut gets a transition" },
//! ]
let tr = args.track;
if tr == "" {
    let best = 0;
    for t in project.tracks() { let n = project.clips(t).len(); if n > best { best = n; tr = t; } }
}
let pool = ["crossfade", "flash_cut", "zoom_through", "whip_pan", "cross_zoom", "glitch_transition", "slice_slide", "spin", "noise_dissolve", "chromatic_split", "ink", "wipe"];
let cs = project.clips(tr);
let n = 0;
for i in 1..cs.len() {
    if rand() > args.probability { continue; }
    let c = project.clip(cs[i]);
    let e = if args.effect == "random" { choose(pool) } else { args.effect };
    project.set(cs[i], "transition_in", #{ effect: e, duration: min(args.duration, c.duration * 0.5), ease: "ease_in_out" });
    n += 1;
}
if n == 0 { log("no cuts to transition on " + tr); } else { log(n + " transitions on " + tr); }
n

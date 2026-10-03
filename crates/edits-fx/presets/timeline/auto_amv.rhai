//! id = "auto_amv"
//! name = "Auto AMV"
//! scope = "timeline"
//! category = "timeline"
//! description = "Builds a complete first-pass AMV from music + footage: beat-synced cuts (faster in drops/choruses, high-motion shots there), impacts and flashes on downbeats in drops, velocity ramps, transitions at section changes, a color grade, title and optional lyrics. Requires an analyzed music asset."
//! tags = ["auto", "amv", "edit", "montage", "full"]
//! params = [
//!   { name = "music", type = "string", default = "", desc = "Music asset id (empty = timing source)" },
//!   { name = "footage", type = "string", default = "", desc = "Comma-separated video ids (empty = all videos except music)" },
//!   { name = "look", type = "enum", options = ["anime_vivid", "cyberpunk", "teal_orange", "dreamy", "sunset", "cold", "none"], default = "anime_vivid" },
//!   { name = "title", type = "string", default = "", desc = "Optional title text at the start" },
//!   { name = "lyrics", type = "string", default = "", desc = "Optional subtitles/lyrics asset id" },
//!   { name = "intensity", type = "float", default = 1.0 },
//!   { name = "fit_duration", type = "bool", default = true, desc = "Set composition duration to the music length" },
//! ]
let music = if args.music == "" { project.get("main", "") ; "" } else { args.music };
let t = project.json().timing;
if music == "" { music = t.source ?? ""; }
if music == "" { throw "no music: pass `music` or run analyze_audio(apply=true) first"; }
if beats().len() < 8 { throw "music not analyzed — run analyze_audio with apply=true"; }
let k = args.intensity;
let mdur = project.asset_duration(music);
if args.fit_duration && mdur > 0.0 { project.set_duration(mdur); }
let comp = project.comp();
// music
let mt = project.add_track("music");
project.add_clip(mt, #{ start: 0.0, duration: comp.duration, source: #{ type: "media", asset: music } });
// footage pool with scene stats, split into calm / action
let ids = if args.footage == "" { project.assets("video").filter(|x| x != music) } else { args.footage.split(",").map(|x| x.trim()) };
if ids.len() == 0 { throw "no footage videos imported"; }
let calm = [];
let action = [];
for id in ids {
    for s in scenes(id) {
        if s.end - s.start < 0.5 { continue; }
        let seg = #{ asset: id, start: s.start, end: s.end, motion: s.motion, brightness: s.brightness };
        if s.motion > 0.18 { action.push(seg); } else { calm.push(seg); }
    }
}
if calm.len() == 0 && action.len() == 0 { throw "no shots detected in footage"; }
if calm.len() == 0 { calm = action; }
if action.len() == 0 { action = calm; }
calm = shuffle(calm);
action = shuffle(action);
let secs = sections();
let section_of = |time| { let lab = "verse"; for s in secs { if time >= s.start && time < s.end { lab = s.label; } } lab };
let ft = project.add_track("footage");
let bs = beats();
let i = 0;
let ci = 0;
let ai = 0;
let prev_label = "";
let made = [];
while i < bs.len() - 1 {
    let a = bs[i];
    if a >= comp.duration { break; }
    let lab = section_of.call(a);
    let hot = lab == "drop" || lab == "chorus";
    let step = if hot { 1 } else if lab == "build" { 2 } else { 4 };
    let j = min(i + step, bs.len() - 1);
    let b = min(bs[j], comp.duration);
    if b - a < 0.08 { i = j; continue; }
    let seg = if hot { ai += 1; action[ai % action.len()] } else { ci += 1; calm[ci % calm.len()] };
    let room = seg.end - seg.start - (b - a);
    let src_in = if room > 0.0 { seg.start + rand(0.0, room) } else { seg.start };
    let c = project.add_clip(ft, #{ start: a, duration: b - a, source: #{ type: "media", asset: seg.asset, no_audio: true }, "in": src_in, fit: "cover" });
    if lab != prev_label && prev_label != "" {
        let tr = if hot { "flash_cut" } else { choose(["zoom_through", "whip_pan", "cross_zoom", "glitch_transition", "slice_slide"]) };
        project.set(c, "transition_in", #{ effect: tr, duration: min(0.35, (b - a) * 0.5), ease: "ease_in_out" });
    }
    if hot { project.apply_preset(c, "zoom_punch", #{ at: 0.0, strength: 0.18 * k }); }
    else if lab == "build" { project.apply_preset(c, "push_in", #{ amount: 0.1 }); }
    else if rand() < 0.3 { project.apply_preset(c, "velocity_ramp", #{ fast: 1.8, slow: 0.35 }); }
    prev_label = lab;
    made.push(c);
    i = j;
}
// impacts on drops, flashes on downbeats inside hot sections
let fxt = project.add_track("fx");
let adj = project.add_clip(fxt, #{ start: 0.0, duration: comp.duration, source: #{ type: "adjustment" } });
let hot_db = downbeats().filter(|d| { let l = section_of.call(d); l == "drop" || l == "chorus" });
if hot_db.len() > 0 {
    project.add_effect(adj, fx("flash", #{ amount: pulse_keys(hot_db, 0.0, 0.6 * k, 0.0, 0.18, "ease_out_quad"), exposure: 1.5 }));
    project.add_effect(adj, fx("shake", #{ amplitude: pulse_keys(hot_db, 0.0, 22.0 * k, 0.0, 0.35, "ease_out_quad"), frequency: 18.0, smear: 0.5, scale_fill: 1.06 }));
    project.add_effect(adj, fx("rgb_split", #{ amount: pulse_keys(hot_db, 0.0, 18.0 * k, 0.0, 0.25, "ease_out_expo") }));
}
for d in drops() {
    project.add_effect(adj, fx("shockwave", #{ radius: keys([[d, 0.0, "ease_out_cubic"], [d + 0.7, 1.6]]), strength: keys([[d, 0.06], [d + 0.7, 0.0]]) }));
}
if args.look != "none" {
    project.add_effect(adj, fx("color_grade", #{ look: args.look, amount: 0.8 }));
    project.add_effect(adj, fx("deep_glow", #{ intensity: 0.6 * k, threshold: 0.6 }));
    project.add_effect(adj, fx("vignette", #{ amount: 0.3 }));
}
if args.title != "" {
    let tt = project.add_track("title");
    let tc = project.add_clip(tt, #{ start: 0.2, duration: min(4.0, comp.duration * 0.25), source: #{ type: "text", text: args.title, size: comp.height * 0.11 } });
    project.apply_preset(tc, "text_cinematic", #{});
}
if args.lyrics != "" { project.apply_preset("", "lyrics_from_subtitles", #{ asset: args.lyrics }); }
log("auto_amv: " + made.len() + " cuts, " + hot_db.len() + " hot downbeats, " + drops().len() + " drops");
made.len()

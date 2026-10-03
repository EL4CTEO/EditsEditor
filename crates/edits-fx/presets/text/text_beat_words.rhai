//! id = "text_beat_words"
//! name = "Words on Beats"
//! scope = "clip"
//! category = "text"
//! description = "Reveals one word per beat (kinetic typography synced to music)."
//! tags = ["text", "kinetic", "beat", "words", "lyrics"]
//! params = [ { name = "on", type = "enum", options = ["beats", "half", "downbeats"], default = "beats" } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
let ts = pick_times(ctx, args.on, 1);
let step = if ts.len() > 1 { ts[1] - ts[0] } else { ctx.beat_len };
let start = if ts.len() > 0 { ts[0] } else { 0.0 };
clip.source.animator = #{ unit: "word", stagger: step, duration: 0.15, start: start, ease: "ease_out_back", from: #{ opacity: 0.0, scale: 1.6 } };
clip.add_fx(fx("transform", #{ scale: expr("1.0 + 0.06 * pulse(10.0)") }));

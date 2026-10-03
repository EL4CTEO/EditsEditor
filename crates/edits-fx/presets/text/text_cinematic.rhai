//! id = "text_cinematic"
//! name = "Cinematic Title"
//! scope = "clip"
//! category = "text"
//! description = "Wide tracking that slowly expands while fading in and out — movie title style."
//! tags = ["text", "cinematic", "title", "tracking", "movie"]
//! params = [ { name = "tracking", type = "float", default = 30.0, desc = "Extra letter spacing at the end" } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
let d = ctx.clip_duration;
clip.source.letter_spacing = keys([[0.0, 4.0, "ease_out_cubic"], [d, args.tracking]]);
clip.source.uppercase = true;
clip.set_kf("opacity", 0.0, 0.0, "ease_out_cubic");
clip.set_kf("opacity", min(0.8, d / 3.0), 1.0, "linear");
clip.set_kf("opacity", max(d - 0.6, d * 0.7), 1.0, "ease_in_cubic");
clip.set_kf("opacity", d, 0.0, "linear");

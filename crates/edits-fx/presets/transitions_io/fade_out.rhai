//! id = "fade_out"
//! name = "Fade Out"
//! scope = "clip"
//! category = "exit"
//! description = "Opacity fade at the end of the clip (also fades clip audio)."
//! tags = ["fade", "out", "exit"]
//! params = [ { name = "duration", type = "float", default = 0.4 }, { name = "ease", type = "string", default = "ease_in_cubic" } ]
let d = ctx.clip_duration;
clip.set_kf("opacity", max(d - args.duration, 0.0), 1.0, args.ease);
clip.set_kf("opacity", d, 0.0, "linear");
clip.set_path("audio.fade_out", args.duration);

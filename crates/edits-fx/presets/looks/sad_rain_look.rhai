//! id = "sad_rain_look"
//! name = "Sad Rain Look"
//! scope = "clip"
//! category = "look"
//! description = "Cold desaturated grade plus a rain overlay clip — melancholic scenes."
//! tags = ["sad", "rain", "cold", "melancholy", "look"]
//! params = [ { name = "rain", type = "bool", default = true } ]
clip.add_fx(fx("color_grade", #{ look: "cold", amount: 1.0 }));
clip.add_fx(fx("hue_saturation", #{ saturation: 0.6 }));
clip.add_fx(fx("vignette", #{ amount: 0.45 }));
if args.rain {
    let tr = project.add_track("rain");
    project.add_clip(tr, #{ start: ctx.clip_start, duration: ctx.clip_duration, blend_mode: "screen",
        source: #{ type: "generator", effect: "rain", params: #{ opacity: 0.5 } } });
}

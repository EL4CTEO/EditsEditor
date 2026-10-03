//! id = "audio_shake"
//! name = "Bass Shake"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "Camera shake whose amplitude follows the bass."
//! tags = ["shake", "bass", "audio", "reactive"]
//! params = [ { name = "amplitude", type = "float", default = 30.0 } ]
clip.add_fx(fx("shake", #{ amplitude: expr(args.amplitude + " * bass() * bass()"), frequency: 14.0, smear: 0.4, scale_fill: 1.06 }));

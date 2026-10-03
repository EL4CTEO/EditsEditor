//! id = "dream_sequence"
//! name = "Dream Sequence"
//! scope = "clip"
//! category = "look"
//! description = "Blurred edges, wavy distortion and glow — flashbacks and dream scenes."
//! tags = ["dream", "flashback", "memory", "blur", "look"]
//! params = [ { name = "intensity", type = "float", default = 1.0 } ]
let k = args.intensity;
clip.add_fx(fx("wave_warp", #{ amplitude: 3.0 * k, wavelength: 300.0, speed: 0.3 }));
clip.add_fx(fx("tilt_shift", #{ radius: 10.0 * k, width: 0.35 }));
clip.add_fx(fx("bloom", #{ intensity: 0.8 * k }));
clip.add_fx(fx("color_grade", #{ look: "dreamy", amount: 0.7 * k }));

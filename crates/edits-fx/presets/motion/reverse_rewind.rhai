//! id = "reverse_rewind"
//! name = "VHS Rewind"
//! scope = "clip"
//! category = "motion"
//! description = "Plays the clip backwards fast with VHS tracking noise — the rewind trope."
//! tags = ["reverse", "rewind", "vhs", "retro"]
//! params = [ { name = "speed", type = "float", default = 2.0 } ]
clip.reverse = true;
clip.speed = args.speed;
clip.add_fx(fx("vhs", #{ intensity: 0.9, tracking: 0.8, noise: 0.25 }));
clip.add_fx(fx("scanlines", #{ amount: 0.3, speed: 60.0 }));

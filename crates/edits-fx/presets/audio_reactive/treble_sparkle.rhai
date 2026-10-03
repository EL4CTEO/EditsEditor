//! id = "treble_rgb"
//! name = "Treble RGB Split"
//! scope = "clip"
//! category = "audio_reactive"
//! description = "RGB split reacting to hi-hats / treble transients."
//! tags = ["rgb split", "treble", "hi-hat", "reactive"]
//! params = [ { name = "amount", type = "float", default = 18.0 } ]
clip.add_fx(fx("rgb_split", #{ amount: expr(args.amount + " * treble() * treble()") }));

//! id = "handheld"
//! name = "Handheld Camera"
//! scope = "clip"
//! category = "motion"
//! description = "Subtle continuous handheld drift — makes static shots feel alive."
//! tags = ["handheld", "camera", "drift", "organic"]
//! params = [
//!   { name = "amount", type = "float", default = 6.0, desc = "Pixels" },
//!   { name = "speed", type = "float", default = 1.5 },
//! ]
clip.add_fx(fx("shake", #{ amplitude: args.amount, rotation: args.amount * 0.08, frequency: args.speed, smear: 0.0, scale_fill: 1.03, zoom: 0.01 }));

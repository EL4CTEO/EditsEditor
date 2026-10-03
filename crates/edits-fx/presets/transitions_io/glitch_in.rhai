//! id = "glitch_in"
//! name = "Glitch In"
//! scope = "clip"
//! category = "entrance"
//! description = "Materializes through digital glitches and flicker."
//! tags = ["glitch", "entrance", "digital"]
//! params = [ { name = "duration", type = "float", default = 0.5 } ]
clip.add_fx(fx("glitch", #{ intensity: keys([[0.0, 1.0, "ease_in_quad"], [args.duration, 0.0]]), speed: 24.0, rgb_shift: 60.0 }));
clip.opacity = keys([[0.0, 0.0, "hold"], [0.05, 1.0, "hold"], [0.1, 0.2, "hold"], [0.15, 1.0, "hold"], [0.22, 0.4, "hold"], [0.27, 1.0]]);

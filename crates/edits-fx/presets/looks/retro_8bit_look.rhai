//! id = "retro_8bit_look"
//! name = "Retro 8-bit Look"
//! scope = "clip"
//! category = "look"
//! description = "Pixelated, dithered game-console look."
//! tags = ["8bit", "pixel", "retro", "gameboy", "look"]
//! params = [
//!   { name = "pixel_size", type = "float", default = 6.0 },
//!   { name = "palette", type = "enum", options = ["gameboy", "cga", "color", "mono", "sepia"], default = "color" },
//! ]
clip.add_fx(fx("pixelate", #{ size: args.pixel_size }));
clip.add_fx(fx("dither", #{ palette: args.palette, levels: 4.0, pixel_size: args.pixel_size }));

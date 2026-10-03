//! id = "ascii"
//! name = "ASCII / Text Mode"
//! kind = "filter"
//! category = "stylize"
//! description = "Renders the image as procedural text-mode glyphs (5x5 bitmap characters)."
//! tags = ["ascii", "text", "matrix", "terminal", "retro"]
//! params = [
//!   { name = "size", type = "float", default = 10.0, min = 4.0, max = 64.0, desc = "Character cell size in pixels" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "use_source_color", type = "bool", default = true },
//!   { name = "background", type = "color", default = "#000000" },
//! ]
fn glyph(n: i32, p: vec2f) -> f32 {
    // p in 0..1 cell space -> 5x5 bit index
    let ip = vec2i(clamp(floor(p * 5.0), vec2f(0.0), vec2f(4.0)));
    let bit = ip.x + ip.y * 5;
    return f32((n >> u32(bit)) & 1);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let px = uv * res();
    let cell = floor(px / p.size);
    let c = src_clamp((cell + 0.5) * p.size / res());
    let l = luma(unpremul(c).rgb);
    var chars = array<i32, 8>(0, 4194304, 4329604, 332772, 15255086, 23385164, 15252014, 33554431);
    let idx = i32(clamp(l * 7.99, 0.0, 7.0));
    let g = glyph(chars[idx], fract(px / p.size) * 1.1 - 0.05);
    var fg = p.color.rgb;
    if (p.use_source_color > 0.5) { fg = unpremul(c).rgb * 1.3; }
    return vec4f(mix(p.background.rgb, fg, g), 1.0);
}

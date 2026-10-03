//! id = "invert"
//! name = "Invert"
//! kind = "filter"
//! category = "color"
//! description = "Invert colors (negative). Flash-inverts on beats are an AMV staple."
//! tags = ["color", "invert", "negative", "amv"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "channels", type = "enum", options = ["rgb", "luma", "hue", "red", "green", "blue"], default = "rgb" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var o = c.rgb;
    let m = i32(p.channels);
    if (m == 0) { o = 1.0 - c.rgb; }
    else if (m == 1) { let l = luma(c.rgb); o = c.rgb + (1.0 - 2.0 * l); }
    else if (m == 2) { var h = rgb2hsv(saturate3(c.rgb)); h.x = fract(h.x + 0.5); o = hsv2rgb(h); }
    else if (m == 3) { o.r = 1.0 - c.r; }
    else if (m == 4) { o.g = 1.0 - c.g; }
    else { o.b = 1.0 - c.b; }
    return grade_end(vec4f(mix(c.rgb, o, p.amount), c.a));
}

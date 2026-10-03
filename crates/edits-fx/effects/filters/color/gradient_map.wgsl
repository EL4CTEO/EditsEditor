//! id = "gradient_map"
//! name = "Gradient Map"
//! kind = "filter"
//! category = "color"
//! description = "Maps luminance through a 4-stop color gradient (stylized color grades, neon looks)."
//! tags = ["color", "gradient map", "stylize", "neon", "duotone"]
//! params = [
//!   { name = "color1", type = "color", default = "#0b0033" },
//!   { name = "color2", type = "color", default = "#7a00ff" },
//!   { name = "color3", type = "color", default = "#ff2e88" },
//!   { name = "color4", type = "color", default = "#fff3b0" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "offset", type = "float", default = 0.0, min = -1.0, max = 1.0, desc = "Shift the gradient (animate for color cycling)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = fract(clamp(luma(c.rgb), 0.0, 0.999) + p.offset);
    var g = vec3f(0.0);
    if (l < 0.333) { g = mix(to_srgb(p.color1.rgb), to_srgb(p.color2.rgb), l * 3.0); }
    else if (l < 0.666) { g = mix(to_srgb(p.color2.rgb), to_srgb(p.color3.rgb), (l - 0.333) * 3.0); }
    else { g = mix(to_srgb(p.color3.rgb), to_srgb(p.color4.rgb), (l - 0.666) * 3.0); }
    return grade_end(vec4f(mix(c.rgb, g, p.amount), c.a));
}

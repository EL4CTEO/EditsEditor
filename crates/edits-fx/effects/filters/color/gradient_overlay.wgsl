//! id = "gradient_overlay"
//! name = "Gradient Overlay"
//! kind = "filter"
//! category = "color"
//! description = "Overlay a linear or radial two-color gradient using screen/multiply/overlay/normal blending."
//! tags = ["gradient", "overlay", "color", "sky"]
//! params = [
//!   { name = "color_a", type = "color", default = "#ff3d7f" },
//!   { name = "color_b", type = "color", default = "#3d5aff" },
//!   { name = "angle", type = "angle", default = 90.0 },
//!   { name = "radial", type = "bool", default = false },
//!   { name = "blend", type = "enum", options = ["normal", "screen", "multiply", "overlay", "soft_light"], default = "soft_light" },
//!   { name = "amount", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var t = dot(uv - 0.5, dir_deg(p.angle)) + 0.5;
    if (p.radial > 0.5) { t = length(centered(uv, vec2f(0.5))) * 1.2; }
    let g = mix(to_srgb(p.color_a.rgb), to_srgb(p.color_b.rgb), clamp(t, 0.0, 1.0));
    var o = c.rgb;
    let m = i32(p.blend);
    if (m == 0) { o = g; }
    else if (m == 1) { o = screen3(c.rgb, g); }
    else if (m == 2) { o = c.rgb * g; }
    else if (m == 3) { o = select(1.0 - 2.0 * (1.0 - c.rgb) * (1.0 - g), 2.0 * c.rgb * g, c.rgb < vec3f(0.5)); }
    else { o = (1.0 - 2.0 * g) * c.rgb * c.rgb + 2.0 * g * c.rgb; }
    return grade_end(vec4f(mix(c.rgb, o, p.amount), c.a));
}

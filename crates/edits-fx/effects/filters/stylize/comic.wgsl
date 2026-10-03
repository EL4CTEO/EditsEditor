//! id = "comic"
//! name = "Comic Book"
//! kind = "filter"
//! category = "stylize"
//! description = "Bold posterized colors, black ink outlines and halftone shading."
//! tags = ["comic", "cartoon", "pop art", "outline", "toon"]
//! params = [
//!   { name = "levels", type = "float", default = 5.0, min = 2.0, max = 16.0 },
//!   { name = "edge", type = "float", default = 1.0, min = 0.0, max = 4.0 },
//!   { name = "halftone", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "saturation", type = "float", default = 1.4, min = 0.0, max = 3.0 },
//! ]
fn L(uv: vec2f) -> f32 { return luma(unpremul(src_clamp(uv)).rgb); }
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var col = mix(vec3f(luma(c.rgb)), c.rgb, p.saturation);
    col = floor(col * p.levels + 0.5) / p.levels;
    let e = texel() * 1.5;
    let gx = L(uv + vec2f(e.x, 0.0)) - L(uv - vec2f(e.x, 0.0));
    let gy = L(uv + vec2f(0.0, e.y)) - L(uv - vec2f(0.0, e.y));
    let edge = smoothstep(0.05, 0.2, length(vec2f(gx, gy)) * p.edge);
    let cell = fract(rot2(0.785) * (uv * res()) / 6.0) - 0.5;
    let shade = 1.0 - luma(col);
    let dotm = 1.0 - smoothstep(shade * 0.5, shade * 0.5 + 0.1, length(cell));
    col *= 1.0 - dotm * p.halftone * 0.5;
    return grade_end(vec4f(col * (1.0 - edge), c.a));
}

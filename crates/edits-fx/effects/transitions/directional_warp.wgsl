//! id = "directional_warp"
//! name = "Directional Warp"
//! kind = "transition"
//! category = "motion"
//! description = "A smooth warping front moves diagonally, scaling the images as it passes."
//! tags = ["warp", "directional", "smooth"]
//! params = [
//!   { name = "angle", type = "angle", default = -45.0 },
//!   { name = "smoothness", type = "float", default = 0.5, min = 0.01, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = normalize(dir_deg(p.angle));
    let v = d / (abs(d.x) + abs(d.y));
    let x = dot(v, uv - 0.5) + 0.5;
    let m = 1.0 - smoothstep(-p.smoothness, 0.0, x - progress() * (1.0 + p.smoothness));
    let a = src_clamp((uv - 0.5) * (1.0 - m) + 0.5);
    let b = orig_clamp((uv - 0.5) * m + 0.5);
    return mix(a, b, m);
}

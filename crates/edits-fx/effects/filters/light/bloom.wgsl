//! id = "bloom"
//! name = "Bloom"
//! kind = "filter"
//! category = "light"
//! description = "Subtle cinematic bloom with soft knee; screen-blended for natural highlights."
//! tags = ["bloom", "glow", "cinematic"]
//! params = [
//!   { name = "intensity", type = "float", default = 0.6, min = 0.0, max = 5.0 },
//!   { name = "radius", type = "float", default = 40.0, min = 1.0, max = 200.0 },
//!   { name = "threshold", type = "float", default = 0.75, min = 0.0, max = 2.0 },
//! ]
//! passes = [
//!   { entry = "bright", scale = 0.25 },
//!   { entry = "blur", scale = 0.25, data = [1.0, 0.0, 0.25, 0.0] },
//!   { entry = "blur", scale = 0.25, data = [0.0, 1.0, 0.25, 0.0] },
//!   { entry = "combine", inputs = ["prev", "input"] },
//! ]
fn bright(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let l = luma(c.rgb);
    let soft = clamp(l - p.threshold + 0.3, 0.0, 0.6);
    let w = max(soft * soft / 2.4, l - p.threshold) / max(l, 0.0001);
    return c * max(w, 0.0);
}
fn blur(uv: vec2f, p: Params) -> vec4f {
    return blur_dir(uv, U.g.pass_data.xy, p.radius * U.g.pass_data.z);
}
fn combine(uv: vec2f, p: Params) -> vec4f {
    let g = src_clamp(uv).rgb * p.intensity;
    let o = orig(uv);
    return vec4f(screen3(o.rgb, g), o.a);
}

//! id = "glow"
//! name = "Glow"
//! kind = "filter"
//! category = "light"
//! description = "Soft glow of the bright parts of the image."
//! tags = ["glow", "bloom", "light", "amv", "dreamy"]
//! params = [
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 10.0 },
//!   { name = "radius", type = "float", default = 24.0, min = 1.0, max = 200.0, desc = "Glow size in pixels" },
//!   { name = "threshold", type = "float", default = 0.6, min = 0.0, max = 2.0, desc = "Brightness where glow starts" },
//!   { name = "tint", type = "color", default = "#ffffff", desc = "Glow color multiplier" },
//!   { name = "glow_only", type = "bool", default = false, desc = "Output only the glow (for compositing)" },
//! ]
//! passes = [
//!   { entry = "bright", scale = 0.5 },
//!   { entry = "blur", scale = 0.5, data = [1.0, 0.0, 0.5, 0.0] },
//!   { entry = "blur", scale = 0.5, data = [0.0, 1.0, 0.5, 0.0] },
//!   { entry = "combine", inputs = ["prev", "input"] },
//! ]
fn bright(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let l = luma(c.rgb);
    let knee = 0.2;
    let k = smoothstep(p.threshold - knee, p.threshold + knee, l);
    return c * k;
}
fn blur(uv: vec2f, p: Params) -> vec4f {
    return blur_dir(uv, U.g.pass_data.xy, p.radius * U.g.pass_data.z);
}
fn combine(uv: vec2f, p: Params) -> vec4f {
    let g = src_clamp(uv) * p.intensity * vec4f(p.tint.rgb, 1.0);
    if (p.glow_only > 0.5) { return g; }
    let o = orig(uv);
    return vec4f(o.rgb + g.rgb, max(o.a, min(g.a, 1.0)));
}

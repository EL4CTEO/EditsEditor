//! id = "deep_glow"
//! name = "Deep Glow"
//! kind = "filter"
//! category = "light"
//! description = "Multi-scale physically inspired glow with a wide, rich falloff — the signature anime-edit glow."
//! tags = ["glow", "deep glow", "bloom", "amv", "neon"]
//! params = [
//!   { name = "intensity", type = "float", default = 1.2, min = 0.0, max = 10.0 },
//!   { name = "radius", type = "float", default = 20.0, min = 1.0, max = 120.0, desc = "Base radius; larger scales derive from it" },
//!   { name = "threshold", type = "float", default = 0.5, min = 0.0, max = 2.0 },
//!   { name = "tint", type = "color", default = "#ffffff" },
//!   { name = "saturation", type = "float", default = 1.2, min = 0.0, max = 3.0, desc = "Glow color saturation" },
//!   { name = "falloff", type = "float", default = 0.7, min = 0.1, max = 1.5, desc = "Weight of the wide layers" },
//! ]
//! passes = [
//!   { entry = "bright", scale = 0.5, name = "b" },
//!   { entry = "blur", scale = 0.5, data = [1.0, 0.0, 0.5, 0.0] },
//!   { entry = "blur", scale = 0.5, data = [0.0, 1.0, 0.5, 0.0], name = "l1" },
//!   { entry = "blur", scale = 0.25, data = [1.0, 0.0, 0.5, 0.0] },
//!   { entry = "blur", scale = 0.25, data = [0.0, 1.0, 0.5, 0.0], name = "l2" },
//!   { entry = "blur", scale = 0.125, data = [1.0, 0.0, 0.6, 0.0] },
//!   { entry = "blur", scale = 0.125, data = [0.0, 1.0, 0.6, 0.0], name = "l3" },
//!   { entry = "sum3", scale = 0.5, inputs = ["l3", "l2", "l1"] },
//!   { entry = "combine", inputs = ["prev", "input"] },
//! ]
fn bright(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let l = luma(c.rgb);
    let k = smoothstep(p.threshold - 0.25, p.threshold + 0.25, l);
    let sat = mix(vec3f(l), c.rgb, p.saturation);
    return vec4f(max(sat, vec3f(0.0)) * k, c.a * k);
}
fn blur(uv: vec2f, p: Params) -> vec4f {
    return blur_dir(uv, U.g.pass_data.xy, p.radius * U.g.pass_data.z);
}
fn sum3(uv: vec2f, p: Params) -> vec4f {
    let a = textureSampleLevel(t0, samp, uv, 0.0);
    let b = textureSampleLevel(t1, samp, uv, 0.0);
    let c = textureSampleLevel(t2, samp, uv, 0.0);
    return c + b * (0.8 * p.falloff) + a * (0.65 * p.falloff * p.falloff);
}
fn combine(uv: vec2f, p: Params) -> vec4f {
    let g = src_clamp(uv) * p.intensity * vec4f(p.tint.rgb, 1.0);
    let o = orig(uv);
    return vec4f(o.rgb + g.rgb, max(o.a, min(g.a, 1.0)));
}

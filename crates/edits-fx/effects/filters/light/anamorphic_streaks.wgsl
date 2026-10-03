//! id = "anamorphic_streaks"
//! name = "Anamorphic Streaks"
//! kind = "filter"
//! category = "light"
//! description = "Long horizontal lens streaks from highlights (sci-fi / cinematic flare)."
//! tags = ["flare", "streak", "anamorphic", "light", "cinematic"]
//! params = [
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 8.0 },
//!   { name = "length", type = "float", default = 300.0, min = 10.0, max = 1500.0, desc = "Streak length in pixels" },
//!   { name = "threshold", type = "float", default = 0.8, min = 0.0, max = 2.0 },
//!   { name = "color", type = "color", default = "#5aa8ff" },
//!   { name = "angle", type = "angle", default = 0.0 },
//! ]
//! passes = [
//!   { entry = "bright", scale = 0.25 },
//!   { entry = "streak", scale = 0.25, data = [0.25, 0.0, 0.0, 0.0] },
//!   { entry = "streak", scale = 0.25, data = [0.08, 0.0, 0.0, 0.0] },
//!   { entry = "combine", inputs = ["prev", "input"] },
//! ]
fn bright(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    return c * smoothstep(p.threshold, p.threshold + 0.3, luma(c.rgb));
}
fn streak(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle);
    let len = p.length * U.g.pass_data.x;
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var i = -24; i <= 24; i++) {
        let x = f32(i) / 24.0;
        let w = exp(-abs(x) * 3.0);
        acc += src_clamp(uv + d * x * len * texel()) * w;
        wsum += w;
    }
    return acc / wsum * 1.5;
}
fn combine(uv: vec2f, p: Params) -> vec4f {
    let s = src_clamp(uv).rgb * p.intensity * p.color.rgb * 3.0;
    let o = orig(uv);
    return vec4f(o.rgb + s, max(o.a, clamp(luma(s), 0.0, 1.0)));
}

//! id = "light_burst"
//! name = "Light Burst"
//! kind = "filter"
//! category = "light"
//! description = "Radial burst of light from a point (impact frames, power-ups)."
//! tags = ["burst", "impact", "rays", "power", "amv"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 5.0 },
//!   { name = "rays", type = "float", default = 24.0, min = 3.0, max = 120.0 },
//!   { name = "spin", type = "float", default = 0.2, min = -5.0, max = 5.0, desc = "Rotation speed (turns/sec)" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "falloff", type = "float", default = 1.5, min = 0.1, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let a = atan2(d.y, d.x) + time() * p.spin * TAU;
    let r = length(d);
    let n = vnoise(vec2f(a * p.rays / TAU * 3.0, 1.0) + seed());
    let ray = pow(abs(sin(a * p.rays * 0.5)), 8.0) * (0.5 + n);
    let core = exp(-r * 6.0 / p.falloff);
    let c = p.color.rgb * (ray * exp(-r * 2.0 / p.falloff) + core) * p.intensity;
    let o = src(uv);
    return vec4f(o.rgb + c, max(o.a, clamp(luma(c), 0.0, 1.0)));
}

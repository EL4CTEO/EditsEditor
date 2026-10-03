//! id = "light_leak"
//! name = "Light Leak"
//! kind = "filter"
//! category = "light"
//! description = "Animated warm film light leaks screened over the image."
//! tags = ["light leak", "film", "warm", "vintage", "dreamy"]
//! params = [
//!   { name = "intensity", type = "float", default = 0.7, min = 0.0, max = 3.0 },
//!   { name = "speed", type = "float", default = 0.3, min = 0.0, max = 5.0 },
//!   { name = "color_a", type = "color", default = "#ff6a00" },
//!   { name = "color_b", type = "color", default = "#ff2f7a" },
//!   { name = "scale", type = "float", default = 1.0, min = 0.2, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time() * p.speed + seed() * 10.0;
    let q = uv * vec2f(aspect(), 1.0) / p.scale;
    let n1 = fbm(q * 1.2 + vec2f(t * 0.6, t * 0.2), 4) * 0.5 + 0.5;
    let n2 = fbm(q * 0.8 - vec2f(t * 0.3, -t * 0.4) + 5.0, 4) * 0.5 + 0.5;
    let edge = smoothstep(0.35, 1.0, max(1.0 - uv.x * 1.3, uv.y * 0.9 + n2 * 0.3));
    var c = mix(p.color_a.rgb, p.color_b.rgb, n2) * pow(n1, 2.0) * edge * p.intensity * 1.6;
    let o = src(uv);
    return vec4f(screen3(o.rgb, c), max(o.a, clamp(luma(c), 0.0, 1.0)));
}

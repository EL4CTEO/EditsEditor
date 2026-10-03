//! id = "light_leak_transition"
//! name = "Light Leak Transition"
//! kind = "transition"
//! category = "light"
//! description = "Warm colorful light leak washes over the cut."
//! tags = ["light leak", "warm", "film", "dreamy"]
//! params = [
//!   { name = "color_a", type = "color", default = "#ff9a3c" },
//!   { name = "color_b", type = "color", default = "#ff3c8a" },
//!   { name = "intensity", type = "float", default = 1.5, min = 0.0, max = 4.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = sin(t * PI);
    let n = fbm(uv * 2.0 + vec2f(t * 1.5, seed()), 4) * 0.5 + 0.5;
    let base = mix(from_img(uv), to_img(uv), smoothstep(0.3, 0.7, t));
    let leak = mix(p.color_a.rgb, p.color_b.rgb, uv.x) * pow(n, 1.5) * k * p.intensity;
    return vec4f(screen3(base.rgb, leak), max(base.a, k));
}

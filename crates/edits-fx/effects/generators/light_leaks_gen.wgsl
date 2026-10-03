//! id = "light_leaks"
//! name = "Light Leaks Overlay"
//! kind = "generator"
//! category = "light"
//! description = "Animated organic light leak blobs on black — set the clip blend mode to screen/add."
//! tags = ["light leak", "overlay", "film", "warm"]
//! params = [
//!   { name = "speed", type = "float", default = 0.3, min = 0.0, max = 5.0 },
//!   { name = "color_a", type = "color", default = "#ff6a00" },
//!   { name = "color_b", type = "color", default = "#ff2f7a" },
//!   { name = "color_c", type = "color", default = "#ffd000" },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 4.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time() * p.speed + seed() * 10.0;
    let q = uv * vec2f(aspect(), 1.0);
    let n1 = fbm(q * 1.5 + vec2f(t * 0.5, t * 0.2), 4) * 0.5 + 0.5;
    let n2 = fbm(q * 1.0 + vec2f(-t * 0.3, t * 0.4) + 3.0, 4) * 0.5 + 0.5;
    let n3 = fbm(q * 2.0 + vec2f(t * 0.2, -t * 0.6) + 7.0, 3) * 0.5 + 0.5;
    var c = p.color_a.rgb * pow(n1, 4.0) + p.color_b.rgb * pow(n2, 4.0) + p.color_c.rgb * pow(n3, 6.0);
    c *= p.intensity * 3.0;
    return vec4f(c, 1.0);
}

//! id = "sparkles"
//! name = "Sparkles / Glitter"
//! kind = "generator"
//! category = "particles"
//! description = "Twinkling 4-point star sparkles (magical girl, shiny moments)."
//! tags = ["sparkle", "glitter", "stars", "magic", "kawaii", "shine"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 5.0 },
//!   { name = "speed", type = "float", default = 1.0, min = 0.0, max = 10.0, desc = "Twinkle speed" },
//!   { name = "color", type = "color", default = "#fff6c9" },
//!   { name = "rainbow", type = "float", default = 0.0, min = 0.0, max = 1.0 },
//! ]
fn star(f: vec2f, s: f32) -> f32 {
    let a = abs(f);
    let cross = max(smoothstep(s * 0.12, 0.0, a.x) * smoothstep(s, 0.0, a.y), smoothstep(s * 0.12, 0.0, a.y) * smoothstep(s, 0.0, a.x));
    let core = smoothstep(s * 0.35, 0.0, length(f));
    return max(cross, core);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * vec2f(aspect(), 1.0) * 8.0 * p.density;
    let cell = floor(q);
    let f = fract(q) - 0.5;
    let h = hash22(cell + seed());
    let pos = (h - 0.5) * 0.6;
    let tw = pow(max(0.0, sin(time() * p.speed * (2.0 + h.x * 4.0) + h.y * 30.0)), 6.0);
    let v = star(f - pos, 0.35 * p.size * (0.4 + h.x)) * tw * step(0.55, h.y);
    let col = mix(p.color.rgb, to_linear(hsv2rgb(vec3f(h.x, 0.6, 1.0))), p.rainbow);
    return vec4f(col * v, clamp(v, 0.0, 1.0));
}

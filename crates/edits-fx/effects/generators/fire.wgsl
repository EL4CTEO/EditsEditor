//! id = "fire"
//! name = "Fire"
//! kind = "generator"
//! category = "energy"
//! description = "Procedural flames rising from the bottom (transparent background)."
//! tags = ["fire", "flames", "burn", "heat", "energy"]
//! params = [
//!   { name = "height", type = "float", default = 0.6, min = 0.05, max = 1.5 },
//!   { name = "speed", type = "float", default = 1.0, min = 0.0, max = 5.0 },
//!   { name = "scale", type = "float", default = 3.0, min = 0.5, max = 20.0 },
//!   { name = "tint", type = "color", default = "#ff6a00" },
//!   { name = "intensity", type = "float", default = 1.5, min = 0.0, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = vec2f(uv.x * aspect(), uv.y) * p.scale;
    let t = time() * p.speed;
    let n = fbm(q + vec2f(0.0, t * 2.0), 5) * 0.5 + 0.5;
    let n2 = fbm(q * 2.0 + vec2f(0.0, t * 3.0), 4) * 0.5 + 0.5;
    let y = 1.0 - uv.y;
    let shape = clamp((p.height - y) / p.height + (n - 0.5) * 0.8 + (n2 - 0.5) * 0.4, 0.0, 1.0);
    let heat = pow(shape, 1.5);
    let c = mix(p.tint.rgb * 0.6, vec3f(1.0, 0.95, 0.6), heat * heat) * heat * p.intensity;
    return vec4f(c, clamp(heat * 1.5, 0.0, 1.0));
}

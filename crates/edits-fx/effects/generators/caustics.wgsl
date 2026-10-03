//! id = "caustics"
//! name = "Water Caustics"
//! kind = "generator"
//! category = "texture"
//! description = "Shimmering underwater light caustics (screen over footage for underwater looks)."
//! tags = ["water", "caustics", "underwater", "pool", "shimmer"]
//! params = [
//!   { name = "scale", type = "float", default = 4.0, min = 0.5, max = 30.0 },
//!   { name = "speed", type = "float", default = 0.5, min = 0.0, max = 5.0 },
//!   { name = "color", type = "color", default = "#9fe8ff" },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * vec2f(aspect(), 1.0) * p.scale;
    let t = time() * p.speed;
    let w1 = worley(q + vec2f(t * 0.3, t * 0.2));
    let w2 = worley(q * 1.3 - vec2f(t * 0.2, -t * 0.25) + 3.0);
    let v = pow(1.0 - (w1.y - w1.x), 6.0) + pow(1.0 - (w2.y - w2.x), 6.0) * 0.6;
    let c = p.color.rgb * v * p.intensity;
    return vec4f(c, clamp(v, 0.0, 1.0));
}

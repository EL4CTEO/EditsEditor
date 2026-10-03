//! id = "smoke"
//! name = "Smoke / Fog"
//! kind = "generator"
//! category = "weather"
//! description = "Slow drifting smoke or ground fog (transparent background)."
//! tags = ["smoke", "fog", "mist", "atmosphere"]
//! params = [
//!   { name = "density", type = "float", default = 0.6, min = 0.0, max = 2.0 },
//!   { name = "speed", type = "float", default = 0.1, min = 0.0, max = 2.0 },
//!   { name = "scale", type = "float", default = 2.0, min = 0.2, max = 20.0 },
//!   { name = "color", type = "color", default = "#c8ccd6" },
//!   { name = "ground", type = "float", default = 0.0, min = 0.0, max = 1.0, desc = "Concentrate fog near the bottom" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * vec2f(aspect(), 1.0) * p.scale;
    let t = time() * p.speed;
    let w = vec2f(fbm(q + vec2f(t, 0.0), 4), fbm(q + vec2f(0.0, t) + 4.0, 4));
    var v = fbm(q + w + vec2f(t * 0.5, 0.0), 5) * 0.5 + 0.5;
    v = smoothstep(0.35, 0.9, v) * p.density;
    v *= mix(1.0, smoothstep(0.3, 1.0, uv.y), p.ground);
    v = clamp(v, 0.0, 1.0);
    return vec4f(p.color.rgb * v, v);
}

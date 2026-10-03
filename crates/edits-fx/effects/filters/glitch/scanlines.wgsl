//! id = "scanlines"
//! name = "Scanlines"
//! kind = "filter"
//! category = "glitch"
//! description = "Horizontal scanline overlay, optionally rolling."
//! tags = ["scanlines", "retro", "tv", "lines"]
//! params = [
//!   { name = "amount", type = "float", default = 0.35, min = 0.0, max = 1.0 },
//!   { name = "spacing", type = "float", default = 4.0, min = 1.0, max = 64.0, desc = "Pixels between lines" },
//!   { name = "speed", type = "float", default = 0.0, min = -200.0, max = 200.0, desc = "Roll speed (pixels/sec)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let y = uv.y * res().y + time() * p.speed;
    let s = 0.5 + 0.5 * sin(y / p.spacing * TAU);
    return vec4f(c.rgb * (1.0 - p.amount * s), c.a);
}

//! id = "white_balance"
//! name = "White Balance"
//! kind = "filter"
//! category = "color"
//! description = "Temperature (blue <-> orange) and tint (green <-> magenta)."
//! tags = ["color", "temperature", "warm", "cool", "tint"]
//! params = [
//!   { name = "temperature", type = "float", default = 0.0, min = -1.0, max = 1.0 },
//!   { name = "tint", type = "float", default = 0.0, min = -1.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = unpremul(src_clamp(uv));
    let m = vec3f(1.0 + p.temperature * 0.3 + p.tint * 0.1, 1.0 - p.tint * 0.2, 1.0 - p.temperature * 0.3 + p.tint * 0.1);
    return premul(vec4f(c.rgb * m, c.a));
}

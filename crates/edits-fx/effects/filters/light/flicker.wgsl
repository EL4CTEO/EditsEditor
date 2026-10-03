//! id = "flicker"
//! name = "Flicker"
//! kind = "filter"
//! category = "light"
//! description = "Random brightness flicker (old film, unstable light, horror)."
//! tags = ["flicker", "film", "unstable"]
//! params = [
//!   { name = "amount", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "speed", type = "float", default = 18.0, min = 1.0, max = 60.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = hash11(floor(time() * p.speed) + seed() * 91.0);
    let k = 1.0 + (n - 0.5) * 2.0 * p.amount;
    let c = src_clamp(uv);
    return vec4f(c.rgb * k, c.a);
}
